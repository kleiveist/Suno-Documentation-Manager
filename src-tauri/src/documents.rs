use crate::error::{AppError, Result};
use crate::model::{
    AudioScreeningState, AudioScreeningStatus, DocumentPreview, DocumentationAnswer, EvidenceItem,
    EvidenceRole, OperationProgress, Profile, StepState, SunoLyricsContentSource,
    SunoLyricsContentType, TrackFields, TrackRecord,
};
use crate::security::{atomic_write, contained_path, portable_relative, sha256_bytes};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const TEMPLATE_VERSION: &str = "1.9";
pub const MANAGED_MARKER: &str = "suno-documentation-manager:template-v1";
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

fn yes_no(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT DOCUMENTED",
    }
}

fn value_or_missing(value: &str) -> &str {
    if value.trim().is_empty() {
        "NOT DOCUMENTED"
    } else {
        value
    }
}

fn documented_list(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        values.join(", ")
    }
}

fn documentation_answer(value: Option<DocumentationAnswer>) -> &'static str {
    match value {
        Some(DocumentationAnswer::Yes) => "YES",
        Some(DocumentationAnswer::No) => "NO",
        Some(DocumentationAnswer::NotDocumented) | None => "NOT DOCUMENTED",
    }
}

fn conditional_documentation_answer(
    controlling: Option<bool>,
    value: Option<DocumentationAnswer>,
) -> &'static str {
    match controlling {
        Some(true) => documentation_answer(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn conditional_value<'a>(controlling: Option<bool>, value: &'a str) -> &'a str {
    match controlling {
        Some(true) => value_or_missing(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

fn suno_content_types(fields: &TrackFields) -> String {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A".into(),
        None => "NOT DOCUMENTED".into(),
        Some(true) if fields.suno_lyrics_content_types.is_empty() => "NOT DOCUMENTED".into(),
        Some(true) => fields
            .suno_lyrics_content_types
            .iter()
            .map(|value| match value {
                SunoLyricsContentType::VocalLyrics => "Vocal lyrics",
                SunoLyricsContentType::StructureInstructions => "Structure instructions",
                SunoLyricsContentType::SoundInstructions => "Sound instructions",
                SunoLyricsContentType::ArrangementInstructions => "Arrangement instructions",
                SunoLyricsContentType::Mixed => "Mixed",
                SunoLyricsContentType::Other => "Other",
            })
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn structure_instructions_present(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
        Some(true) if fields.suno_lyrics_content_types.is_empty() => "NOT DOCUMENTED",
        Some(true) => {
            if fields
                .suno_lyrics_content_types
                .contains(&SunoLyricsContentType::StructureInstructions)
            {
                "YES"
            } else {
                "NO"
            }
        }
    }
}

fn suno_content_source(fields: &TrackFields) -> &'static str {
    match fields.suno_lyrics_field_content {
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
        Some(true) => match fields.suno_lyrics_content_source {
            Some(SunoLyricsContentSource::Human) => "human",
            Some(SunoLyricsContentSource::Ai) => "AI",
            Some(SunoLyricsContentSource::Mixed) => "mixed",
            None => "NOT DOCUMENTED",
        },
    }
}

fn render_suno_field_document(fields: &TrackFields) -> String {
    let heading = if fields.vocal_lyrics_present == Some(true) {
        "Vocal Lyrics"
    } else {
        "Suno Structure / Generation Instructions"
    };
    let mut output = format!(
        "{}# {heading}\n\n- Instrumental track [User-confirmed fact]: {}\n- Vocal lyrics present [User-confirmed fact]: {}\n- Suno lyrics/structure field content [User-confirmed fact]: {}\n- Content types [User-confirmed fact]: {}\n- Structure instructions present [User-confirmed fact]: {}\n- Content source [User-confirmed fact]: {}\n- Other content type [User-confirmed fact]: {}\n\n## Exact Suno field text\n\n{}\n",
        marker(),
        yes_no(fields.instrumental_track),
        yes_no(fields.vocal_lyrics_present),
        yes_no(fields.suno_lyrics_field_content),
        suno_content_types(fields),
        structure_instructions_present(fields),
        suno_content_source(fields),
        if fields.suno_lyrics_field_content == Some(true)
            && fields
                .suno_lyrics_content_types
                .contains(&SunoLyricsContentType::Other)
        {
            value_or_missing(&fields.suno_lyrics_other_content_type)
        } else {
            "N/A"
        },
        match fields.suno_lyrics_field_content {
            Some(true) => value_or_missing(&fields.suno_lyrics_field_text),
            Some(false) => "N/A",
            None => "NOT DOCUMENTED",
        },
    );
    if !fields.lyrics_source.trim().is_empty() || !fields.lyrics_text.trim().is_empty() {
        output
            .push_str("\n## Unclassified legacy lyrics data\n\n- Classification: NOT DOCUMENTED\n");
        output.push_str(&format!(
            "- Legacy source value: {}\n- Legacy text: {}\n\nThis retained historical data is unclassified legacy data and is not a Vocal Lyrics claim.\n",
            value_or_missing(&fields.lyrics_source),
            value_or_missing(&fields.lyrics_text),
        ));
    }
    output
}

fn deepfake_indicator_summary(fields: &TrackFields) -> &'static str {
    if fields.generative_ai_used == Some(false) {
        return "N/A";
    }
    if fields.generative_ai_used != Some(true) {
        return "INCOMPLETE – generative AI use NOT DOCUMENTED";
    }
    let answers = [
        fields.real_person_voice_intentionally_imitated,
        fields.real_person_identity_intentionally_represented,
        fields.real_event_represented_as_authentic_recording,
        fields.real_location_institution_event_presented_as_authentic_ai_recording,
    ];
    if answers
        .iter()
        .any(|answer| *answer == Some(DocumentationAnswer::Yes))
    {
        "Potential deepfake-related indicator recorded"
    } else if answers
        .iter()
        .all(|answer| *answer == Some(DocumentationAnswer::No))
    {
        "Documented deepfake-related indicators: none recorded"
    } else {
        "Deepfake-related indicator documentation: INCOMPLETE"
    }
}

fn render_ai_audio(fields: &TrackFields) -> String {
    let active = fields.generative_ai_used;
    format!(
        "- Generative AI used [User-confirmed fact]: {}\n- AI system [User-confirmed fact]: {}\n- AI-assisted audio elements [User-confirmed fact]: {}\n- AI-generated audio elements [User-confirmed fact]: {}\n- Real person voice intentionally imitated [User-confirmed fact]: {}\n- Real person's identity intentionally represented [User-confirmed fact]: {}\n- Real event represented as authentic recording [User-confirmed fact]: {}\n- Real location / institution / event presented as authentic AI recording [User-confirmed fact]: {}\n- Disclosure applied [User-confirmed fact]: {}\n- Disclosure locations [User-confirmed fact]: {}\n- Disclosure text [User-confirmed fact]: {}\n- Disclosure reason / note [User-confirmed fact]: {}\n- Deepfake-related indicator summary: {}\n",
        yes_no(active),
        conditional_value(active, &fields.audio_ai_system),
        conditional_documentation_answer(active, fields.ai_assisted_audio_elements),
        conditional_documentation_answer(active, fields.ai_generated_audio_elements),
        conditional_documentation_answer(active, fields.real_person_voice_intentionally_imitated,),
        conditional_documentation_answer(
            active,
            fields.real_person_identity_intentionally_represented,
        ),
        conditional_documentation_answer(
            active,
            fields.real_event_represented_as_authentic_recording,
        ),
        conditional_documentation_answer(
            active,
            fields.real_location_institution_event_presented_as_authentic_ai_recording,
        ),
        conditional_documentation_answer(active, fields.audio_disclosure_applied),
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                documented_list(&fields.audio_disclosure_locations)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                value_or_missing(&fields.audio_disclosure_text)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::No)) => {
                value_or_missing(&fields.audio_disclosure_reason)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        deepfake_indicator_summary(fields),
    )
}

fn fact_origin_label(origin: crate::model::FactOrigin) -> &'static str {
    match origin {
        crate::model::FactOrigin::UserConfirmedFact => "User-confirmed fact",
        crate::model::FactOrigin::EvidenceDerivedMetadata => "Evidence-derived metadata",
        crate::model::FactOrigin::NotDocumented => "NOT DOCUMENTED",
    }
}

const LEGACY_SELECTION_NOTICE: &str =
    "Legacy value retained in the track record; select a defined category in the app.";

const SOURCE_CHOICES: &[(&str, &[&str])] = &[
    (
        "Audio from a licensed sample library",
        &["Lizenzierte Sample-Bibliothek"],
    ),
    (
        "Licensed beat or instrumental",
        &["Lizenzierter Beat oder Instrumentaltrack"],
    ),
    (
        "Audio supplied by a collaborator",
        &["Von Mitwirkenden bereitgestelltes Audio"],
    ),
    ("Commissioned recording", &["Beauftragte Aufnahme"]),
    ("Public-domain recording", &["Gemeinfreie Aufnahme"]),
    (
        "Creative Commons recording",
        &["Aufnahme unter Creative-Commons-Lizenz"],
    ),
    ("Original vocal recording", &["Eigene Gesangsaufnahme"]),
    (
        "Original instrument recording",
        &["Eigene Instrumentalaufnahme"],
    ),
    (
        "Original field recording",
        &["Eigene Feldaufnahme", "Eigene Aufnahme"],
    ),
    (
        "Original MIDI or software render",
        &["Eigener MIDI- oder Software-Render"],
    ),
    ("Original sound design", &["Eigenes Sounddesign"]),
    (
        "Commercial sample library",
        &["Kommerzielle Sample-Bibliothek"],
    ),
    ("Royalty-free sample pack", &["Royalty-free Sample-Pack"]),
    (
        "Directly licensed from the sample creator",
        &["Direkt vom Sample-Urheber lizenziert"],
    ),
    ("Public-domain archive", &["Gemeinfreies Archiv"]),
    ("Creative Commons source", &["Creative-Commons-Quelle"]),
];

const RIGHTS_CHOICES: &[(&str, &[&str])] = &[
    (
        "Commercial-use license",
        &["Lizenz für kommerzielle Nutzung"],
    ),
    (
        "Direct permission from the rights holder",
        &["Direkte Erlaubnis des Rechteinhabers"],
    ),
    ("Joint rights agreement", &["Gemeinsame Rechtevereinbarung"]),
    ("Public domain", &["Gemeinfreiheit"]),
    ("Creative Commons license", &["Creative-Commons-Lizenz"]),
    (
        "Solely owned by the artist",
        &["Ausschließlich eigene Rechte", "Eigene Produktion"],
    ),
    (
        "Jointly owned with collaborators",
        &["Gemeinsame Rechte mit Mitwirkenden"],
    ),
    (
        "Participant permissions documented",
        &["Einwilligungen der Beteiligten dokumentiert"],
    ),
    ("Commercial sample license", &["Kommerzielle Sample-Lizenz"]),
    ("Royalty-free license", &["Royalty-free Lizenz"]),
];

const HUMAN_WORK_CHOICES: &[(&str, &[&str])] = &[
    ("Arrangement", &[]),
    ("Lyrics", &[]),
    ("Timing and cuts", &["Timing und Cuts"]),
    ("Sound design", &["Sounddesign"]),
    ("EQ", &[]),
    ("Mixing", &[]),
    ("Mastering", &[]),
    ("Loudness adjustment", &["Lautheitsanpassung"]),
];

const POST_EXPORT_CHOICES: &[(&str, &[&str])] = &[
    ("Editing and cuts", &["Schnitt"]),
    ("Arrangement", &[]),
    ("Timing correction", &["Timing-Korrektur"]),
    ("Sound design", &["Sounddesign"]),
    ("EQ", &[]),
    ("Mixing", &[]),
    ("Mastering", &[]),
    ("Loudness adjustment", &["Lautheitsanpassung"]),
    ("Noise reduction", &["Rauschreduzierung"]),
    ("Dynamics processing", &["Dynamikbearbeitung"]),
];

const RELEASE_CHOICES: &[(&str, &[&str])] = &[
    ("Original Suno version", &["Originale Suno-Fassung"]),
    ("Streaming master", &["Streaming-Master"]),
    ("Radio edit", &["Radio Edit"]),
    ("Extended mix", &["Extended Mix"]),
    ("Instrumental version", &["Instrumental"]),
    ("Clean version", &["Clean Version"]),
    ("Explicit version", &["Explicit Version"]),
    ("Social-media version", &["Social-Media-Version"]),
];

fn choice_key(value: &str) -> String {
    value.trim().to_lowercase()
}

fn english_guided_value(value: &str, choices: &[(&str, &[&str])]) -> String {
    if value.trim().is_empty() {
        return "NOT DOCUMENTED".into();
    }
    let key = choice_key(value);
    choices
        .iter()
        .find(|(english, aliases)| {
            choice_key(english) == key || aliases.iter().any(|alias| choice_key(alias) == key)
        })
        .map(|(english, _)| (*english).to_owned())
        .unwrap_or_else(|| LEGACY_SELECTION_NOTICE.into())
}

fn english_guided_list(value: &str, choices: &[(&str, &[&str])]) -> String {
    if value.trim().is_empty() {
        return "NOT DOCUMENTED".into();
    }
    let mut normalized = Vec::new();
    for item in value
        .split('|')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let mapped = english_guided_value(item, choices);
        if !normalized.contains(&mapped) {
            normalized.push(mapped);
        }
    }
    if normalized.is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        normalized.join(", ")
    }
}

fn evidence_path<'a>(evidence: &'a [EvidenceItem], role: crate::model::EvidenceRole) -> &'a str {
    evidence
        .iter()
        .find(|item| item.role == role)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT DOCUMENTED")
}

fn technically_verified(item: &EvidenceItem) -> bool {
    item.verified
        && item.verification_error.is_none()
        && item.sha256.as_deref().is_some_and(|hash| !hash.is_empty())
}

fn release_identity_status(evidence: &[EvidenceItem]) -> &'static str {
    let release = evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav && technically_verified(item));
    let suno = evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoFinalExport && technically_verified(item));
    match (release, suno) {
        (Some(release), Some(suno)) if release.sha256 == suno.sha256 => "YES",
        (Some(_), Some(_)) => "NO",
        _ => "NOT VERIFIED",
    }
}

fn terms_evidence_status(evidence: &[EvidenceItem]) -> &'static str {
    let terms = evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        "NO"
    } else if terms.iter().any(|item| technically_verified(item)) {
        "YES"
    } else {
        "NOT VERIFIED"
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
                "- Evidence ID `{}` — path `{}` — role `{}` — original `{}` — {} bytes — SHA-256 `{}` — imported `{}` — provenance `{}` — document `{}` — provider `{}` — source URL `{}` — retrieval `{}` — effective `{}` — applicable production period `{}` — note `{}` — source global `{}` — derived from `{}` — generator `{}` — generated disclosure `{}`\n",
                item.id,
                item.relative_path,
                item.role.as_str(),
                value_or_missing(&item.metadata.original_file_name),
                item.size_bytes,
                item.sha256.as_deref().unwrap_or("NOT DOCUMENTED"),
                item.imported_at,
                item.provenance.as_str(),
                value_or_missing(&item.metadata.document_title),
                value_or_missing(&item.metadata.provider),
                value_or_missing(&item.metadata.source_url),
                value_or_missing(&item.metadata.retrieval_date),
                value_or_missing(&item.metadata.effective_date),
                value_or_missing(&item.metadata.applicable_production_period),
                value_or_missing(&item.metadata.factual_note),
                item.source_global_evidence_id.as_deref().unwrap_or("N/A"),
                item.derived_from_evidence_id.as_deref().unwrap_or("N/A"),
                item.generator_version.as_deref().unwrap_or("N/A"),
                item.generated_disclosure_text.as_deref().unwrap_or("N/A"),
            )
        })
        .collect()
}

/// Render the portable, review-friendly screening summary without exposing the
/// full local fingerprint, raw ACRCloud response, request signature, or any
/// credential. The complete fingerprint remains only in the dedicated local
/// JSON artifact, which is integrity-protected with the rest of the phase-one
/// documentation.
fn audio_screening_documentation_summary(state: &AudioScreeningState) -> String {
    let local = &state.local;
    let external = &state.external;
    let matches = if external.matches.is_empty() {
        "NONE RECORDED".to_owned()
    } else {
        external
            .matches
            .iter()
            .take(5)
            .enumerate()
            .map(|(index, item)| {
                let artists = if item.artists.is_empty() {
                    "NOT DOCUMENTED".to_owned()
                } else {
                    item.artists.join(", ")
                };
                let mut value = format!("{} — {}", value_or_missing(&item.title), artists);
                if let Some(album) = item
                    .album
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    value.push_str(&format!("; album {album}"));
                }
                if let Some(isrc) = item
                    .isrc
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    value.push_str(&format!("; ISRC {isrc}"));
                }
                if let Some(acrid) = item
                    .acrid
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    value.push_str(&format!("; ACRID {acrid}"));
                }
                if let Some(score) = item.score {
                    value.push_str(&format!("; score {score}"));
                }
                format!("- Provider match {}: {value}\n", index + 1)
            })
            .collect::<String>()
    };
    let match_block = if matches == "NONE RECORDED" {
        "- Provider matches: NONE RECORDED\n".to_owned()
    } else {
        matches
    };

    format!(
        "## Pre-release audio screening\n\n- Local screening status [System verification]: {}\n- Local engine [System verification]: {}\n- Local engine version [System verification]: {}\n- Fingerprint algorithm [System verification]: {}\n- Local source Evidence ID [System verification]: {}\n- Local source path [System verification]: {}\n- Local source SHA-256 [System verification]: {}\n- Local source size (bytes) [System verification]: {}\n- Local measured duration (ms) [System verification]: {}\n- Local record path [System verification]: {}\n- Local record SHA-256 [System verification]: {}\n- Local generated at [System value]: {}\n\n- External screening provider [System value]: {}\n- External screening status [System verification]: {}\n- External provider configured at snapshot [System value]: {}\n- External source Evidence ID [System verification]: {}\n- External source path [System verification]: {}\n- External source SHA-256 [System verification]: {}\n- External checked at [System value]: {}\n- External sample offset (ms) [System value]: {}\n- External sample duration (ms) [System value]: {}\n- External source duration (ms) [System value]: {}\n- External request count [System value]: {}\n- External response archive [System verification]: {}\n- External response SHA-256 [System verification]: {}\n{match_block}\nAudio-screening results are technical comparison records only. They do not establish authorship, ownership, permission, infringement, legality, release clearance, or any legal conclusion.\n",
        audio_screening_status_label(local.status),
        value_or_missing(&local.engine),
        value_or_missing(&local.engine_version),
        value_or_missing(&local.fingerprint_algorithm),
        value_or_missing(&local.source_evidence_id),
        value_or_missing(&local.source_relative_path),
        value_or_missing(&local.source_sha256),
        local.source_size_bytes,
        local
            .duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        value_or_missing(&local.artifact_relative_path),
        value_or_missing(&local.artifact_sha256),
        local.generated_at.as_deref().unwrap_or("NOT DOCUMENTED"),
        value_or_missing(&external.provider),
        audio_screening_status_label(external.status),
        yes_no(external.configured_at_snapshot),
        value_or_missing(&external.source_evidence_id),
        value_or_missing(&external.source_relative_path),
        value_or_missing(&external.source_sha256),
        external.checked_at.as_deref().unwrap_or("NOT DOCUMENTED"),
        external
            .sample_offset_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external
            .sample_duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external
            .source_duration_milliseconds
            .map(|value| value.to_string())
            .unwrap_or_else(|| "NOT DOCUMENTED".into()),
        external.request_count,
        external
            .response_relative_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("NOT RECORDED"),
        external
            .response_sha256
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("NOT RECORDED"),
    )
}

fn audio_screening_status_label(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::NotRun => "NOT RUN",
        AudioScreeningStatus::FingerprintGenerated => "FINGERPRINT GENERATED",
        AudioScreeningStatus::NoMatchDetected => "NO MATCH DETECTED",
        AudioScreeningStatus::MatchDetected => "MATCH DETECTED",
        AudioScreeningStatus::SkippedNotConfigured => "SKIPPED NOT CONFIGURED",
        AudioScreeningStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        AudioScreeningStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        AudioScreeningStatus::ConfigurationInvalid => "CONFIGURATION INVALID",
        AudioScreeningStatus::EngineUnavailable => "ENGINE UNAVAILABLE",
        AudioScreeningStatus::UnsupportedFormat => "UNSUPPORTED FORMAT",
        AudioScreeningStatus::ProcessingFailed => "PROCESSING FAILED",
        AudioScreeningStatus::Stale => "STALE",
    }
}

fn render(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    _steps: &[StepState],
) -> BTreeMap<String, String> {
    let phase_one_evidence = evidence
        .iter()
        .filter(|item| item.role != EvidenceRole::ExternalTimestamp)
        .cloned()
        .collect::<Vec<_>>();
    let evidence = phase_one_evidence.as_slice();
    let automation = crate::workflow::automation_summary(track, evidence);
    // Rendering is defensive as well as patch normalization: older workspace records may still
    // contain hidden answers from a formerly active conditional branch.
    let normalized_fields = track.fields.normalized_conditionals();
    let f = &normalized_fields;
    let artwork_present = !matches!(f.artwork_origin.as_str(), "" | "none");
    let ai_artwork = matches!(f.artwork_origin.as_str(), "ai_generated" | "ai_assisted");
    let ai_transparency_disabled = ai_artwork
        && f.depicts_real_person == Some(false)
        && f.depicts_real_event == Some(false)
        && f.contains_trademark == Some(false);
    let ai_original = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::AiArtworkOriginal)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT DOCUMENTED");
    let final_artwork = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::FinalArtwork)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT DOCUMENTED");
    let source_code_file = evidence_path(evidence, crate::model::EvidenceRole::SourceCodeFile);
    let code_generated_audio_file =
        evidence_path(evidence, crate::model::EvidenceRole::CodeGeneratedAudioFile);
    let mut source_declarations = format!(
        "\n## Source declarations\n\n- External audio uploaded: {}\n",
        yes_no(f.external_audio_uploaded)
    );
    if f.external_audio_uploaded == Some(true) {
        source_declarations.push_str(&format!(
            "- External audio source category: {}\n- External audio rights basis: {}\n",
            english_guided_value(&f.external_audio_source, SOURCE_CHOICES),
            english_guided_value(&f.external_audio_ownership, RIGHTS_CHOICES)
        ));
    }
    source_declarations.push_str(&format!(
        "- Own audio uploaded: {}\n",
        yes_no(f.own_audio_uploaded)
    ));
    if f.own_audio_uploaded == Some(true) {
        source_declarations.push_str(&format!(
            "- Own audio source category: {}\n- Own audio rights basis: {}\n",
            english_guided_value(&f.own_audio_source, SOURCE_CHOICES),
            english_guided_value(&f.own_audio_ownership, RIGHTS_CHOICES)
        ));
    }
    source_declarations.push_str(&format!(
        "- Code-based generation: {}\n",
        yes_no(f.code_based_generation)
    ));
    if f.code_based_generation == Some(true) {
        source_declarations.push_str(&format!("- Source-code evidence: {source_code_file}\n"));
        source_declarations.push_str(&format!(
            "- Post-processing performed: {}\n",
            yes_no(f.code_audio_post_processed)
        ));
        if f.code_audio_post_processed == Some(true) {
            source_declarations.push_str(&format!(
                "- Post-processing operations: {}\n",
                documented_list(&f.code_audio_post_processing_operations)
            ));
            if !f.code_audio_post_processing_note.trim().is_empty() {
                source_declarations.push_str(&format!(
                    "- Other post-processing note: {}\n",
                    value_or_missing(&f.code_audio_post_processing_note)
                ));
            }
        }
        source_declarations.push_str(&format!(
            "- Code-generated audio evidence: {code_generated_audio_file}\n"
        ));
    }
    source_declarations.push_str(&format!(
        "- Third-party samples uploaded: {}\n",
        yes_no(f.third_party_samples_uploaded)
    ));
    if f.third_party_samples_uploaded == Some(true) {
        source_declarations.push_str(&format!(
            "- Third-party sample source category: {}\n- Third-party sample rights basis: {}\n",
            english_guided_value(&f.third_party_sample_source, SOURCE_CHOICES),
            english_guided_value(&f.third_party_sample_ownership, RIGHTS_CHOICES)
        ));
    }
    let lyrics_document = render_suno_field_document(f);

    let style_document = format!(
        "{}# Suno style prompt\n\n{}\n",
        marker(),
        value_or_missing(&f.suno_style_prompt)
    );

    let mut confirmed_work = format!(
        "\n## Confirmed work and release choices\n\n- Human editing performed: {}\n",
        yes_no(f.human_editing_performed)
    );
    if f.human_editing_performed == Some(true) {
        confirmed_work.push_str(&format!(
            "- Confirmed human work: {}\n",
            english_guided_list(&f.human_editing_details, HUMAN_WORK_CHOICES)
        ));
    }
    confirmed_work.push_str(&format!(
        "- Desktop-PC editing after the Suno WAV: {}\n",
        yes_no(f.post_export_editing_performed)
    ));
    if f.post_export_editing_performed == Some(true) {
        confirmed_work.push_str(&format!(
            "- Confirmed desktop-PC editing work: {}\n",
            english_guided_list(&f.post_export_editing_details, POST_EXPORT_CHOICES)
        ));
    }
    confirmed_work.push_str(&format!(
        "- Release notes: {}\n",
        english_guided_list(&f.release_notes, RELEASE_CHOICES)
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
                    documented_list(&f.human_artwork_modifications)
                ));
                if !f.custom_artwork_change.trim().is_empty() {
                    ai_artwork_usage.push_str(&format!(
                        "- Other human editing details: {}\n",
                        value_or_missing(&f.custom_artwork_change)
                    ));
                }
            }
            if ai_transparency_disabled {
                ai_artwork_usage.push_str(
                    "- AI transparency step: Disabled after all three content checks were answered No\n",
                );
            } else {
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
        }
        ai_artwork_usage.push_str(&format!("- Final output: {}\n", final_artwork));
    }

    let image_generation_document = if ai_artwork {
        let mut content = format!(
            "{}# AI image generation record\n\n- AI image service: {}\n- Artwork origin: {}\n",
            marker(),
            value_or_missing(&f.ai_image_service),
            value_or_missing(&f.artwork_origin)
        );
        if ai_transparency_disabled {
            content.push_str("- AI transparency step: Disabled after all three content checks were answered No\n");
        } else {
            content.push_str(&format!(
                "- Project transparency policy: {}\n- Disclosure applied: {}\n",
                profile.artwork_transparency_policy,
                yes_no(f.disclosure_applied)
            ));
            if f.disclosure_applied == Some(true) {
                content.push_str(&format!(
                    "- Disclosure text: {}\n",
                    value_or_missing(&f.disclosure_text)
                ));
            }
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
        if f.artwork_origin == "human" {
            artwork_document.push_str(&format!(
                "- Human process operations: {}\n",
                documented_list(&f.human_artwork_process_operations)
            ));
            if !f.human_artwork_process_notes.trim().is_empty() {
                artwork_document.push_str(&format!(
                    "- Human process notes: {}\n",
                    value_or_missing(&f.human_artwork_process_notes)
                ));
            }
        }
        if ai_artwork {
            artwork_document.push_str(&format!(
                "- AI service: {}\n- AI-generated base image: {}\n",
                value_or_missing(&f.ai_image_service),
                ai_original
            ));
            if f.artwork_origin == "ai_assisted" {
                artwork_document.push_str(&format!(
                    "- Human modifications: {}\n",
                    documented_list(&f.human_artwork_modifications)
                ));
                if !f.custom_artwork_change.trim().is_empty() {
                    artwork_document.push_str(&format!(
                        "- Other human editing details: {}\n",
                        value_or_missing(&f.custom_artwork_change)
                    ));
                }
            }
            if ai_transparency_disabled {
                artwork_document.push_str(
                    "- AI transparency step: Disabled after all three content checks were answered No\n",
                );
            } else {
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
        }
        artwork_document.push_str(&format!(
            "- Final output: {}\n- Real person intentionally depicted: {}\n",
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
            "- Real event represented as authentic: {}\n",
            yes_no(f.depicts_real_event)
        ));
        if f.depicts_real_event == Some(true) {
            artwork_document.push_str(&format!(
                "- Real-event note: {}\n",
                value_or_missing(&f.real_event_notes)
            ));
        }
        artwork_document.push_str(&format!(
            "- Trademark or company logo reproduced: {}\n",
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

    let terms_evidence = evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .cloned()
        .collect::<Vec<_>>();
    let terms_evidence_status = terms_evidence_status(evidence);
    let release_identity_status = release_identity_status(evidence);
    let terms_ids = if terms_evidence.is_empty() {
        "N/A".to_owned()
    } else {
        terms_evidence
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let timestamp_recommendation = if f.commercial_use_intended {
        "\nFor long-term evidentiary preservation, an external timestamp can be added after technical finalization.\n"
    } else {
        ""
    };
    let audio_screening_summary = audio_screening_documentation_summary(&track.audio_screening);
    let mut values = BTreeMap::new();
    values.insert(
        "02_SUNO/suno_project.txt".into(),
        format!(
            "# {MANAGED_MARKER}\nTemplate version: {TEMPLATE_VERSION}\nTrack: {}\nFinal generation date [{}]: {}\nFinal generation ID [{}]: {}\nSuno project URL [User-confirmed fact]: {}\nDownload/export date [{}]: {}\nSuno Studio metadata detected [System verification]: {}\nMetadata origin: {}\nSuno model [User-confirmed fact]: {}\nSuno plan at generation [User-confirmed fact]: {}\nLegacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\nRelease identical to Suno final export [System verification]: {}\nProduction start: {}\nProduction end: {}\nLast editing date: {}\nActual Suno export filename: {}\nInstrumental track [User-confirmed fact]: {}\nVocal lyrics present [User-confirmed fact]: {}\nSuno lyrics/structure field content [User-confirmed fact]: {}\nContent types [User-confirmed fact]: {}\nContent source [User-confirmed fact]: {}\nExternal audio uploaded: {}\nOwn audio uploaded: {}\nCode-based generation: {}\nSource-code evidence: {}\nCode-audio post-processing performed: {}\nCode-audio post-processing operations: {}\nCode-generated audio evidence: {}\nThird-party samples uploaded: {}\n",
            f.title,
            fact_origin_label(automation.final_generation_origin),
            value_or_missing(&f.suno_final_generation_date),
            fact_origin_label(automation.final_generation_id_origin),
            value_or_missing(&f.suno_final_generation_id),
            value_or_missing(&f.suno_project_url),
            fact_origin_label(automation.download_export_origin),
            value_or_missing(&f.suno_download_export_date),
            if automation.suno_metadata_detected { "YES" } else { "NO" },
            if automation.suno_metadata_detected { "Evidence-derived metadata" } else { "NOT DOCUMENTED" },
            value_or_missing(&f.suno_model),
            value_or_missing(&f.suno_plan_at_generation),
            value_or_missing(&f.legacy_suno_plan_at_creation),
            release_identity_status,
            value_or_missing(&f.production_start_date),
            value_or_missing(&f.production_end_date),
            value_or_missing(&f.final_export_date),
            crate::workflow::original_evidence_file_name(evidence, EvidenceRole::SunoFinalExport).unwrap_or("NOT RECORDED"),
            yes_no(f.instrumental_track),
            yes_no(f.vocal_lyrics_present),
            yes_no(f.suno_lyrics_field_content),
            suno_content_types(f),
            suno_content_source(f),
            yes_no(f.external_audio_uploaded),
            yes_no(f.own_audio_uploaded),
            yes_no(f.code_based_generation),
            conditional_value(f.code_based_generation, source_code_file),
            if f.code_based_generation == Some(true) { yes_no(f.code_audio_post_processed) } else { "N/A" },
            if f.code_based_generation == Some(true) && f.code_audio_post_processed == Some(true) { documented_list(&f.code_audio_post_processing_operations) } else { "N/A".into() },
            conditional_value(f.code_based_generation, code_generated_audio_file),
            yes_no(f.third_party_samples_uploaded),
        ),
    );
    values.insert(
        "03_DOCUMENTATION/README.md".into(),
        format!(
            "{}# Track documentation: {}\n\nTemplate version: `{}`\nWorkflow: `{}` version `{}`\n\n## Snapshot\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Suno plan at generation: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Commercial use intended: {}\n- Production period: {} to {}\n- Last editing date: {}\n- Final generation date [{}]: {}\n- Final generation ID [{}]: {}\n- Suno project URL: {}\n- Download/export date [{}]: {}\n- Actual release filename: {}\n- Actual Suno export filename: {}\n- Instrumental track: {}\n- Vocal lyrics present: {}\n- Suno lyrics/structure field content: {}\n{}{}\n{}\n## Workflow status\n\n- Documentation status meaning: configured documentation requirements completed.\n- PASS means: Configured documentation requirements for this step were satisfied.\n- The authoritative evaluated step results are stored in the completion certificate after finalization.\n\n## External Timestamp Evidence\n\n- External timestamp evidence at technical finalization: NOT RECORDED\n- No external timestamp evidence recorded.\n{}\n## Evidence\n\n{}",
            marker(), f.title, TEMPLATE_VERSION, track.workflow_id, track.workflow_version,
            value_or_missing(&profile.artist_name), value_or_missing(&profile.suno_profile_name),
            value_or_missing(&profile.suno_handle), value_or_missing(&f.suno_plan_at_generation),
            value_or_missing(&f.legacy_suno_plan_at_creation),
            if f.commercial_use_intended { "YES" } else { "NO" },
            value_or_missing(&f.production_start_date), value_or_missing(&f.production_end_date),
            value_or_missing(&f.final_export_date),
            fact_origin_label(automation.final_generation_origin),
            value_or_missing(&f.suno_final_generation_date),
            fact_origin_label(automation.final_generation_id_origin),
            value_or_missing(&f.suno_final_generation_id),
            value_or_missing(&f.suno_project_url),
            fact_origin_label(automation.download_export_origin),
            value_or_missing(&f.suno_download_export_date),
            crate::workflow::original_evidence_file_name(evidence, EvidenceRole::ReleaseWav).unwrap_or("NOT RECORDED"),
            crate::workflow::original_evidence_file_name(evidence, EvidenceRole::SunoFinalExport).unwrap_or("NOT RECORDED"),
            yes_no(f.instrumental_track),
            yes_no(f.vocal_lyrics_present),
            yes_no(f.suno_lyrics_field_content),
            source_declarations,
            confirmed_work,
            audio_screening_summary,
            timestamp_recommendation,
            evidence_list(evidence)
        ),
    );
    values.insert(
        "03_DOCUMENTATION/AI_USAGE.md".into(),
        format!(
            "{}# AI usage\n\n## Final Suno Generation\n\n- Final generation date [{}]: {}\n- Final generation ID [{}]: {}\n- Suno project URL [User-confirmed fact]: {}\n- Download/export date [{}]: {}\n- Suno Studio metadata detected [System verification]: {}\n- Suno model [User-confirmed fact]: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Release identical to Suno final export [System verification]: {}\n- Instrumental track [User-confirmed fact]: {}\n- Vocal lyrics present [User-confirmed fact]: {}\n- Suno lyrics/structure field content [User-confirmed fact]: {}\n- Content types [User-confirmed fact]: {}\n- Content source [User-confirmed fact]: {}\n- External audio uploaded: {}\n- Code-based generation: {}\n- Source-code evidence: {}\n- Code-audio post-processing performed: {}\n- Code-audio post-processing operations: {}\n- Code-generated audio evidence: {}\n\n## AI Transparency Assessment – Audio\n\n{}\nNo AI Act compliance, legal necessity, or legal safety determination is made.\n\n## AI Transparency Assessment – Artwork\n\n{}",
            marker(), fact_origin_label(automation.final_generation_origin),
            value_or_missing(&f.suno_final_generation_date), fact_origin_label(automation.final_generation_id_origin),
            value_or_missing(&f.suno_final_generation_id), value_or_missing(&f.suno_project_url),
            fact_origin_label(automation.download_export_origin),
            value_or_missing(&f.suno_download_export_date), if automation.suno_metadata_detected { "YES" } else { "NO" },
            value_or_missing(&f.suno_model), value_or_missing(&f.suno_plan_at_generation),
            value_or_missing(&f.legacy_suno_plan_at_creation),
            release_identity_status,
            yes_no(f.instrumental_track), yes_no(f.vocal_lyrics_present), yes_no(f.suno_lyrics_field_content),
            suno_content_types(f), suno_content_source(f), yes_no(f.external_audio_uploaded),
            yes_no(f.code_based_generation),
            conditional_value(f.code_based_generation, source_code_file),
            if f.code_based_generation == Some(true) { yes_no(f.code_audio_post_processed) } else { "N/A" },
            if f.code_based_generation == Some(true) && f.code_audio_post_processed == Some(true) { documented_list(&f.code_audio_post_processing_operations) } else { "N/A".into() },
            conditional_value(f.code_based_generation, code_generated_audio_file),
            render_ai_audio(f),
            ai_artwork_usage
        ),
    );
    values.insert("02_SUNO/Lyrics.md".into(), lyrics_document);
    values.insert("02_SUNO/Style.md".into(), style_document);
    values.insert(
        "04_LICENSES/suno_account_and_license.md".into(),
        format!(
            "{}# Suno account, subscription, and archived terms evidence\n\n- Artist: {}\n- Suno profile: {}\n- Suno handle: {}\n- Suno plan at generation [User-confirmed fact]: {}\n- Legacy plan-at-creation value [Historical user data; not a plan-at-generation claim]: {}\n- Workspace subscription start date: {}\n- Commercial use intended: {}\n- Final generation date: {}\n- Assigned subscription evidence jointly covers the production period [System verification]: {}\n- Final-generation date covered [System verification]: {}\n- Terms evidence exists [System verification]: {}\n- Terms evidence IDs: {}\n- Terms evidence not available [User-confirmed fact]: {}\n\n## Archived service-terms evidence\n\n{}\nThis page records supplied account facts and locally archived evidence only. It does not confirm rights ownership, license validity, legality, or non-infringement.\n",
            marker(), value_or_missing(&profile.artist_name), value_or_missing(&profile.suno_profile_name),
            value_or_missing(&profile.suno_handle), value_or_missing(&f.suno_plan_at_generation),
            value_or_missing(&f.legacy_suno_plan_at_creation),
            value_or_missing(&profile.subscription_start_date), if f.commercial_use_intended { "YES" } else { "NO" },
            value_or_missing(&f.suno_final_generation_date),
            match crate::workflow::subscription_production_coverage(track, evidence) {
                crate::workflow::CoverageStatus::Yes => "YES",
                crate::workflow::CoverageStatus::No => "NO",
                crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
            },
            match crate::workflow::subscription_generation_coverage(track, evidence) {
                crate::workflow::CoverageStatus::Yes => "YES",
                crate::workflow::CoverageStatus::No => "NO",
                crate::workflow::CoverageStatus::NotVerified => "NOT VERIFIED",
            },
            terms_evidence_status,
            terms_ids,
            yes_no(f.suno_terms_evidence_not_available),
            evidence_list(&terms_evidence)
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

    const INACTIVE_FIXTURE_VALUES: [&str; 8] = [
        "INACTIVE-LYRICS",
        "INACTIVE-EXTERNAL-SOURCE",
        "INACTIVE-EXTERNAL-OWNERSHIP",
        "INACTIVE-SAMPLE-SOURCE",
        "INACTIVE-SAMPLE-OWNERSHIP",
        "INACTIVE-POST-EXPORT-EDIT",
        "INACTIVE-REAL-PERSON-NOTE",
        "INACTIVE-TRADEMARK-NOTE",
    ];

    const PRIVATE_FIXTURE_VALUES: [&str; 5] = [
        "private.person@example.invalid",
        "+49 30 555 0100",
        "1990-01-01",
        "/home/fixture-user",
        "password-secret",
    ];

    const FORBIDDEN_LEGAL_CLAIMS: [&str; 7] = [
        "guaranteed not to infringe copyright",
        "license is legally sufficient",
        "governmentally certified",
        "we certify legal compliance",
        "copyright ownership is confirmed",
        "owns all copyrights",
        "universal legal requirement",
    ];

    const ADOPTION_SENTINEL: &[u8] =
        b"user-authored sentinel document\n\0preserve these exact bytes\n";

    fn record_with_fields(fields: crate::model::TrackFields) -> TrackRecord {
        TrackRecord {
            id: "conditional-render-test".into(),
            relative_path: "conditional-render-test".into(),
            status: crate::model::TrackStatus::Active,
            workflow_id: "suno-track-documentation".into(),
            workflow_version: "1.3".into(),
            profile_snapshot: Profile::default(),
            library: Default::default(),
            field_origins: Default::default(),
            fields,
            audio_screening: Default::default(),
            documents: crate::model::DocumentState::default(),
            integrity: crate::model::IntegrityState::default(),
            certificate: crate::model::CertificateState::default(),
            created_at: "2026-08-13T00:00:00Z".into(),
            updated_at: "2026-08-13T00:00:00Z".into(),
            legacy: false,
        }
    }

    fn fixture_input() -> (TrackRecord, Profile, Vec<EvidenceItem>) {
        let mut fields = crate::model::TrackFields {
            title: "Golden Signal".into(),
            production_start_date: "2026-02-03".into(),
            production_end_date: "2026-02-05".into(),
            suno_model: "v4.5".into(),
            suno_project_url: "https://suno.example/projects/golden-signal".into(),
            suno_project_version_id: "project-version-golden".into(),
            suno_final_generation_id: "generation-golden".into(),
            suno_final_generation_date: "2026-02-05".into(),
            suno_final_generation_time: "14:35".into(),
            suno_download_export_date: "2026-02-06".into(),
            suno_plan_at_generation: "Pro".into(),
            final_export_date: "2026-02-06".into(),
            instrumental_track: Some(true),
            vocal_lyrics_present: Some(false),
            suno_lyrics_field_content: Some(true),
            suno_lyrics_content_types: vec![
                SunoLyricsContentType::StructureInstructions,
                SunoLyricsContentType::SoundInstructions,
            ],
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: "[Intro]\n[Drop]\n[Outro]".into(),
            suno_lyrics_other_content_type: INACTIVE_FIXTURE_VALUES[0].into(),
            suno_style_prompt: "dark synthwave, driving bass, cinematic".into(),
            external_audio_uploaded: Some(false),
            external_audio_source: INACTIVE_FIXTURE_VALUES[1].into(),
            external_audio_ownership: INACTIVE_FIXTURE_VALUES[2].into(),
            own_audio_uploaded: Some(true),
            own_audio_source: "Original field recording".into(),
            own_audio_ownership: "Solely owned by the artist".into(),
            code_based_generation: Some(false),
            code_audio_post_processed: None,
            code_audio_post_processing_operations: Vec::new(),
            code_audio_post_processing_note: String::new(),
            third_party_samples_uploaded: Some(false),
            third_party_sample_source: INACTIVE_FIXTURE_VALUES[3].into(),
            third_party_sample_ownership: INACTIVE_FIXTURE_VALUES[4].into(),
            human_editing_performed: Some(true),
            human_editing_details: "Timing and cuts | EQ".into(),
            post_export_editing_performed: Some(false),
            post_export_editing_details: INACTIVE_FIXTURE_VALUES[5].into(),
            commercial_use_intended: true,
            suno_terms_evidence_not_available: Some(true),
            artwork_origin: "ai_assisted".into(),
            ai_image_service: "Example Image Service".into(),
            human_artwork_process_operations: Vec::new(),
            human_artwork_process_notes: String::new(),
            human_artwork_modifications: vec![
                "Cropping".into(),
                "Brightness/contrast adjusted".into(),
            ],
            custom_artwork_change: String::new(),
            depicts_real_person: Some(false),
            real_person_notes: INACTIVE_FIXTURE_VALUES[6].into(),
            depicts_real_event: Some(true),
            real_event_notes: "Synthetic night-sky scene.".into(),
            contains_trademark: Some(false),
            trademark_notes: INACTIVE_FIXTURE_VALUES[7].into(),
            disclosure_applied: Some(false),
            disclosure_text: String::new(),
            release_notes: "Streaming master".into(),
            ..Default::default()
        };
        fields.normalize_conditionals();

        let profile = Profile {
            artist_name: "Fixture Artist".into(),
            suno_profile_name: "Fixture Profile".into(),
            suno_handle: "@fixture-artist".into(),
            suno_plan: "Pro".into(),
            subscription_start_date: "2026-01-15".into(),
            default_commercial_use: true,
            default_ai_image_service: "Example Image Service".into(),
            artwork_transparency_policy: "always".into(),
            disclosure_text: "AI-assisted".into(),
            certificate_language: crate::model::CertificateLanguage::En,
        };

        let evidence = vec![
            fixture_evidence(
                "final-artwork",
                crate::model::EvidenceRole::FinalArtwork,
                "05_ARTWORK/final-cover.png",
                '3',
            ),
            fixture_evidence(
                "release-wav",
                crate::model::EvidenceRole::ReleaseWav,
                "01_RELEASE/golden-signal.wav",
                '1',
            ),
            fixture_evidence(
                "ai-original",
                crate::model::EvidenceRole::AiArtworkOriginal,
                "05_ARTWORK/ai-original.png",
                '2',
            ),
        ];

        (record_with_fields(fields), profile, evidence)
    }

    fn fixture_evidence(
        id: &str,
        role: crate::model::EvidenceRole,
        relative_path: &str,
        hash_digit: char,
    ) -> EvidenceItem {
        EvidenceItem {
            id: id.to_owned(),
            role,
            file_name: "birthday-1990-01-01.png".into(),
            relative_path: relative_path.into(),
            sha256: Some(std::iter::repeat_n(hash_digit, 64).collect()),
            size_bytes: 1_024,
            imported_at: "2026-02-06T12:00:00Z".into(),
            verified: true,
            verification_error: Some("password-secret +49 30 555 0100 /home/fixture-user".into()),
            source_global_evidence_id: None,
            coverage_start: None,
            coverage_end: None,
            provenance: crate::model::EvidenceProvenance::ManagedCopy,
            derived_from_evidence_id: None,
            generator_version: None,
            generated_disclosure_text: None,
            metadata: crate::model::EvidenceMetadata {
                original_file_name: Path::new(relative_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("portable fixture file name")
                    .into(),
                ..Default::default()
            },
        }
    }

    fn write_adoption_sentinel(track_root: &Path) -> PathBuf {
        let relative = Path::new("03_DOCUMENTATION/README.md");
        let path = track_root.join(relative);
        fs::create_dir_all(path.parent().expect("sentinel parent"))
            .expect("create sentinel parent");
        fs::write(&path, ADOPTION_SENTINEL).expect("write adoption sentinel");
        path
    }

    #[test]
    fn all_documents_are_deterministic_and_exclude_forbidden_content() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let (track, profile, evidence) = fixture_input();

        let generated = generate(workspace.path(), &track, &profile, &evidence, &[], false)
            .expect("generate fixture documents");
        assert_eq!(generated.len(), DOCUMENT_PATHS.len());

        let mut first_generation = BTreeMap::new();
        let mut combined = String::new();
        for relative in DOCUMENT_PATHS {
            let actual = fs::read(workspace.path().join(relative))
                .unwrap_or_else(|error| panic!("read generated document {relative}: {error}"));
            combined.push_str(
                std::str::from_utf8(&actual)
                    .unwrap_or_else(|error| panic!("generated UTF-8 for {relative}: {error}")),
            );
            first_generation.insert(relative, actual);
        }
        assert!(
            combined.contains("Template version: `1.9`")
                || combined.contains("Template version: 1.9")
        );
        assert!(combined.contains("Suno Structure / Generation Instructions"));
        assert!(combined.contains("Vocal lyrics present [User-confirmed fact]: NO"));
        assert!(combined.contains("Final generation ID [User-confirmed fact]: generation-golden"));
        assert!(!combined.contains("Suno project/version ID"));
        assert!(!combined.contains("project-version-golden"));
        assert!(combined
            .contains("External timestamp evidence at technical finalization: NOT RECORDED"));

        generate(workspace.path(), &track, &profile, &evidence, &[], false)
            .expect("regenerate identical fixture documents");
        for (relative, first_bytes) in first_generation {
            let second_bytes = fs::read(workspace.path().join(relative))
                .unwrap_or_else(|error| panic!("read regenerated document {relative}: {error}"));
            assert_eq!(
                second_bytes, first_bytes,
                "nondeterministic bytes for {relative}"
            );
        }

        for inactive in INACTIVE_FIXTURE_VALUES {
            assert!(
                !combined.contains(inactive),
                "inactive conditional value was rendered: {inactive}"
            );
        }
        for private in PRIVATE_FIXTURE_VALUES {
            assert!(
                !combined.contains(private),
                "private metadata was rendered: {private}"
            );
        }
        let lowercase = combined.to_lowercase();
        for forbidden in FORBIDDEN_LEGAL_CLAIMS {
            assert!(
                !lowercase.contains(forbidden),
                "forbidden legal claim was rendered: {forbidden}"
            );
        }
    }

    #[test]
    fn document_generation_reports_each_managed_output() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let (track, profile, evidence) = fixture_input();
        let mut progress_events = Vec::new();

        let generated = generate_with_progress(
            workspace.path(),
            &track,
            &profile,
            &evidence,
            &[],
            false,
            &mut |progress| progress_events.push(progress),
        )
        .expect("generate fixture documents with progress");

        assert_eq!(generated.len(), DOCUMENT_PATHS.len());
        for expected_stage in [
            "preparing_documents",
            "rendering_documents",
            "writing_documents",
            "finalizing_documents",
        ] {
            assert!(
                progress_events
                    .iter()
                    .any(|progress| progress.stage == expected_stage),
                "missing progress stage {expected_stage}"
            );
        }

        for relative in DOCUMENT_PATHS {
            assert!(
                progress_events.iter().any(|progress| {
                    progress.stage == "writing_documents"
                        && progress.current_file.as_deref() == Some(relative)
                }),
                "missing progress event for {relative}"
            );
        }

        let completed = progress_events
            .iter()
            .find(|progress| progress.stage == "finalizing_documents")
            .expect("final document progress");
        assert_eq!(completed.processed_files, DOCUMENT_PATHS.len() as u32);
        assert_eq!(completed.total_files, DOCUMENT_PATHS.len() as u32);
    }

    #[test]
    fn generation_moves_managed_lyrics_and_style_documents_into_suno_folder() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let legacy_directory = workspace.path().join("03_DOCUMENTATION");
        fs::create_dir_all(&legacy_directory).expect("legacy document directory");
        fs::write(
            legacy_directory.join("Lyrics.md"),
            format!("{}# Legacy lyrics\n", marker()),
        )
        .expect("legacy lyrics");
        fs::write(
            legacy_directory.join("Styles.md"),
            format!("{}# Legacy styles\n", marker()),
        )
        .expect("legacy styles");
        let (track, profile, evidence) = fixture_input();

        generate(workspace.path(), &track, &profile, &evidence, &[], false)
            .expect("generate migrated documents");

        assert!(workspace.path().join("02_SUNO/Lyrics.md").is_file());
        assert!(workspace.path().join("02_SUNO/Style.md").is_file());
        assert!(!legacy_directory.join("Lyrics.md").exists());
        assert!(!legacy_directory.join("Styles.md").exists());
    }

    #[test]
    fn guided_german_ui_labels_render_as_english_document_values() {
        let (mut track, profile, mut evidence) = fixture_input();
        track.fields.external_audio_uploaded = Some(true);
        track.fields.external_audio_source = "Lizenzierte Sample-Bibliothek".into();
        track.fields.external_audio_ownership = "Lizenz für kommerzielle Nutzung".into();
        track.fields.code_based_generation = Some(true);
        track.fields.human_editing_details = "Timing und Cuts | Lautheitsanpassung".into();
        track.fields.post_export_editing_performed = Some(true);
        track.fields.post_export_editing_details = "Schnitt | Mastering".into();
        track.fields.release_notes = "Originale Suno-Fassung | Radio Edit".into();
        evidence.push(fixture_evidence(
            "source-code",
            crate::model::EvidenceRole::SourceCodeFile,
            "02_SUNO/generator.py",
            '4',
        ));
        evidence.push(fixture_evidence(
            "code-generated-audio",
            crate::model::EvidenceRole::CodeGeneratedAudioFile,
            "02_SUNO/generated.wav",
            '5',
        ));

        let rendered = render(&track, &profile, &evidence, &[]);
        let combined = rendered.values().cloned().collect::<String>();

        for expected in [
            "External audio source category: Audio from a licensed sample library",
            "External audio rights basis: Commercial-use license",
            "Source-code evidence: 02_SUNO/generator.py",
            "Code-generated audio evidence: 02_SUNO/generated.wav",
            "Confirmed human work: Timing and cuts, Loudness adjustment",
            "Confirmed desktop-PC editing work: Editing and cuts, Mastering",
            "Release notes: Original Suno version, Radio edit",
        ] {
            assert!(
                combined.contains(expected),
                "missing English value: {expected}"
            );
        }
        for german_label in [
            "Lizenzierte Sample-Bibliothek",
            "Lizenz für kommerzielle Nutzung",
            "Timing und Cuts",
            "Lautheitsanpassung",
            "Originale Suno-Fassung",
        ] {
            assert!(
                !combined.contains(german_label),
                "localized UI label leaked into generated documents: {german_label}"
            );
        }
    }

    #[test]
    fn conditional_post_processing_and_artwork_facts_render_without_legal_inference() {
        let (mut track, profile, evidence) = fixture_input();
        track.fields.code_based_generation = Some(true);
        track.fields.code_audio_post_processed = Some(false);
        track.fields.code_audio_post_processing_operations = vec!["STALE-MIXING".into()];
        track.fields.code_audio_post_processing_note = "STALE-NOTE".into();

        let rendered = render(&track, &profile, &evidence, &[]);
        let readme = &rendered["03_DOCUMENTATION/README.md"];
        assert!(readme.contains("Post-processing performed: NO"));
        assert!(!readme.contains("Post-processing operations:"));
        assert!(!readme.contains("STALE-MIXING"));
        assert!(!readme.contains("STALE-NOTE"));

        track.fields.code_audio_post_processed = Some(true);
        track.fields.code_audio_post_processing_operations = vec![
            "Mixing".into(),
            "EQ".into(),
            "Compression".into(),
            "Mastering".into(),
            "Other post-processing".into(),
        ];
        track.fields.code_audio_post_processing_note = "Manual spectral repair".into();
        track.fields.artwork_origin = "human".into();
        track.fields.human_artwork_process_operations = vec![
            "Photographed".into(),
            "Retouching".into(),
            "Typography added".into(),
        ];
        track.fields.human_artwork_process_notes = "Manual darkroom scan".into();
        track.fields.depicts_real_person = Some(true);
        track.fields.real_person_notes = "The performing artist".into();
        track.fields.depicts_real_event = Some(false);
        track.fields.contains_trademark = Some(true);
        track.fields.trademark_notes = "A user-supplied company logo".into();

        let rendered = render(&track, &profile, &evidence, &[]);
        let combined = rendered.values().cloned().collect::<String>();
        for factual_statement in [
            "Post-processing performed: YES",
            "Post-processing operations: Mixing, EQ, Compression, Mastering, Other post-processing",
            "Other post-processing note: Manual spectral repair",
            "Human process operations: Photographed, Retouching, Typography added",
            "Human process notes: Manual darkroom scan",
            "Real person intentionally depicted: YES",
            "Real-person note: The performing artist",
            "Real event represented as authentic: NO",
            "Trademark or company logo reproduced: YES",
            "Trademark/logo note: A user-supplied company logo",
        ] {
            assert!(
                combined.contains(factual_statement),
                "missing factual document statement: {factual_statement}"
            );
        }
        let lowercase = combined.to_lowercase();
        for forbidden in FORBIDDEN_LEGAL_CLAIMS {
            assert!(!lowercase.contains(forbidden));
        }
    }

    #[test]
    fn explicit_vocal_and_structure_fields_render_without_legacy_inference() {
        let (mut track, profile, evidence) = fixture_input();
        track.fields.instrumental_track = Some(false);
        track.fields.vocal_lyrics_present = Some(true);
        track.fields.suno_lyrics_field_content = Some(true);
        track.fields.suno_lyrics_content_types = vec![SunoLyricsContentType::VocalLyrics];
        track.fields.suno_lyrics_content_source = Some(SunoLyricsContentSource::Human);
        track.fields.suno_lyrics_field_text = "Explicit vocal text".into();
        track.fields.lyrics_source.clear();
        track.fields.lyrics_text.clear();

        let rendered = render(&track, &profile, &evidence, &[]);
        let lyrics = &rendered["02_SUNO/Lyrics.md"];
        assert!(lyrics.contains("# Vocal Lyrics"));
        assert!(lyrics.contains("Instrumental track [User-confirmed fact]: NO"));
        assert!(lyrics.contains("Vocal lyrics present [User-confirmed fact]: YES"));
        assert!(lyrics.contains("Explicit vocal text"));
        assert!(!lyrics.contains("Unclassified legacy lyrics data"));
    }

    #[test]
    fn complete_terms_metadata_uses_same_evidence_id_in_summary_and_register() {
        let (track, profile, mut evidence) = fixture_input();
        let mut terms = fixture_evidence(
            "terms-record",
            EvidenceRole::SunoTermsRights,
            "04_LICENSES/suno-terms.pdf",
            '8',
        );
        terms.metadata.document_title = "Suno Terms of Service".into();
        terms.metadata.provider = "Suno, Inc.".into();
        terms.metadata.source_url = "https://suno.example/terms".into();
        terms.metadata.retrieval_date = "2026-02-01".into();
        terms.metadata.effective_date = "2026-01-01".into();
        terms.metadata.applicable_production_period = "2026-02-03 to 2026-02-05".into();
        terms.metadata.factual_note = "Locally archived copy".into();
        terms.verification_error = None;
        let evidence_id = terms.id.clone();
        evidence.push(terms);

        let rendered = render(&track, &profile, &evidence, &[]);
        let license = &rendered["04_LICENSES/suno_account_and_license.md"];
        assert!(license.contains("Terms evidence exists [System verification]: YES"));
        assert!(license.contains(&format!("Terms evidence IDs: {evidence_id}")));
        assert!(license.contains(&format!("Evidence ID `{evidence_id}`")));
        for value in [
            "Suno Terms of Service",
            "Suno, Inc.",
            "https://suno.example/terms",
            "2026-02-01",
            "2026-01-01",
            "2026-02-03 to 2026-02-05",
            "Locally archived copy",
            "04_LICENSES/suno-terms.pdf",
        ] {
            assert!(license.contains(value), "missing terms value: {value}");
        }
    }

    #[test]
    fn managed_documents_do_not_call_unverified_terms_or_missing_hash_pairs_verified() {
        let (track, profile, mut evidence) = fixture_input();
        let unverified_terms = fixture_evidence(
            "unverified-terms",
            EvidenceRole::SunoTermsRights,
            "04_LICENSES/unverified-terms.pdf",
            '8',
        );
        evidence.push(unverified_terms);

        let rendered = render(&track, &profile, &evidence, &[]);
        assert!(rendered["04_LICENSES/suno_account_and_license.md"]
            .contains("Terms evidence exists [System verification]: NOT VERIFIED"));
        for path in ["02_SUNO/suno_project.txt", "03_DOCUMENTATION/AI_USAGE.md"] {
            assert!(rendered[path].contains(
                "Release identical to Suno final export [System verification]: NOT VERIFIED"
            ));
        }

        let release = evidence
            .iter_mut()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release fixture");
        release.verification_error = None;
        let mut suno = fixture_evidence(
            "suno-export",
            EvidenceRole::SunoFinalExport,
            "02_SUNO/suno-export.wav",
            '1',
        );
        suno.verification_error = None;
        evidence.push(suno);
        let rendered = render(&track, &profile, &evidence, &[]);
        assert!(rendered["02_SUNO/suno_project.txt"]
            .contains("Release identical to Suno final export [System verification]: YES"));

        evidence
            .iter_mut()
            .find(|item| item.role == EvidenceRole::SunoFinalExport)
            .expect("Suno export fixture")
            .sha256 = Some("2".repeat(64));
        let rendered = render(&track, &profile, &evidence, &[]);
        assert!(rendered["02_SUNO/suno_project.txt"]
            .contains("Release identical to Suno final export [System verification]: NO"));
    }

    #[test]
    fn audio_transparency_distinguishes_complete_no_and_not_documented() {
        let (mut track, profile, evidence) = fixture_input();
        track.fields.generative_ai_used = Some(true);
        track.fields.audio_ai_system = "Suno".into();
        track.fields.ai_assisted_audio_elements = Some(DocumentationAnswer::No);
        track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::Yes);
        track.fields.real_person_voice_intentionally_imitated = Some(DocumentationAnswer::No);
        track.fields.real_person_identity_intentionally_represented = Some(DocumentationAnswer::No);
        track.fields.real_event_represented_as_authentic_recording = Some(DocumentationAnswer::No);
        track
            .fields
            .real_location_institution_event_presented_as_authentic_ai_recording =
            Some(DocumentationAnswer::No);
        track.fields.audio_disclosure_applied = Some(DocumentationAnswer::No);
        track.fields.audio_disclosure_reason = "Documented user reason".into();

        let rendered = render(&track, &profile, &evidence, &[]);
        let ai = &rendered["03_DOCUMENTATION/AI_USAGE.md"];
        assert!(ai.contains("Generative AI used [User-confirmed fact]: YES"));
        assert!(ai.contains("AI-assisted audio elements [User-confirmed fact]: NO"));
        assert!(ai.contains("Disclosure locations [User-confirmed fact]: N/A"));
        assert!(ai.contains("Documented user reason"));
        assert!(ai.contains("Documented deepfake-related indicators: none recorded"));
        assert!(!ai.contains("AI Act compliant"));

        track.fields.ai_generated_audio_elements = Some(DocumentationAnswer::NotDocumented);
        let rendered = render(&track, &profile, &evidence, &[]);
        assert!(rendered["03_DOCUMENTATION/AI_USAGE.md"]
            .contains("AI-generated audio elements [User-confirmed fact]: NOT DOCUMENTED"));
    }

    #[test]
    fn legacy_external_timestamp_is_excluded_from_phase_one_documents_and_fingerprint() {
        let (track, profile, mut evidence) = fixture_input();
        let without_timestamp = input_fingerprint(&track, &profile, &evidence)
            .expect("phase-one fingerprint without timestamp");
        let mut timestamp = fixture_evidence(
            "legacy-timestamp",
            EvidenceRole::ExternalTimestamp,
            "03_DOCUMENTATION/legacy-timestamp.pdf",
            '9',
        );
        timestamp.metadata.provider = "MUST-NOT-APPEAR-TIMESTAMP-PROVIDER".into();
        timestamp.metadata.external_timestamp = "2026-02-06T12:00:00Z".into();
        evidence.push(timestamp);

        assert_eq!(
            input_fingerprint(&track, &profile, &evidence)
                .expect("phase-one fingerprint with ignored timestamp"),
            without_timestamp
        );
        let rendered = render(&track, &profile, &evidence, &[]);
        let combined = rendered.values().cloned().collect::<String>();
        assert!(!combined.contains("MUST-NOT-APPEAR-TIMESTAMP-PROVIDER"));
        assert!(!combined.contains("legacy-timestamp.pdf"));
        assert!(combined
            .contains("External timestamp evidence at technical finalization: NOT RECORDED"));
    }

    #[test]
    fn audio_screening_changes_freshness_and_readme_without_raw_fingerprint() {
        let (mut track, profile, evidence) = fixture_input();
        let before = input_fingerprint(&track, &profile, &evidence).expect("initial fingerprint");
        track.audio_screening.local.status = AudioScreeningStatus::FingerprintGenerated;
        track.audio_screening.local.engine_version = "1.6.1".into();
        track.audio_screening.local.source_evidence_id = "release-wav".into();
        track.audio_screening.local.source_relative_path = "01_RELEASE/golden-signal.wav".into();
        track.audio_screening.local.source_sha256 = "1".repeat(64);
        track.audio_screening.local.fingerprint = "RAW_FINGERPRINT_MUST_NOT_RENDER".into();
        track.audio_screening.local.message = "ACCESS_SECRET_MUST_NOT_RENDER".into();
        track.audio_screening.local.artifact_relative_path =
            "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json".into();
        track.audio_screening.local.artifact_sha256 = "2".repeat(64);

        assert_ne!(
            before,
            input_fingerprint(&track, &profile, &evidence).expect("screening fingerprint")
        );
        let rendered = render(&track, &profile, &evidence, &[]);
        let readme = &rendered["03_DOCUMENTATION/README.md"];
        assert!(readme.contains("## Pre-release audio screening"));
        assert!(readme.contains("FINGERPRINT GENERATED"));
        assert!(readme.contains("LOCAL_FINGERPRINT.json"));
        assert!(!readme.contains("RAW_FINGERPRINT_MUST_NOT_RENDER"));
        assert!(!readme.contains("ACCESS_SECRET_MUST_NOT_RENDER"));
    }

    #[test]
    fn unknown_legacy_selection_is_retained_in_data_but_not_copied_into_english_documents() {
        let (mut track, profile, evidence) = fixture_input();
        track.fields.own_audio_source = "Historischer deutscher Freitext".into();

        let rendered = render(&track, &profile, &evidence, &[]);
        let readme = &rendered["03_DOCUMENTATION/README.md"];

        assert!(!readme.contains("Historischer deutscher Freitext"));
        assert!(readme.contains(LEGACY_SELECTION_NOTICE));
        assert_eq!(
            track.fields.own_audio_source,
            "Historischer deutscher Freitext"
        );
    }

    #[test]
    fn adopt_existing_false_leaves_unmanaged_sentinel_unchanged() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let sentinel = write_adoption_sentinel(workspace.path());
        let (track, profile, evidence) = fixture_input();

        let result = generate(workspace.path(), &track, &profile, &evidence, &[], false);

        assert!(matches!(result, Err(AppError::AdoptionRequired(_))));
        assert_eq!(
            fs::read(sentinel).expect("read sentinel"),
            ADOPTION_SENTINEL
        );
        assert!(!workspace.path().join("02_SUNO/suno_project.txt").exists());
    }

    #[test]
    fn adopt_existing_true_archives_exact_bytes_before_managed_replacement() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let sentinel = write_adoption_sentinel(workspace.path());
        let (track, profile, evidence) = fixture_input();
        let expected_readme = render(&track, &profile, &evidence, &[])
            ["03_DOCUMENTATION/README.md"]
            .as_bytes()
            .to_vec();

        generate(workspace.path(), &track, &profile, &evidence, &[], true)
            .expect("adopt unmanaged document");

        let adoption_roots = fs::read_dir(workspace.path().join(".archive/adoptions"))
            .expect("read adoption archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect adoption archive entries");
        assert_eq!(adoption_roots.len(), 1);
        assert!(adoption_roots[0]
            .file_type()
            .expect("adoption entry type")
            .is_dir());
        let backup = adoption_roots[0].path().join("03_DOCUMENTATION/README.md");
        assert_eq!(
            fs::read(backup).expect("read archived sentinel"),
            ADOPTION_SENTINEL
        );
        assert_eq!(
            fs::read(sentinel).expect("read managed replacement"),
            expected_readme
        );
    }

    #[test]
    fn forced_adoption_backup_failure_leaves_original_unchanged() {
        let workspace = tempfile::tempdir().expect("temporary track root");
        let sentinel = write_adoption_sentinel(workspace.path());
        fs::write(
            workspace.path().join(".archive"),
            b"block archive directory creation",
        )
        .expect("create forced archive failure");
        let (track, profile, evidence) = fixture_input();

        let result = generate(workspace.path(), &track, &profile, &evidence, &[], true);

        assert!(
            matches!(&result, Err(AppError::Io { .. })),
            "forced backup failure returned an unexpected result: {result:?}"
        );
        assert_eq!(
            fs::read(sentinel).expect("read sentinel"),
            ADOPTION_SENTINEL
        );
        assert!(!workspace.path().join("02_SUNO/suno_project.txt").exists());
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
            suno_lyrics_field_content: Some(false),
            suno_lyrics_field_text: stale_values[0].into(),
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
            human_artwork_modifications: vec![stale_values[10].into()],
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
        assert!(!rendered["02_SUNO/Lyrics.md"].contains("## Text"));
        assert!(!rendered["03_DOCUMENTATION/README.md"].contains("Confirmed human work"));
        assert!(!rendered["03_DOCUMENTATION/README.md"].contains("Confirmed post-export work"));
        assert!(!rendered["03_DOCUMENTATION/AI_USAGE.md"].contains("AI service:"));
        assert!(!rendered["05_ARTWORK/artwork_process.md"].contains("Real-person note:"));

        let clean = record_with_fields(stale.fields.normalized_conditionals());
        assert_eq!(
            input_fingerprint(&stale, &Profile::default(), &[]).expect("stale fingerprint"),
            input_fingerprint(&clean, &Profile::default(), &[]).expect("clean fingerprint")
        );
    }
}
