use crate::error::{AppError, Result};
use crate::model::{DocumentPreview, EvidenceItem, Profile, StepState, TrackRecord};
use crate::security::{atomic_write, contained_path, portable_relative, sha256_bytes};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const TEMPLATE_VERSION: &str = "1.0";
pub const MANAGED_MARKER: &str = "suno-documentation-manager:template-v1";
const MARKDOWN_MARKER_HEADER: &str = "<!-- suno-documentation-manager:template-v1 -->\n";
const TEXT_MARKER_HEADER: &str = "# suno-documentation-manager:template-v1\n";
pub const DOCUMENT_PATHS: [&str; 8] = [
    "02_SUNO/suno_project.txt",
    "03_DOCUMENTATION/README.md",
    "03_DOCUMENTATION/AI_USAGE.md",
    "03_DOCUMENTATION/Lyrics.md",
    "03_DOCUMENTATION/Styles.md",
    "04_LICENSES/suno_account_and_license.md",
    "04_LICENSES/openai_image_generation.md",
    "05_ARTWORK/artwork_process.md",
];

#[derive(Serialize)]
struct Fingerprint<'a> {
    template_version: &'static str,
    workflow_id: &'a str,
    workflow_version: &'a str,
    profile: &'a Profile,
    fields: &'a crate::model::TrackFields,
    evidence: Vec<(&'a str, &'a str, Option<&'a str>)>,
}

pub fn input_fingerprint(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
) -> Result<String> {
    let normalized_fields = track.fields.normalized_conditionals();
    let mut evidence_values = evidence
        .iter()
        .map(|item| {
            (
                item.role.as_str(),
                item.relative_path.as_str(),
                item.sha256.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    evidence_values.sort_unstable();
    let value = serde_json::to_vec(&Fingerprint {
        template_version: TEMPLATE_VERSION,
        workflow_id: &track.workflow_id,
        workflow_version: &track.workflow_version,
        profile,
        fields: &normalized_fields,
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

pub fn generate(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    steps: &[StepState],
    adopt_existing: bool,
) -> Result<Vec<String>> {
    let preview = preview(track_root)?;
    if preview.adoption_required && !adopt_existing {
        return Err(AppError::AdoptionRequired(preview.collisions.join(", ")));
    }
    if preview.adoption_required {
        archive_existing(track_root, &preview.collisions)?;
    }

    let generated = render(track, profile, evidence, steps);
    for (relative, content) in &generated {
        let target = contained_path(track_root, Path::new(relative), false)?;
        atomic_write(&target, content.as_bytes())?;
    }
    Ok(generated.keys().cloned().collect())
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

fn yes_no(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Not documented",
    }
}

fn value_or_missing(value: &str) -> &str {
    if value.trim().is_empty() {
        "Not documented"
    } else {
        value
    }
}

fn evidence_list(evidence: &[EvidenceItem]) -> String {
    if evidence.is_empty() {
        return "- No evidence files registered\n".into();
    }
    let mut values = evidence.to_vec();
    values.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    values
        .iter()
        .map(|item| {
            format!(
                "- `{}` — role `{}` — SHA-256 `{}`\n",
                item.relative_path,
                item.role.as_str(),
                item.sha256.as_deref().unwrap_or("not calculated")
            )
        })
        .collect()
}

fn render(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    _steps: &[StepState],
) -> BTreeMap<String, String> {
    // Rendering is defensive as well as patch normalization: older workspace records may still
    // contain hidden answers from a formerly active conditional branch.
    let normalized_fields = track.fields.normalized_conditionals();
    let f = &normalized_fields;
    let artwork_present = !matches!(f.artwork_origin.as_str(), "" | "none");
    let ai_artwork = matches!(f.artwork_origin.as_str(), "ai_generated" | "ai_assisted");
    let ai_original = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::AiArtworkOriginal)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("Not documented");
    let final_artwork = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::FinalArtwork)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("Not documented");
    let mut lyrics_document = format!(
        "{}# Lyrics\n\nSource: {}\n",
        marker(),
        value_or_missing(&f.lyrics_source)
    );
    if matches!(f.lyrics_source.as_str(), "human" | "mixed") {
        lyrics_document.push_str(&format!(
            "\n## Text\n\n{}\n",
            value_or_missing(&f.lyrics_text)
        ));
    }

    let mut styles_document = format!(
        "{}# Styles and editing\n\n- Human editing performed: {}\n",
        marker(),
        yes_no(f.human_editing_performed)
    );
    if f.human_editing_performed == Some(true) {
        styles_document.push_str(&format!(
            "- Confirmed human work: {}\n",
            value_or_missing(&f.human_editing_details)
        ));
    }
    styles_document.push_str(&format!(
        "- Post-export editing performed: {}\n",
        yes_no(f.post_export_editing_performed)
    ));
    if f.post_export_editing_performed == Some(true) {
        styles_document.push_str(&format!(
            "- Confirmed post-export work: {}\n",
            value_or_missing(&f.post_export_editing_details)
        ));
    }
    styles_document.push_str(&format!(
        "- Release notes: {}\n",
        value_or_missing(&f.release_notes)
    ));

    let mut ai_artwork_usage = format!("- Origin: {}\n", value_or_missing(&f.artwork_origin));
    if artwork_present {
        if ai_artwork {
            ai_artwork_usage.push_str(&format!(
                "- AI service: {}\n- AI-generated base image: {}\n",
                value_or_missing(&f.ai_image_service),
                ai_original
            ));
            if f.artwork_origin == "ai_assisted" {
                ai_artwork_usage.push_str(&format!(
                    "- Human modifications: {}\n",
                    value_or_missing(&f.human_artwork_modifications)
                ));
            }
            ai_artwork_usage.push_str(&format!(
                "- Project transparency policy: {}\n- Visible disclosure applied: {}\n",
                profile.artwork_transparency_policy,
                yes_no(f.disclosure_applied)
            ));
            if f.disclosure_applied == Some(true) {
                ai_artwork_usage.push_str(&format!(
                    "- Disclosure text: {}\n",
                    value_or_missing(&f.disclosure_text)
                ));
            }
        }
        ai_artwork_usage.push_str(&format!("- Final output: {}\n", final_artwork));
    }

    let image_generation_document = if ai_artwork {
        let mut content = format!(
            "{}# AI image generation record\n\n- AI image service: {}\n- Artwork origin: {}\n- Project transparency policy: {}\n- Disclosure applied: {}\n",
            marker(),
            value_or_missing(&f.ai_image_service),
            value_or_missing(&f.artwork_origin),
            profile.artwork_transparency_policy,
            yes_no(f.disclosure_applied)
        );
        if f.disclosure_applied == Some(true) {
            content.push_str(&format!(
                "- Disclosure text: {}\n",
                value_or_missing(&f.disclosure_text)
            ));
        }
        content.push_str(
            "\nThe service name is a user-supplied fact; this document does not assert license rights.\n",
        );
        content
    } else {
        format!(
            "{}# AI image generation record\n\n- Artwork origin: {}\n\nNo AI image generation record applies to the documented artwork origin.\n",
            marker(),
            value_or_missing(&f.artwork_origin)
        )
    };

    let mut artwork_document = format!(
        "{}# Artwork process\n\n- Origin: {}\n",
        marker(),
        value_or_missing(&f.artwork_origin)
    );
    if artwork_present {
        if ai_artwork {
            artwork_document.push_str(&format!(
                "- AI service: {}\n- AI-generated base image: {}\n",
                value_or_missing(&f.ai_image_service),
                ai_original
            ));
            if f.artwork_origin == "ai_assisted" {
                artwork_document.push_str(&format!(
                    "- Human modifications: {}\n",
                    value_or_missing(&f.human_artwork_modifications)
                ));
            }
            artwork_document.push_str(&format!(
                "- Disclosure policy: {}\n- Disclosure applied: {}\n",
                profile.artwork_transparency_policy,
                yes_no(f.disclosure_applied)
            ));
            if f.disclosure_applied == Some(true) {
                artwork_document.push_str(&format!(
                    "- Disclosure text: {}\n",
                    value_or_missing(&f.disclosure_text)
                ));
            }
        }
        artwork_document.push_str(&format!(
            "- Final output: {}\n- Depicts a real person: {}\n",
            final_artwork,
            yes_no(f.depicts_real_person)
        ));
        if f.depicts_real_person == Some(true) {
            artwork_document.push_str(&format!(
                "- Real-person note: {}\n",
                value_or_missing(&f.real_person_notes)
            ));
        }
        artwork_document.push_str(&format!(
            "- Represents a real event as authentic: {}\n",
            yes_no(f.depicts_real_event)
        ));
        if f.depicts_real_event == Some(true) {
            artwork_document.push_str(&format!(
                "- Real-event note: {}\n",
                value_or_missing(&f.real_event_notes)
            ));
        }
        artwork_document.push_str(&format!(
            "- Contains a trademark or logo: {}\n",
            yes_no(f.contains_trademark)
        ));
        if f.contains_trademark == Some(true) {
            artwork_document.push_str(&format!(
                "- Trademark/logo note: {}\n",
                value_or_missing(&f.trademark_notes)
            ));
        }
        let artwork_evidence = evidence
            .iter()
            .filter(|item| {
                item.relative_path.starts_with("05_ARTWORK/")
                    && (ai_artwork
                        || !matches!(
                            item.role,
                            crate::model::EvidenceRole::AiArtworkOriginal
                                | crate::model::EvidenceRole::AiArtworkEdited
                        ))
            })
            .cloned()
            .collect::<Vec<_>>();
        artwork_document.push_str(&format!(
            "\n## Artwork evidence\n\n{}",
            evidence_list(&artwork_evidence)
        ));
    } else {
        artwork_document.push_str("\nNo artwork process applies to this track.\n");
    }

    let mut values = BTreeMap::new();
    values.insert(
        "02_SUNO/suno_project.txt".into(),
        format!(
            "# {MANAGED_MARKER}\nTemplate version: {TEMPLATE_VERSION}\nTrack: {}\nSuno project URL: {}\nSuno model: {}\nSuno plan at creation: {}\nProduction start: {}\nProduction end: {}\nFinal export date: {}\nExternal audio uploaded: {}\nOwn audio uploaded: {}\nThird-party samples uploaded: {}\n",
            f.title,
            value_or_missing(&f.suno_project_url),
            value_or_missing(&f.suno_model),
            value_or_missing(&f.suno_plan_at_creation),
            value_or_missing(&f.production_start_date),
            value_or_missing(&f.production_end_date),
            value_or_missing(&f.final_export_date),
            yes_no(f.external_audio_uploaded),
            yes_no(f.own_audio_uploaded),
            yes_no(f.third_party_samples_uploaded),
        ),
    );
    values.insert(
        "03_DOCUMENTATION/README.md".into(),
        format!(
            "{}# Track documentation: {}\n\nTemplate version: `{}`  \nWorkflow: `{}` version `{}`\n\n## Snapshot\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Suno plan at creation: {}\n- Commercial use intended: {}\n- Production period: {} to {}\n- Final export date: {}\n\n## Workflow status\n\n{}\n## Evidence\n\n{}",
            marker(), f.title, TEMPLATE_VERSION, track.workflow_id, track.workflow_version,
            value_or_missing(&profile.artist_name), value_or_missing(&profile.suno_profile_name),
            value_or_missing(&profile.suno_handle), value_or_missing(&f.suno_plan_at_creation),
            if f.commercial_use_intended { "Yes" } else { "No" },
            value_or_missing(&f.production_start_date), value_or_missing(&f.production_end_date),
            value_or_missing(&f.final_export_date),
            "- The authoritative evaluated step results are stored in the completion certificate after finalization.\n",
            evidence_list(evidence)
        ),
    );
    values.insert(
        "03_DOCUMENTATION/AI_USAGE.md".into(),
        format!(
            "{}# AI usage\n\n## Music generation\n\n- Suno model: {}\n- Suno project: {}\n- Lyrics source: {}\n- External audio uploaded: {}\n\n## Artwork\n\n{}",
            marker(), value_or_missing(&f.suno_model), value_or_missing(&f.suno_project_url),
            value_or_missing(&f.lyrics_source), yes_no(f.external_audio_uploaded),
            ai_artwork_usage
        ),
    );
    values.insert("03_DOCUMENTATION/Lyrics.md".into(), lyrics_document);
    values.insert("03_DOCUMENTATION/Styles.md".into(), styles_document);
    values.insert(
        "04_LICENSES/suno_account_and_license.md".into(),
        format!(
            "{}# Suno account and plan snapshot\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Plan at creation: {}\n- Workspace subscription start date: {}\n- Commercial use intended: {}\n\nThis page records the supplied account and plan facts. It makes no legal determination.\n",
            marker(), value_or_missing(&profile.artist_name), value_or_missing(&profile.suno_profile_name),
            value_or_missing(&profile.suno_handle), value_or_missing(&f.suno_plan_at_creation),
            value_or_missing(&profile.subscription_start_date), if f.commercial_use_intended { "Yes" } else { "No" }
        ),
    );
    values.insert(
        "04_LICENSES/openai_image_generation.md".into(),
        image_generation_document,
    );
    values.insert("05_ARTWORK/artwork_process.md".into(), artwork_document);
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_with_fields(fields: crate::model::TrackFields) -> TrackRecord {
        TrackRecord {
            id: "conditional-render-test".into(),
            relative_path: "conditional-render-test".into(),
            status: crate::model::TrackStatus::Active,
            workflow_id: "suno-track-documentation".into(),
            workflow_version: "1.0".into(),
            profile_snapshot: Profile::default(),
            fields,
            documents: crate::model::DocumentState::default(),
            integrity: crate::model::IntegrityState::default(),
            certificate: crate::model::CertificateState::default(),
            created_at: "2026-08-13T00:00:00Z".into(),
            updated_at: "2026-08-13T00:00:00Z".into(),
            legacy: false,
        }
    }

    #[test]
    fn preview_requires_the_marker_at_the_exact_header() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let path = workspace.path().join("03_DOCUMENTATION/README.md");
        fs::create_dir_all(path.parent().expect("document parent"))
            .expect("create document parent");
        fs::write(
            &path,
            format!("# Legacy document\n\nThis mentions {MANAGED_MARKER}, but is not managed.\n"),
        )
        .expect("write legacy document");

        let result = preview(workspace.path()).expect("preview documents");
        assert!(result.adoption_required);
        assert_eq!(result.collisions, vec!["03_DOCUMENTATION/README.md"]);

        fs::write(&path, format!("{MARKDOWN_MARKER_HEADER}# Managed\n"))
            .expect("write managed document");
        let result = preview(workspace.path()).expect("preview managed documents");
        assert!(!result.adoption_required);
        assert!(result.collisions.is_empty());
    }

    #[test]
    fn rendering_and_fingerprints_ignore_inactive_conditional_values() {
        let stale_values = [
            "STALE-LYRICS",
            "STALE-EXTERNAL-SOURCE",
            "STALE-EXTERNAL-RIGHTS",
            "STALE-OWN-SOURCE",
            "STALE-OWN-RIGHTS",
            "STALE-SAMPLE-SOURCE",
            "STALE-SAMPLE-RIGHTS",
            "STALE-HUMAN-EDIT",
            "STALE-POST-EDIT",
            "STALE-AI-SERVICE",
            "STALE-ARTWORK-EDIT",
            "STALE-PERSON-NOTE",
            "STALE-EVENT-NOTE",
            "STALE-TRADEMARK-NOTE",
            "STALE-DISCLOSURE",
        ];
        let fields = crate::model::TrackFields {
            title: "Conditional Track".into(),
            lyrics_source: "instrumental".into(),
            lyrics_text: stale_values[0].into(),
            external_audio_uploaded: Some(false),
            external_audio_source: stale_values[1].into(),
            external_audio_ownership: stale_values[2].into(),
            own_audio_uploaded: Some(false),
            own_audio_source: stale_values[3].into(),
            own_audio_ownership: stale_values[4].into(),
            third_party_samples_uploaded: Some(false),
            third_party_sample_source: stale_values[5].into(),
            third_party_sample_ownership: stale_values[6].into(),
            human_editing_performed: Some(false),
            human_editing_details: stale_values[7].into(),
            post_export_editing_performed: Some(false),
            post_export_editing_details: stale_values[8].into(),
            artwork_origin: "none".into(),
            ai_image_service: stale_values[9].into(),
            human_artwork_modifications: stale_values[10].into(),
            depicts_real_person: Some(false),
            real_person_notes: stale_values[11].into(),
            depicts_real_event: Some(false),
            real_event_notes: stale_values[12].into(),
            contains_trademark: Some(false),
            trademark_notes: stale_values[13].into(),
            disclosure_applied: Some(true),
            disclosure_text: stale_values[14].into(),
            ..crate::model::TrackFields::default()
        };
        let stale = record_with_fields(fields);
        let rendered = render(&stale, &Profile::default(), &[], &[]);
        let combined = rendered.values().cloned().collect::<String>();

        for stale_value in stale_values {
            assert!(
                !combined.contains(stale_value),
                "inactive value was rendered: {stale_value}"
            );
        }
        assert!(!rendered["03_DOCUMENTATION/Lyrics.md"].contains("## Text"));
        assert!(!rendered["03_DOCUMENTATION/Styles.md"].contains("Confirmed human work"));
        assert!(!rendered["03_DOCUMENTATION/Styles.md"].contains("Confirmed post-export work"));
        assert!(!rendered["03_DOCUMENTATION/AI_USAGE.md"].contains("AI service:"));
        assert!(!rendered["05_ARTWORK/artwork_process.md"].contains("Real-person note:"));

        let clean = record_with_fields(stale.fields.normalized_conditionals());
        assert_eq!(
            input_fingerprint(&stale, &Profile::default(), &[]).expect("stale fingerprint"),
            input_fingerprint(&clean, &Profile::default(), &[]).expect("clean fingerprint")
        );
    }
}
