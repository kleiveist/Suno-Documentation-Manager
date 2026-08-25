use crate::error::{AppError, Result};
use crate::model::{
    AudioScreeningState, DocumentPreview, EvidenceItem, EvidenceRole, OperationProgress, Profile,
    StepState, TrackRecord,
};
use crate::security::{atomic_write, contained_path, portable_relative, sha256_bytes};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

mod audio_screening;
mod evidence;
mod presentation;
mod rendering;
#[cfg(test)]
mod tests;

use rendering::render;

pub const TEMPLATE_VERSION: &str = "1.11";
pub const MANAGED_MARKER: &str = "suno-documentation-manager:template-v1";
pub const ARTWORK_IMPORT_TIMESTAMP_NOTICE: &str = "Import timestamps document only the import into SunoDM and do not establish the actual creation or editing sequence of the artwork files.";
const MARKDOWN_MARKER_HEADER: &str = "<!-- suno-documentation-manager:template-v1 -->\n";
const TEXT_MARKER_HEADER: &str = "# suno-documentation-manager:template-v1\n";
pub const DOCUMENT_PATHS: [&str; 8] = [
    "02_SUNO/suno_project.txt",
    "02_SUNO/Lyrics.md",
    "02_SUNO/Style.md",
    "03_DOCUMENTATION/README.md",
    "03_DOCUMENTATION/AI_USAGE.md",
    "04_LICENSES/suno_account_and_license.md",
    "04_LICENSES/openai_image_generation.md",
    "05_ARTWORK/artwork_process.md",
];
const LEGACY_MANAGED_DOCUMENT_PATHS: [&str; 2] =
    ["03_DOCUMENTATION/Lyrics.md", "03_DOCUMENTATION/Styles.md"];

#[derive(Serialize)]
struct Fingerprint<'a> {
    template_version: &'static str,
    workflow_id: &'a str,
    workflow_version: &'a str,
    profile: &'a Profile,
    fields: &'a crate::model::TrackFields,
    /// Includes the durable state (including the local fingerprint) only in
    /// the internal freshness digest. It is never rendered into a managed
    /// Markdown/text document by this module.
    audio_screening: &'a AudioScreeningState,
    evidence: Vec<(
        &'a str,
        &'a str,
        Option<&'a str>,
        &'a crate::model::EvidenceMetadata,
    )>,
}

pub fn input_fingerprint(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
) -> Result<String> {
    let normalized_fields = track.fields.normalized_conditionals();
    let mut sorted_evidence = evidence
        .iter()
        .filter(|item| item.role != EvidenceRole::ExternalTimestamp)
        .collect::<Vec<_>>();
    sorted_evidence.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let evidence_values = sorted_evidence
        .into_iter()
        .map(|item| {
            (
                item.role.as_str(),
                item.relative_path.as_str(),
                item.sha256.as_deref(),
                &item.metadata,
            )
        })
        .collect::<Vec<_>>();
    let value = serde_json::to_vec(&Fingerprint {
        template_version: TEMPLATE_VERSION,
        workflow_id: &track.workflow_id,
        workflow_version: &track.workflow_version,
        profile,
        fields: &normalized_fields,
        audio_screening: &track.audio_screening,
        evidence: evidence_values,
    })?;
    Ok(sha256_bytes(&value))
}

pub fn is_current(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    steps: &[StepState],
) -> Result<bool> {
    let expected = render(track, profile, evidence, steps);
    let files_match = DOCUMENT_PATHS.iter().all(|relative| {
        let Ok(path) = contained_path(track_root, Path::new(relative), true) else {
            return false;
        };
        let Some(expected_content) = expected.get(*relative) else {
            return false;
        };
        fs::read(path)
            .map(|content| content == expected_content.as_bytes())
            .unwrap_or(false)
    });
    Ok(track.documents.generated
        && track.documents.template_version == TEMPLATE_VERSION
        && track.documents.input_fingerprint == input_fingerprint(track, profile, evidence)?
        && files_match)
}

pub fn preview(track_root: &Path) -> Result<DocumentPreview> {
    let mut collisions = Vec::new();
    for relative in DOCUMENT_PATHS {
        let path = contained_path(track_root, Path::new(relative), false)?;
        if path.exists() {
            let managed = has_exact_managed_header(&path, relative)?;
            if !managed {
                collisions.push(relative.to_owned());
            }
        }
    }
    Ok(DocumentPreview {
        files: DOCUMENT_PATHS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        adoption_required: !collisions.is_empty(),
        collisions,
    })
}

#[cfg(test)]
pub fn generate(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    steps: &[StepState],
    adopt_existing: bool,
) -> Result<Vec<String>> {
    generate_with_progress(
        track_root,
        track,
        profile,
        evidence,
        steps,
        adopt_existing,
        &mut |_| {},
    )
}

pub fn generate_with_progress(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    steps: &[StepState],
    adopt_existing: bool,
    on_progress: &mut impl FnMut(OperationProgress),
) -> Result<Vec<String>> {
    let total_files = DOCUMENT_PATHS.len() as u32;
    on_progress(document_progress(
        "preparing_documents",
        0,
        total_files,
        None,
    ));
    let preview = preview(track_root)?;
    if preview.adoption_required && !adopt_existing {
        return Err(AppError::AdoptionRequired(preview.collisions.join(", ")));
    }
    if preview.adoption_required {
        archive_existing(track_root, &preview.collisions)?;
    }

    on_progress(document_progress(
        "rendering_documents",
        0,
        total_files,
        None,
    ));
    let generated = render(track, profile, evidence, steps);
    for (index, (relative, content)) in generated.iter().enumerate() {
        on_progress(document_progress(
            "writing_documents",
            index as u32,
            total_files,
            Some(relative.clone()),
        ));
        let target = contained_path(track_root, Path::new(relative), false)?;
        atomic_write(&target, content.as_bytes())?;
        on_progress(document_progress(
            "writing_documents",
            index as u32 + 1,
            total_files,
            Some(relative.clone()),
        ));
    }
    on_progress(document_progress(
        "finalizing_documents",
        total_files,
        total_files,
        None,
    ));
    for relative in LEGACY_MANAGED_DOCUMENT_PATHS {
        let legacy = contained_path(track_root, Path::new(relative), false)?;
        if legacy.is_file() && has_exact_managed_header(&legacy, relative)? {
            fs::remove_file(&legacy).map_err(|error| AppError::io(&legacy, error))?;
        }
    }
    Ok(generated.keys().cloned().collect())
}

fn document_progress(
    stage: &str,
    processed_files: u32,
    total_files: u32,
    current_file: Option<String>,
) -> OperationProgress {
    OperationProgress {
        stage: stage.to_owned(),
        processed_files,
        total_files,
        current_file,
        ..OperationProgress::default()
    }
}

fn archive_existing(track_root: &Path, collisions: &[String]) -> Result<()> {
    let archive_relative = PathBuf::from(".archive")
        .join("adoptions")
        .join(Uuid::new_v4().to_string());
    for relative in collisions {
        let source = contained_path(track_root, Path::new(relative), true)?;
        let destination_relative = archive_relative.join(relative);
        let destination = contained_path(track_root, &destination_relative, false)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }
        fs::copy(&source, &destination).map_err(|e| AppError::io(&destination, e))?;
        if crate::security::sha256_file(&source)? != crate::security::sha256_file(&destination)? {
            return Err(AppError::Validation(format!(
                "Archived backup could not be verified: {}",
                portable_relative(Path::new(relative))
            )));
        }
    }
    Ok(())
}

fn marker() -> &'static str {
    MARKDOWN_MARKER_HEADER
}

fn has_exact_managed_header(path: &Path, relative: &str) -> Result<bool> {
    let expected = if relative.ends_with(".md") {
        MARKDOWN_MARKER_HEADER.as_bytes()
    } else {
        TEXT_MARKER_HEADER.as_bytes()
    };
    let mut file = fs::File::open(path).map_err(|error| AppError::io(path, error))?;
    let mut actual = vec![0_u8; expected.len()];
    match file.read_exact(&mut actual) {
        Ok(()) => Ok(actual == expected),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(AppError::io(path, error)),
    }
}
