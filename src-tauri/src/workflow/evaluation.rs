use super::automation::consistency_issues;
use super::coverage::{
    subscription_generation_coverage, subscription_production_coverage, CoverageStatus,
};
use super::*;

pub fn evaluate(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    stored_steps: &[StepState],
) -> Result<WorkflowEvaluation> {
    let config = config()?;
    let missing_by_step = collect_missing_by_step(&config, track, profile, evidence, deviations);
    let steps = evaluated_steps(&config, track, &missing_by_step, stored_steps);
    let missing = config
        .steps
        .iter()
        .flat_map(|step| missing_by_step.get(&step.id).cloned().unwrap_or_default())
        .collect();
    Ok(WorkflowEvaluation { steps, missing })
}

fn collect_missing_by_step(
    config: &WorkflowConfig,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
) -> HashMap<String, Vec<String>> {
    let mut missing_by_step: HashMap<String, Vec<String>> = HashMap::new();
    let evidence_roles: HashSet<&str> = evidence
        .iter()
        .filter(|item| item.verified && item.sha256.is_some() && item.verification_error.is_none())
        .map(|item| item.role.as_str())
        .collect();

    for item in evidence {
        if !item.verified || item.sha256.is_none() || item.verification_error.is_some() {
            missing_by_step
                .entry("evidence_licenses".into())
                .or_default()
                .push(format!(
                    "Evidence is missing or not verified: {}",
                    item.relative_path
                ));
        }
    }

    // Consistency findings use the existing workflow/finalization gate. They
    // are derived from the same persisted facts and evidence records and do
    // not create a parallel deviation workflow for the user to maintain.
    for issue in consistency_issues(track, evidence)
        .into_iter()
        .filter(|issue| issue.blocking)
    {
        missing_by_step
            .entry(issue.step_id)
            .or_default()
            .push(issue.message);
    }

    for requirement in &config.requirements {
        if !requirement.required || !condition_applies(&requirement.when, track) {
            continue;
        }
        if !requirement_met(
            requirement,
            track,
            profile,
            evidence,
            &evidence_roles,
            deviations,
        ) {
            missing_by_step
                .entry(requirement.step_id.clone())
                .or_default()
                .push(requirement.missing_message.clone());
        }
    }
    missing_by_step
}

fn evaluated_steps(
    config: &WorkflowConfig,
    track: &TrackRecord,
    missing_by_step: &HashMap<String, Vec<String>>,
    stored_steps: &[StepState],
) -> Vec<StepState> {
    let stored: HashMap<&str, &StepState> = stored_steps
        .iter()
        .map(|state| (state.id.as_str(), state))
        .collect();
    let now = Utc::now().to_rfc3339();
    let mut steps = Vec::new();
    for step in &config.steps {
        let missing = missing_by_step
            .get(&step.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let step_applies = step
            .when
            .as_deref()
            .map(|condition| condition_applies(condition, track))
            .unwrap_or(true);
        let applicable = step_applies
            && config.requirements.iter().any(|requirement| {
                requirement.step_id == step.id
                    && requirement.required
                    && condition_applies(&requirement.when, track)
            });
        let preceding_step_blocked = step.id == "finalize"
            && (missing_by_step
                .iter()
                .any(|(step_id, items)| step_id != "finalize" && !items.is_empty())
                || steps.iter().any(|state: &StepState| {
                    matches!(
                        state.status,
                        StepStatus::Fail | StepStatus::Blocked | StepStatus::NotVerified
                    )
                }));
        let state = if let Some(stored) = stored.get(step.id.as_str()) {
            let justified_na = stored.status == StepStatus::NotApplicable
                && !applicable
                && stored
                    .na_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty());
            let explicitly_blocking =
                matches!(stored.status, StepStatus::Fail | StepStatus::Blocked)
                    || (stored.status == StepStatus::NotVerified
                        && (!missing.is_empty() || preceding_step_blocked));
            if justified_na || explicitly_blocking {
                (*stored).clone()
            } else if missing.is_empty() && !preceding_step_blocked {
                StepState {
                    id: step.id.clone(),
                    status: StepStatus::Pass,
                    na_reason: None,
                    updated_at: Some(now.clone()),
                }
            } else {
                StepState {
                    id: step.id.clone(),
                    status: StepStatus::Blocked,
                    na_reason: None,
                    updated_at: Some(now.clone()),
                }
            }
        } else {
            StepState {
                id: step.id.clone(),
                status: if missing.is_empty() && !preceding_step_blocked {
                    StepStatus::Pass
                } else if track.legacy {
                    StepStatus::NotVerified
                } else {
                    StepStatus::Blocked
                },
                na_reason: None,
                updated_at: Some(now.clone()),
            }
        };
        steps.push(state);
    }
    steps
}
pub fn can_mark_na(step_id: &str, track: &TrackRecord) -> Result<bool> {
    let config = config()?;
    let step = config.steps.iter().find(|step| step.id == step_id);
    if step.is_none() {
        return Err(AppError::Validation(format!(
            "Unknown workflow step: {step_id}"
        )));
    }
    let step_applies = step
        .and_then(|value| value.when.as_deref())
        .map(|condition| condition_applies(condition, track))
        .unwrap_or(true);
    Ok(!step_applies
        || !config.requirements.iter().any(|requirement| {
            requirement.step_id == step_id
                && requirement.required
                && condition_applies(&requirement.when, track)
        }))
}

pub fn progress(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
) -> Result<u8> {
    let config = config()?;
    let evidence_roles: HashSet<&str> = evidence
        .iter()
        .filter(|item| item.verified && item.sha256.is_some() && item.verification_error.is_none())
        .map(|item| item.role.as_str())
        .collect();
    let applicable = config
        .requirements
        .iter()
        .filter(|requirement| requirement.required && condition_applies(&requirement.when, track))
        .collect::<Vec<_>>();
    let unverified_evidence = evidence
        .iter()
        .filter(|item| !item.verified || item.sha256.is_none() || item.verification_error.is_some())
        .count();
    let consistency_issue_count = consistency_issues(track, evidence)
        .into_iter()
        .filter(|issue| issue.blocking)
        .count();
    if applicable.is_empty() && unverified_evidence == 0 && consistency_issue_count == 0 {
        return Ok(0);
    }
    let completed = applicable
        .iter()
        .filter(|requirement| {
            requirement_met(
                requirement,
                track,
                profile,
                evidence,
                &evidence_roles,
                deviations,
            )
        })
        .count();
    Ok(
        ((completed * 100) / (applicable.len() + unverified_evidence + consistency_issue_count))
            .min(100) as u8,
    )
}

pub(super) fn condition_applies(condition: &str, track: &TrackRecord) -> bool {
    source_condition(condition, track)
        .or_else(|| content_condition(condition, track))
        .or_else(|| artwork_condition(condition, track))
        .or_else(|| finalization_condition(condition, track))
        .unwrap_or(true)
}

fn source_condition(condition: &str, track: &TrackRecord) -> Option<bool> {
    let fields = &track.fields;
    match condition {
        "always" => Some(true),
        "external_audio" => Some(fields.external_audio_uploaded == Some(true)),
        "own_audio" => Some(fields.own_audio_uploaded == Some(true)),
        "code_based_generation" => Some(fields.code_based_generation == Some(true)),
        "code_audio_post_processed" => Some(
            fields.code_based_generation == Some(true)
                && fields.code_audio_post_processed == Some(true),
        ),
        "third_party_samples" => Some(fields.third_party_samples_uploaded == Some(true)),
        _ => None,
    }
}

fn content_condition(condition: &str, track: &TrackRecord) -> Option<bool> {
    let fields = &track.fields;
    match condition {
        "suno_content_non_empty" => Some(
            fields
                .suno_content_classification
                .is_some_and(|value| value != SunoContentClassification::Empty),
        ),
        "suno_content_other" => {
            Some(fields.suno_content_classification == Some(SunoContentClassification::Other))
        }
        "human_editing" => Some(fields.human_editing_performed == Some(true)),
        "post_export_editing" => Some(fields.post_export_editing_performed == Some(true)),
        _ => None,
    }
}

fn artwork_condition(condition: &str, track: &TrackRecord) -> Option<bool> {
    let fields = &track.fields;
    match condition {
        "artwork_present" => Some(!matches!(fields.artwork_origin.as_str(), "" | "none")),
        "ai_artwork" => Some(matches!(
            fields.artwork_origin.as_str(),
            "ai_generated" | "ai_assisted"
        )),
        "ai_assisted_artwork" => Some(fields.artwork_origin == "ai_assisted"),
        "ai_transparency_required" => Some(
            matches!(
                fields.artwork_origin.as_str(),
                "ai_generated" | "ai_assisted"
            ) && !content_check_all_negative(track),
        ),
        "real_person" => Some(fields.depicts_real_person == Some(true)),
        "real_event" => Some(fields.depicts_real_event == Some(true)),
        "trademark_or_logo" => Some(fields.contains_trademark == Some(true)),
        _ => None,
    }
}

fn finalization_condition(condition: &str, track: &TrackRecord) -> Option<bool> {
    let fields = &track.fields;
    match condition {
        "generative_ai_used" => Some(fields.generative_ai_used == Some(true)),
        "audio_disclosure_yes" => Some(
            fields.generative_ai_used == Some(true)
                && fields.audio_disclosure_applied == Some(crate::model::DocumentationAnswer::Yes),
        ),
        "commercial_use" => Some(fields.commercial_use_intended),
        "finalization_ready" => Some(true),
        _ => None,
    }
}
pub(super) fn requirement_met(
    requirement: &WorkflowRequirement,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    evidence_roles: &HashSet<&str>,
    deviations: &[BlockingDeviation],
) -> bool {
    match requirement.kind.as_str() {
        "evidence" => {
            evidence_requirement_met(requirement, track, profile, evidence, evidence_roles)
        }
        "generated_document" => track.documents.generated && track.documents.current,
        "hash_verification" => hash_verification_met(track),
        "audio_screening" => audio_screening_requirement_met(requirement, track, evidence),
        "field" => field_kind_requirement_met(requirement, track, profile, evidence, deviations),
        _ => false,
    }
}

fn evidence_requirement_met(
    requirement: &WorkflowRequirement,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    evidence_roles: &HashSet<&str>,
) -> bool {
    if requirement.key == "evidence_licenses.portable_copy" {
        return subscription_production_coverage(track, evidence) == CoverageStatus::Yes;
    }
    if requirement.key == "artwork.final" && disclosure_required(track, profile) {
        let disclosed_hashes = evidence
            .iter()
            .filter(|item| verified_local_disclosure(item, track, evidence))
            .filter_map(|item| item.sha256.as_deref())
            .collect::<HashSet<_>>();
        return evidence.iter().any(|item| {
            verified_role(item, EvidenceRole::FinalArtwork)
                && item
                    .sha256
                    .as_deref()
                    .is_some_and(|hash| disclosed_hashes.contains(hash))
        });
    }
    requirement
        .evidence_role
        .as_deref()
        .is_some_and(|role| evidence_roles.contains(role))
}

fn hash_verification_met(track: &TrackRecord) -> bool {
    track.integrity.generated
        && track.integrity.verified
        && track.integrity.file_count > 0
        && track.integrity.file_count == track.integrity.verified_count
        && track.integrity.mismatch_files.is_empty()
}

fn audio_screening_requirement_met(
    requirement: &WorkflowRequirement,
    track: &TrackRecord,
    evidence: &[EvidenceItem],
) -> bool {
    if requirement.key != "release.audio_screening_local" {
        return false;
    }
    let release = evidence
        .iter()
        .find(|item| verified_role(item, EvidenceRole::ReleaseWav));
    release.is_some_and(|release| {
        audio_screening::local_record_matches_source(
            &track.audio_screening.local,
            &track.id,
            release,
        )
    })
}

fn field_kind_requirement_met(
    requirement: &WorkflowRequirement,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
) -> bool {
    match requirement.key.as_str() {
        "ai_transparency.disclosure_result" => {
            let disclosed_artwork_present = evidence
                .iter()
                .any(|item| verified_local_disclosure(item, track, evidence));
            match track.fields.disclosure_applied {
                Some(true) => present(&track.fields.disclosure_text) && disclosed_artwork_present,
                Some(false) => true,
                None => false,
            }
        }
        "release.filename_consistency" => filename_requirement_met(
            track,
            evidence,
            EvidenceRole::ReleaseWav,
            track.fields.release_filename_difference_confirmed,
        ),
        "suno.export_filename_consistency" => filename_requirement_met(
            track,
            evidence,
            EvidenceRole::SunoFinalExport,
            track.fields.suno_export_filename_difference_confirmed,
        ),
        "evidence_licenses.subscription_generation_coverage" => {
            subscription_generation_coverage(track, evidence) == CoverageStatus::Yes
        }
        "evidence_licenses.terms_complete" => {
            track.fields.suno_terms_evidence_not_available != Some(true)
                && evidence.iter().any(|item| {
                    verified_role(item, EvidenceRole::SunoTermsRights) && terms_complete(item)
                })
        }
        _ => field_requirement_met(&requirement.key, track, profile, deviations),
    }
}
pub(super) fn verified_role(item: &EvidenceItem, role: EvidenceRole) -> bool {
    item.role == role && item.verified && item.sha256.is_some() && item.verification_error.is_none()
}

pub(super) fn terms_complete(item: &EvidenceItem) -> bool {
    present(&item.metadata.document_title)
        && present(&item.metadata.provider)
        && NaiveDate::parse_from_str(item.metadata.retrieval_date.trim(), "%Y-%m-%d").is_ok()
}

fn verified_local_disclosure(
    item: &EvidenceItem,
    track: &TrackRecord,
    evidence: &[EvidenceItem],
) -> bool {
    verified_role(item, EvidenceRole::AiArtworkEdited)
        && item.provenance == crate::model::EvidenceProvenance::GeneratedDisclosure
        && item.generator_version.as_deref() == Some(crate::artwork::DISCLOSURE_GENERATOR_VERSION)
        && item.generated_disclosure_text.as_deref() == Some(track.fields.disclosure_text.trim())
        && item
            .derived_from_evidence_id
            .as_deref()
            .is_some_and(|source_id| {
                source_id != item.id
                    && evidence.iter().any(|source| {
                        source.id == source_id
                            && verified_role(source, EvidenceRole::AiArtworkOriginal)
                    })
            })
}

pub(super) fn disclosure_required(track: &TrackRecord, _profile: &Profile) -> bool {
    matches!(
        track.fields.artwork_origin.as_str(),
        "ai_generated" | "ai_assisted"
    ) && track.fields.disclosure_applied == Some(true)
}

pub(super) fn content_check_all_negative(track: &TrackRecord) -> bool {
    track.fields.depicts_real_person == Some(false)
        && track.fields.depicts_real_event == Some(false)
        && track.fields.contains_trademark == Some(false)
}

pub(super) fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(super) fn filename_matches_documented_title(title: &str, file_name: &str) -> bool {
    let stem = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name);
    normalize_filename_identity(title) == normalize_filename_identity(stem)
}

fn normalize_filename_identity(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(super) fn filename_requirement_met(
    track: &TrackRecord,
    evidence: &[EvidenceItem],
    role: EvidenceRole,
    confirmed: Option<bool>,
) -> bool {
    original_evidence_file_name(evidence, role).is_some_and(|file_name| {
        filename_matches_documented_title(&track.fields.title, file_name) || confirmed == Some(true)
    })
}

pub(super) fn field_requirement_met(
    key: &str,
    track: &TrackRecord,
    profile: &Profile,
    deviations: &[BlockingDeviation],
) -> bool {
    if key.starts_with("track.") || key.starts_with("profile.") {
        return track_profile_requirement_met(key, track, profile);
    }
    if key.starts_with("source.") {
        return source_requirement_met(key, track);
    }
    if key.starts_with("suno.") {
        return suno_requirement_met(key, track);
    }
    if key.starts_with("human_work.") {
        return human_work_requirement_met(key, track);
    }
    if key.starts_with("artwork.") {
        return artwork_requirement_met(key, track);
    }
    if key.starts_with("ai_transparency.") {
        return ai_transparency_requirement_met(key, track, profile);
    }
    if key == "finalize.blocking_deviations_resolved" {
        return deviations
            .iter()
            .all(|deviation| !deviation.blocking || deviation.resolved);
    }
    false
}

fn track_profile_requirement_met(key: &str, track: &TrackRecord, profile: &Profile) -> bool {
    let fields = &track.fields;
    match key {
        "track.title" => present(&fields.title),
        "track.production_start" => present(&fields.production_start_date),
        "track.production_end" => present(&fields.production_end_date),
        "track.commercial_use_intended" => true,
        "profile.artist_name" => present(&profile.artist_name),
        "profile.suno_profile_name" => present(&profile.suno_profile_name),
        "profile.suno_handle" => present(&profile.suno_handle),
        "profile.suno_plan" => present(&profile.suno_plan),
        "profile.subscription_start_date" => present(&profile.subscription_start_date),
        "profile.default_ai_image_service" => present(&profile.default_ai_image_service),
        "profile.artwork_transparency_policy" => matches!(
            profile.artwork_transparency_policy.as_str(),
            "always" | "per_artwork" | "none"
        ),
        _ => false,
    }
}

fn source_requirement_met(key: &str, track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match key {
        "source.external_audio_uploaded" => fields.external_audio_uploaded.is_some(),
        "source.external_audio_details" => {
            present(&fields.external_audio_source) && present(&fields.external_audio_ownership)
        }
        "source.own_audio_uploaded" => fields.own_audio_uploaded.is_some(),
        "source.own_audio_details" => {
            present(&fields.own_audio_source) && present(&fields.own_audio_ownership)
        }
        "source.code_based_generation" => fields.code_based_generation.is_some(),
        "source.code_audio_post_processed" => fields.code_audio_post_processed.is_some(),
        "source.code_audio_post_processing_operations" => fields
            .code_audio_post_processing_operations
            .iter()
            .any(|value| present(value)),
        "source.third_party_samples_uploaded" => fields.third_party_samples_uploaded.is_some(),
        "source.third_party_sample_details" => {
            present(&fields.third_party_sample_source)
                && present(&fields.third_party_sample_ownership)
        }
        _ => false,
    }
}

fn suno_requirement_met(key: &str, track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match key {
        "suno.model" => present(&fields.suno_model),
        "suno.project_url" => present(&fields.suno_project_url),
        "suno.final_generation_date" => present(&fields.suno_final_generation_date),
        "suno.download_export_date" => present(&fields.suno_download_export_date),
        "suno.plan_at_generation" => present(&fields.suno_plan_at_generation),
        "suno.final_export_date" => present(&fields.final_export_date),
        _ => false,
    }
}

fn human_work_requirement_met(key: &str, track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match key {
        "human_work.instrumental_answer" => fields.instrumental_track.is_some(),
        "human_work.vocal_lyrics_present" => fields.vocal_lyrics_present.is_some(),
        "human_work.vocal_intent" => fields.vocal_intent.is_some(),
        "human_work.suno_content_classification" => fields.suno_content_classification.is_some(),
        "human_work.suno_lyrics_content_source" => fields.suno_lyrics_content_source.is_some(),
        "human_work.suno_lyrics_field_text" => present(&fields.suno_lyrics_field_text),
        "human_work.suno_lyrics_other_content_type" => {
            present(&fields.suno_lyrics_other_content_type)
        }
        "human_work.suno_style_prompt" => present(&fields.suno_style_prompt),
        "human_work.human_editing_performed" => fields.human_editing_performed.is_some(),
        "human_work.human_editing_details" => present(&fields.human_editing_details),
        "human_work.post_export_editing_performed" => {
            fields.post_export_editing_performed.is_some()
        }
        "human_work.post_export_editing_details" => present(&fields.post_export_editing_details),
        _ => false,
    }
}

fn artwork_requirement_met(key: &str, track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match key {
        "artwork.origin" => present(&fields.artwork_origin),
        "artwork.human_modifications" => fields
            .human_artwork_modifications
            .iter()
            .any(|value| present(value)),
        "artwork.real_person" => fields.depicts_real_person.is_some(),
        "artwork.real_person_note" => present(&fields.real_person_notes),
        "artwork.real_event" => fields.depicts_real_event.is_some(),
        "artwork.real_event_note" => present(&fields.real_event_notes),
        "artwork.trademark_or_logo" => fields.contains_trademark.is_some(),
        "artwork.trademark_or_logo_note" => present(&fields.trademark_notes),
        _ => false,
    }
}

fn ai_transparency_requirement_met(key: &str, track: &TrackRecord, profile: &Profile) -> bool {
    let fields = &track.fields;
    match key {
        "ai_transparency.image_service" => present(&fields.ai_image_service),
        "ai_transparency.policy" => present(&profile.artwork_transparency_policy),
        "ai_transparency.generative_ai_used" => fields.generative_ai_used.is_some(),
        "ai_transparency.audio_ai_system" => present(&fields.audio_ai_system),
        "ai_transparency.ai_assisted_audio_elements" => fields.ai_assisted_audio_elements.is_some(),
        "ai_transparency.ai_generated_audio_elements" => {
            fields.ai_generated_audio_elements.is_some()
        }
        "ai_transparency.real_person_voice_intentionally_imitated" => {
            fields.real_person_voice_intentionally_imitated.is_some()
        }
        "ai_transparency.real_person_identity_intentionally_represented" => fields
            .real_person_identity_intentionally_represented
            .is_some(),
        "ai_transparency.real_event_represented_as_authentic_recording" => fields
            .real_event_represented_as_authentic_recording
            .is_some(),
        "ai_transparency.real_location_institution_event_presented_as_authentic_ai_recording" => {
            fields
                .real_location_institution_event_presented_as_authentic_ai_recording
                .is_some()
        }
        "ai_transparency.audio_disclosure_decision" => {
            fields.audio_disclosure_applied.is_some()
                && !(fields.commercial_use_intended
                    && fields.generative_ai_used == Some(true)
                    && fields.audio_disclosure_applied
                        == Some(crate::model::DocumentationAnswer::NotDocumented))
        }
        "ai_transparency.audio_disclosure_locations" => fields
            .audio_disclosure_locations
            .iter()
            .any(|value| present(value)),
        "ai_transparency.audio_disclosure_text" => present(&fields.audio_disclosure_text),
        _ => false,
    }
}
pub fn evidence_role_from_str(value: &str) -> Result<EvidenceRole> {
    serde_json::from_str(&format!("\"{}\"", value.replace('"', "")))
        .map_err(|_| AppError::Validation(format!("Unknown evidence role: {value}")))
}
