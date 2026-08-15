use crate::error::{AppError, Result};
use crate::model::{
    BlockingDeviation, EvidenceItem, EvidenceRole, Profile, StepState, StepStatus, TrackRecord,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const WORKFLOW_SOURCE: &str = include_str!("../../workflows/suno-track.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    pub blocker_statuses: Vec<String>,
    pub steps: Vec<WorkflowStep>,
    pub requirements: Vec<WorkflowRequirement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub order: u32,
    pub name: String,
    pub required: bool,
    pub when: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowRequirement {
    pub key: String,
    pub step_id: String,
    pub kind: String,
    pub required: bool,
    pub when: String,
    pub missing_message: String,
    pub evidence_role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    pub steps: Vec<WorkflowStepDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStepDto {
    pub id: String,
    pub number: String,
    pub label: String,
    pub title: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowEvaluation {
    pub steps: Vec<StepState>,
    pub missing: Vec<String>,
}

pub fn config() -> Result<WorkflowConfig> {
    let config: WorkflowConfig = toml::from_str(WORKFLOW_SOURCE)
        .map_err(|e| AppError::Data(format!("Invalid embedded workflow: {e}")))?;
    validate_config(&config)?;
    Ok(config)
}

#[cfg(test)]
pub fn config_with_version_for_test(version: &str) -> Result<WorkflowConfig> {
    let mut config = config()?;
    config.version = version.to_owned();
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &WorkflowConfig) -> Result<()> {
    if config.schema_version != 1 || config.id.trim().is_empty() || config.version.trim().is_empty()
    {
        return Err(AppError::Data("Unsupported workflow metadata.".into()));
    }
    let declared_blockers = config
        .blocker_statuses
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let expected_blockers = HashSet::from(["FAIL", "BLOCKED", "NOT_VERIFIED"]);
    if declared_blockers != expected_blockers {
        return Err(AppError::Data(
            "Workflow blocker_statuses must be FAIL, BLOCKED, and NOT_VERIFIED.".into(),
        ));
    }
    let allowed_conditions: HashSet<&str> = [
        "always",
        "external_audio",
        "own_audio",
        "code_based_generation",
        "third_party_samples",
        "lyrics_text",
        "human_editing",
        "post_export_editing",
        "artwork_present",
        "ai_artwork",
        "ai_transparency_required",
        "real_person",
        "real_event",
        "trademark_or_logo",
        "commercial_use",
        "finalization_ready",
    ]
    .into_iter()
    .collect();
    let allowed_kinds: HashSet<&str> = [
        "field",
        "evidence",
        "generated_document",
        "hash_verification",
    ]
    .into_iter()
    .collect();
    let mut ids = HashSet::new();
    let mut orders = HashSet::new();
    for step in &config.steps {
        if step.id.trim().is_empty() {
            return Err(AppError::Data("Workflow step id must not be empty.".into()));
        }
        if !ids.insert(step.id.as_str()) {
            return Err(AppError::Data(format!(
                "Duplicate workflow step: {}",
                step.id
            )));
        }
        if !(1..=10).contains(&step.order) || !orders.insert(step.order) {
            return Err(AppError::Data(format!(
                "Invalid workflow step order: {}",
                step.order
            )));
        }
        if !step.required {
            return Err(AppError::Data(format!(
                "Workflow step {} must be required.",
                step.id
            )));
        }
        if let Some(condition) = step.when.as_deref() {
            if !allowed_conditions.contains(condition) {
                return Err(AppError::Data(format!(
                    "Unknown workflow condition: {condition}"
                )));
            }
        }
    }
    if config.steps.len() != 10 {
        return Err(AppError::Data(
            "The Suno workflow must contain exactly ten steps.".into(),
        ));
    }
    let mut requirement_keys = HashSet::new();
    for requirement in &config.requirements {
        if !requirement_keys.insert(requirement.key.as_str()) {
            return Err(AppError::Data(format!(
                "Duplicate workflow requirement: {}",
                requirement.key
            )));
        }
        if !ids.contains(requirement.step_id.as_str()) {
            return Err(AppError::Data(format!(
                "Requirement {} uses an unknown step.",
                requirement.key
            )));
        }
        if !allowed_kinds.contains(requirement.kind.as_str()) {
            return Err(AppError::Data(format!(
                "Unknown workflow requirement kind: {}",
                requirement.kind
            )));
        }
        if !allowed_conditions.contains(requirement.when.as_str()) {
            return Err(AppError::Data(format!(
                "Unknown workflow condition: {}",
                requirement.when
            )));
        }
        if requirement.kind == "evidence" {
            let role = requirement.evidence_role.as_deref().ok_or_else(|| {
                AppError::Data(format!(
                    "Evidence requirement {} has no role.",
                    requirement.key
                ))
            })?;
            evidence_role_from_str(role)?;
        } else if requirement.evidence_role.is_some() {
            return Err(AppError::Data(format!(
                "Non-evidence requirement {} declares an evidence role.",
                requirement.key
            )));
        }
    }
    Ok(())
}

pub fn definition() -> Result<WorkflowDefinition> {
    let config = config()?;
    Ok(WorkflowDefinition {
        schema_version: config.schema_version,
        id: config.id,
        version: config.version,
        name: config.name,
        steps: config
            .steps
            .into_iter()
            .map(|step| {
                let title = step
                    .name
                    .split_once(' ')
                    .map(|(_, title)| title)
                    .unwrap_or(&step.name)
                    .to_owned();
                WorkflowStepDto {
                    id: step.id,
                    number: format!("{:02}", step.order),
                    label: step.name,
                    title,
                    description: step.description,
                    required: step.required,
                }
            })
            .collect(),
    })
}

pub fn evaluate(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    stored_steps: &[StepState],
) -> Result<WorkflowEvaluation> {
    let config = config()?;
    let mut missing_by_step: HashMap<String, Vec<String>> = HashMap::new();
    let evidence_roles: HashSet<&str> = evidence
        .iter()
        .filter(|e| e.verified && e.sha256.is_some() && e.verification_error.is_none())
        .map(|e| e.role.as_str())
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
                    .is_some_and(|r| !r.trim().is_empty());
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

    let missing = config
        .steps
        .iter()
        .flat_map(|step| missing_by_step.get(&step.id).cloned().unwrap_or_default())
        .collect();
    Ok(WorkflowEvaluation { steps, missing })
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
    if applicable.is_empty() && unverified_evidence == 0 {
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
    Ok(((completed * 100) / (applicable.len() + unverified_evidence)).min(100) as u8)
}

fn condition_applies(condition: &str, track: &TrackRecord) -> bool {
    let f = &track.fields;
    match condition {
        "always" => true,
        "external_audio" => f.external_audio_uploaded == Some(true),
        "own_audio" => f.own_audio_uploaded == Some(true),
        "code_based_generation" => f.code_based_generation == Some(true),
        "third_party_samples" => f.third_party_samples_uploaded == Some(true),
        "lyrics_text" => !matches!(f.lyrics_source.as_str(), "" | "instrumental"),
        "human_editing" => f.human_editing_performed == Some(true),
        "post_export_editing" => f.post_export_editing_performed == Some(true),
        "artwork_present" => !matches!(f.artwork_origin.as_str(), "" | "none"),
        "ai_artwork" => matches!(f.artwork_origin.as_str(), "ai_generated" | "ai_assisted"),
        "ai_transparency_required" => {
            matches!(f.artwork_origin.as_str(), "ai_generated" | "ai_assisted")
                && !content_check_all_negative(track)
        }
        "real_person" => f.depicts_real_person == Some(true),
        "real_event" => f.depicts_real_event == Some(true),
        "trademark_or_logo" => f.contains_trademark == Some(true),
        "commercial_use" => f.commercial_use_intended,
        "finalization_ready" => true,
        _ => true, // Unknown conditions fail closed by making their requirement applicable.
    }
}

fn requirement_met(
    requirement: &WorkflowRequirement,
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    evidence_roles: &HashSet<&str>,
    deviations: &[BlockingDeviation],
) -> bool {
    match requirement.kind.as_str() {
        "evidence" if requirement.key == "evidence_licenses.portable_copy" => {
            evidence.iter().any(|item| {
                item.role == EvidenceRole::SubscriptionPayment
                    && item.verified
                    && item.sha256.is_some()
                    && item.verification_error.is_none()
                    && item.source_global_evidence_id.is_some()
                    && item
                        .coverage_start
                        .as_deref()
                        .is_some_and(|start| start <= track.fields.production_start_date.as_str())
                    && item
                        .coverage_end
                        .as_deref()
                        .is_some_and(|end| end >= track.fields.production_end_date.as_str())
            })
        }
        "evidence" if requirement.key == "artwork.final" && disclosure_required(track, profile) => {
            let disclosed_hashes = evidence
                .iter()
                .filter(|item| verified_local_disclosure(item, track, evidence))
                .filter_map(|item| item.sha256.as_deref())
                .collect::<HashSet<_>>();
            evidence.iter().any(|item| {
                verified_role(item, EvidenceRole::FinalArtwork)
                    && item
                        .sha256
                        .as_deref()
                        .is_some_and(|hash| disclosed_hashes.contains(hash))
            })
        }
        "evidence" => requirement
            .evidence_role
            .as_deref()
            .is_some_and(|role| evidence_roles.contains(role)),
        "generated_document" => track.documents.generated && track.documents.current,
        "hash_verification" => {
            track.integrity.generated
                && track.integrity.verified
                && track.integrity.file_count > 0
                && track.integrity.file_count == track.integrity.verified_count
                && track.integrity.mismatch_files.is_empty()
        }
        "field" if requirement.key == "ai_transparency.disclosure_result" => {
            let disclosed_artwork_present = evidence
                .iter()
                .any(|item| verified_local_disclosure(item, track, evidence));
            match profile.artwork_transparency_policy.as_str() {
                "always" => {
                    track.fields.disclosure_applied == Some(true)
                        && present(&track.fields.disclosure_text)
                        && disclosed_artwork_present
                }
                "per_artwork" => match track.fields.disclosure_applied {
                    Some(true) => {
                        present(&track.fields.disclosure_text) && disclosed_artwork_present
                    }
                    Some(false) => true,
                    None => false,
                },
                "none" => true,
                _ => false,
            }
        }
        "field" => field_requirement_met(&requirement.key, track, profile, deviations),
        _ => false,
    }
}

fn verified_role(item: &EvidenceItem, role: EvidenceRole) -> bool {
    item.role == role && item.verified && item.sha256.is_some() && item.verification_error.is_none()
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

fn disclosure_required(track: &TrackRecord, profile: &Profile) -> bool {
    matches!(
        track.fields.artwork_origin.as_str(),
        "ai_generated" | "ai_assisted"
    ) && !content_check_all_negative(track)
        && (profile.artwork_transparency_policy == "always"
            || (profile.artwork_transparency_policy == "per_artwork"
                && track.fields.disclosure_applied == Some(true)))
}

fn content_check_all_negative(track: &TrackRecord) -> bool {
    track.fields.depicts_real_person == Some(false)
        && track.fields.depicts_real_event == Some(false)
        && track.fields.contains_trademark == Some(false)
}

fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

fn field_requirement_met(
    key: &str,
    track: &TrackRecord,
    profile: &Profile,
    deviations: &[BlockingDeviation],
) -> bool {
    let f = &track.fields;
    match key {
        "track.title" => present(&f.title),
        "track.production_start" => present(&f.production_start_date),
        "track.production_end" => present(&f.production_end_date),
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
        "source.external_audio_uploaded" => f.external_audio_uploaded.is_some(),
        "source.external_audio_details" => {
            present(&f.external_audio_source) && present(&f.external_audio_ownership)
        }
        "source.own_audio_uploaded" => f.own_audio_uploaded.is_some(),
        "source.own_audio_details" => {
            present(&f.own_audio_source) && present(&f.own_audio_ownership)
        }
        "source.code_based_generation" => f.code_based_generation.is_some(),
        "source.third_party_samples_uploaded" => f.third_party_samples_uploaded.is_some(),
        "source.third_party_sample_details" => {
            present(&f.third_party_sample_source) && present(&f.third_party_sample_ownership)
        }
        "suno.model" => present(&f.suno_model),
        "suno.project_url" => present(&f.suno_project_url),
        "suno.plan_at_creation" => present(&f.suno_plan_at_creation),
        "suno.final_export_date" => present(&f.final_export_date),
        "human_work.lyrics_source" => present(&f.lyrics_source),
        "human_work.human_lyrics_details" => present(&f.lyrics_text),
        "human_work.suno_style_prompt" => present(&f.suno_style_prompt),
        "human_work.human_editing_performed" => f.human_editing_performed.is_some(),
        "human_work.human_editing_details" => present(&f.human_editing_details),
        "human_work.post_export_editing_performed" => f.post_export_editing_performed.is_some(),
        "human_work.post_export_editing_details" => present(&f.post_export_editing_details),
        "artwork.origin" => present(&f.artwork_origin),
        "artwork.real_person" => f.depicts_real_person.is_some(),
        "artwork.real_person_note" => present(&f.real_person_notes),
        "artwork.real_event" => f.depicts_real_event.is_some(),
        "artwork.real_event_note" => present(&f.real_event_notes),
        "artwork.trademark_or_logo" => f.contains_trademark.is_some(),
        "artwork.trademark_or_logo_note" => present(&f.trademark_notes),
        "ai_transparency.image_service" => present(&f.ai_image_service),
        "ai_transparency.policy" => present(&profile.artwork_transparency_policy),
        "finalize.blocking_deviations_resolved" => deviations
            .iter()
            .all(|deviation| !deviation.blocking || deviation.resolved),
        _ => false,
    }
}

pub fn evidence_role_from_str(value: &str) -> Result<EvidenceRole> {
    serde_json::from_str(&format!("\"{}\"", value.replace('"', "")))
        .map_err(|_| AppError::Validation(format!("Unknown evidence role: {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disclosure_track(origin: &str, applied: Option<bool>) -> TrackRecord {
        TrackRecord {
            id: "track".into(),
            relative_path: "track".into(),
            status: crate::model::TrackStatus::Active,
            workflow_id: "suno-track".into(),
            workflow_version: "1.0".into(),
            profile_snapshot: Profile::default(),
            library: Default::default(),
            fields: crate::model::TrackFields {
                artwork_origin: origin.into(),
                disclosure_applied: applied,
                ..Default::default()
            },
            documents: Default::default(),
            integrity: Default::default(),
            certificate: Default::default(),
            created_at: "2026-08-13T00:00:00Z".into(),
            updated_at: "2026-08-13T00:00:00Z".into(),
            legacy: false,
        }
    }

    fn verified_evidence(role: EvidenceRole) -> EvidenceItem {
        EvidenceItem {
            id: role.as_str().into(),
            role,
            file_name: "fixture.png".into(),
            relative_path: "05_ARTWORK/fixture.png".into(),
            sha256: Some("a".repeat(64)),
            size_bytes: 42,
            imported_at: "2026-08-15T00:00:00Z".into(),
            verified: true,
            verification_error: None,
            source_global_evidence_id: None,
            coverage_start: None,
            coverage_end: None,
            provenance: crate::model::EvidenceProvenance::ManagedCopy,
            derived_from_evidence_id: None,
            generator_version: None,
            generated_disclosure_text: None,
        }
    }

    fn embedded_config() -> WorkflowConfig {
        toml::from_str(WORKFLOW_SOURCE).expect("embedded workflow must deserialize")
    }

    fn assert_data_error(config: &WorkflowConfig, expected: &str) {
        match validate_config(config) {
            Err(AppError::Data(message)) => assert_eq!(message, expected),
            Err(error) => panic!("expected data error {expected:?}, got {error:?}"),
            Ok(()) => panic!("expected data error {expected:?}, validation succeeded"),
        }
    }

    fn workflow_value() -> toml::Value {
        toml::from_str(WORKFLOW_SOURCE).expect("embedded workflow TOML value")
    }

    #[test]
    fn valid_version_1_1_configuration_is_accepted() {
        let config = embedded_config();

        validate_config(&config).expect("valid workflow 1.1");

        assert_eq!(config.schema_version, 1);
        assert_eq!(config.id, "suno-track");
        assert_eq!(config.version, "1.1");
        assert_eq!(config.steps.len(), 10);
        assert_eq!(config.steps.first().map(|step| step.order), Some(1));
        assert_eq!(config.steps.last().map(|step| step.order), Some(10));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut config = embedded_config();
        config.schema_version = 2;

        assert_data_error(&config, "Unsupported workflow metadata.");
    }

    #[test]
    fn empty_step_id_is_rejected_even_when_requirements_use_the_same_id() {
        for empty_id in ["", "   "] {
            let mut config = embedded_config();
            let original_id = config.steps[0].id.clone();
            config.steps[0].id = empty_id.into();
            for requirement in &mut config.requirements {
                if requirement.step_id == original_id {
                    requirement.step_id = empty_id.into();
                }
            }

            assert_data_error(&config, "Workflow step id must not be empty.");
        }
    }

    #[test]
    fn duplicate_step_ids_are_rejected() {
        let mut config = embedded_config();
        let duplicate = config.steps[0].id.clone();
        config.steps[1].id = duplicate.clone();

        assert_data_error(&config, &format!("Duplicate workflow step: {duplicate}"));
    }

    #[test]
    fn unknown_requirement_kind_is_rejected() {
        let mut config = embedded_config();
        config.requirements[0].kind = "arbitrary_step_type".into();

        assert_data_error(
            &config,
            "Unknown workflow requirement kind: arbitrary_step_type",
        );
    }

    #[test]
    fn missing_required_toml_fields_fail_deserialization() {
        let mut missing_schema = workflow_value();
        missing_schema
            .as_table_mut()
            .expect("workflow table")
            .remove("schema_version");

        let mut missing_step_required = workflow_value();
        missing_step_required
            .get_mut("steps")
            .and_then(toml::Value::as_array_mut)
            .and_then(|steps| steps.first_mut())
            .and_then(toml::Value::as_table_mut)
            .expect("first workflow step")
            .remove("required");

        let mut missing_requirement_message = workflow_value();
        missing_requirement_message
            .get_mut("requirements")
            .and_then(toml::Value::as_array_mut)
            .and_then(|requirements| requirements.first_mut())
            .and_then(toml::Value::as_table_mut)
            .expect("first workflow requirement")
            .remove("missing_message");

        for (case, value, field) in [
            ("workflow metadata", missing_schema, "schema_version"),
            ("workflow step", missing_step_required, "required"),
            (
                "workflow requirement",
                missing_requirement_message,
                "missing_message",
            ),
        ] {
            let source = toml::to_string(&value).expect("serialize malformed workflow fixture");
            let error = match toml::from_str::<WorkflowConfig>(&source) {
                Err(error) => error,
                Ok(_) => panic!("{case} unexpectedly deserialized"),
            };
            assert!(
                error
                    .to_string()
                    .contains(&format!("missing field `{field}`")),
                "{case} reported an unexpected error: {error}"
            );
        }
    }

    #[test]
    fn missing_mandatory_step_is_rejected() {
        let mut config = embedded_config();
        config.steps.pop();

        assert_data_error(&config, "The Suno workflow must contain exactly ten steps.");
    }

    #[test]
    fn disclosure_requirement_matrix_matches_origin_policy_and_track_decision() {
        for origin in ["ai_generated", "ai_assisted", "human", "none"] {
            for policy in ["always", "per_artwork", "none"] {
                for applied in [None, Some(false), Some(true)] {
                    let track = disclosure_track(origin, applied);
                    let profile = Profile {
                        artwork_transparency_policy: policy.into(),
                        ..Default::default()
                    };
                    let expected = matches!(origin, "ai_generated" | "ai_assisted")
                        && (policy == "always"
                            || (policy == "per_artwork" && applied == Some(true)));
                    assert_eq!(
                        disclosure_required(&track, &profile),
                        expected,
                        "origin={origin}, policy={policy}, applied={applied:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn three_negative_content_checks_disable_ai_transparency() {
        let mut track = disclosure_track("ai_assisted", Some(true));
        track.fields.depicts_real_person = Some(false);
        track.fields.depicts_real_event = Some(false);
        track.fields.contains_trademark = Some(false);
        let profile = Profile {
            artwork_transparency_policy: "always".into(),
            ..Default::default()
        };

        assert!(content_check_all_negative(&track));
        assert!(!condition_applies("ai_transparency_required", &track));
        assert!(!disclosure_required(&track, &profile));

        let evidence = vec![
            verified_evidence(EvidenceRole::AiArtworkOriginal),
            verified_evidence(EvidenceRole::FinalArtwork),
        ];
        let evaluation = evaluate(&track, &profile, &evidence, &[], &[])
            .expect("evaluate negative content checks");
        assert!(!evaluation
            .missing
            .iter()
            .any(|item| item.contains("AI-Kennzeichnung") || item.contains("finale Artwork")));
    }

    #[test]
    fn code_based_generation_requires_source_code_and_generated_audio_evidence() {
        let mut track = disclosure_track("none", None);
        let profile = Profile::default();

        let unanswered = evaluate(&track, &profile, &[], &[], &[])
            .expect("evaluate unanswered code-generation branch");
        assert!(unanswered
            .missing
            .iter()
            .any(|item| item.contains("codebasierten Erzeugung")));

        track.fields.code_based_generation = Some(false);
        let negative = evaluate(&track, &profile, &[], &[], &[])
            .expect("evaluate negative code-generation branch");
        assert!(!negative
            .missing
            .iter()
            .any(|item| item.contains("Quellcode oder die Quelldatei")));
        assert!(!negative
            .missing
            .iter()
            .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

        track.fields.code_based_generation = Some(true);
        let positive_without_file = evaluate(&track, &profile, &[], &[], &[])
            .expect("evaluate positive code-generation branch without evidence");
        assert!(positive_without_file
            .missing
            .iter()
            .any(|item| item.contains("Quellcode oder die Quelldatei")));
        assert!(positive_without_file
            .missing
            .iter()
            .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

        let source_only = vec![verified_evidence(EvidenceRole::SourceCodeFile)];
        let positive_with_source_only = evaluate(&track, &profile, &source_only, &[], &[])
            .expect("evaluate code-generation branch with source only");
        assert!(!positive_with_source_only
            .missing
            .iter()
            .any(|item| item.contains("Quellcode oder die Quelldatei")));
        assert!(positive_with_source_only
            .missing
            .iter()
            .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));

        let complete_evidence = vec![
            verified_evidence(EvidenceRole::SourceCodeFile),
            verified_evidence(EvidenceRole::CodeGeneratedAudioFile),
        ];
        let complete = evaluate(&track, &profile, &complete_evidence, &[], &[])
            .expect("evaluate complete code-generation branch");
        assert!(!complete
            .missing
            .iter()
            .any(|item| item.contains("Quellcode oder die Quelldatei")));
        assert!(!complete
            .missing
            .iter()
            .any(|item| item.contains("erzeugte WAV- oder MP3-Datei")));
    }

    #[test]
    fn finalize_is_blocked_while_a_preceding_step_is_incomplete() {
        let track = disclosure_track("none", None);
        let evaluation = evaluate(&track, &Profile::default(), &[], &[], &[])
            .expect("evaluate incomplete track");

        assert_eq!(
            evaluation
                .steps
                .iter()
                .find(|step| step.id == "finalize")
                .map(|step| &step.status),
            Some(&StepStatus::Blocked)
        );
    }

    #[test]
    fn fulfilled_legacy_step_recovers_from_stored_not_verified_status() {
        let mut track = disclosure_track("none", None);
        track.legacy = true;
        track.fields.title = "Recovered Track".into();
        track.fields.production_start_date = "2026-08-01".into();
        track.fields.production_end_date = "2026-08-02".into();
        let profile = Profile {
            artist_name: "Recovered Artist".into(),
            artwork_transparency_policy: "always".into(),
            ..Default::default()
        };
        let stored = StepState {
            id: "track".into(),
            status: StepStatus::NotVerified,
            na_reason: None,
            updated_at: Some("2026-08-01T00:00:00Z".into()),
        };

        let evaluation = evaluate(&track, &profile, &[], &[], &[stored])
            .expect("evaluate recovered legacy step");

        assert_eq!(
            evaluation
                .steps
                .iter()
                .find(|step| step.id == "track")
                .map(|step| &step.status),
            Some(&StepStatus::Pass)
        );
    }
}
