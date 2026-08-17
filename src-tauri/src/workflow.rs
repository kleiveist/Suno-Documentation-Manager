use crate::audio_metadata::{has_suno_studio_marker, parse_suno_metadata};
use crate::error::{AppError, Result};
use crate::model::{
    BlockingDeviation, ByteIdenticalPair, ConsistencyIssue, EvidenceDerivedField, EvidenceItem,
    EvidenceRole, FactOrigin, Profile, StepState, StepStatus, TrackAutomation, TrackRecord,
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
    pub allow_explicit_unavailable: Option<bool>,
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
        "code_audio_post_processed",
        "third_party_samples",
        "lyrics_text",
        "human_editing",
        "post_export_editing",
        "artwork_present",
        "ai_artwork",
        "ai_assisted_artwork",
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
        if requirement.allow_explicit_unavailable.is_some()
            && requirement.key != "evidence_licenses.terms_or_unavailable"
        {
            return Err(AppError::Data(format!(
                "Requirement {} cannot configure the terms-unavailable alternative.",
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

fn condition_applies(condition: &str, track: &TrackRecord) -> bool {
    let f = &track.fields;
    match condition {
        "always" => true,
        "external_audio" => f.external_audio_uploaded == Some(true),
        "own_audio" => f.own_audio_uploaded == Some(true),
        "code_based_generation" => f.code_based_generation == Some(true),
        "code_audio_post_processed" => {
            f.code_based_generation == Some(true) && f.code_audio_post_processed == Some(true)
        }
        "third_party_samples" => f.third_party_samples_uploaded == Some(true),
        "lyrics_text" => !matches!(f.lyrics_source.as_str(), "" | "instrumental"),
        "human_editing" => f.human_editing_performed == Some(true),
        "post_export_editing" => f.post_export_editing_performed == Some(true),
        "artwork_present" => !matches!(f.artwork_origin.as_str(), "" | "none"),
        "ai_artwork" => matches!(f.artwork_origin.as_str(), "ai_generated" | "ai_assisted"),
        "ai_assisted_artwork" => f.artwork_origin == "ai_assisted",
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
        "field" if requirement.key == "release.filename_consistency" => filename_requirement_met(
            track,
            evidence,
            EvidenceRole::ReleaseWav,
            track.fields.release_filename_difference_confirmed,
        ),
        "field" if requirement.key == "suno.export_filename_consistency" => {
            filename_requirement_met(
                track,
                evidence,
                EvidenceRole::SunoFinalExport,
                track.fields.suno_export_filename_difference_confirmed,
            )
        }
        "field" if requirement.key == "evidence_licenses.subscription_generation_coverage" => {
            subscription_generation_coverage(track, evidence) == CoverageStatus::Yes
        }
        "field" if requirement.key == "evidence_licenses.terms_or_unavailable" => {
            let has_terms = evidence
                .iter()
                .any(|item| verified_role(item, EvidenceRole::SunoTermsRights));
            if has_terms {
                track.fields.suno_terms_evidence_not_available != Some(true)
            } else {
                requirement.allow_explicit_unavailable == Some(true)
                    && track.fields.suno_terms_evidence_not_available == Some(true)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoverageStatus {
    Yes,
    No,
    NotVerified,
}

pub fn subscription_generation_coverage(
    track: &TrackRecord,
    evidence: &[EvidenceItem],
) -> CoverageStatus {
    let generation_date = track.fields.suno_final_generation_date.trim();
    if generation_date.is_empty() {
        return CoverageStatus::NotVerified;
    }
    let subscriptions = evidence
        .iter()
        .filter(|item| verified_role(item, EvidenceRole::SubscriptionPayment))
        .collect::<Vec<_>>();
    if subscriptions.is_empty()
        || subscriptions.iter().any(|item| {
            item.coverage_start.as_deref().is_none_or(str::is_empty)
                || item.coverage_end.as_deref().is_none_or(str::is_empty)
        })
    {
        return CoverageStatus::NotVerified;
    }
    if subscriptions.iter().any(|item| {
        item.coverage_start
            .as_deref()
            .is_some_and(|start| start <= generation_date)
            && item
                .coverage_end
                .as_deref()
                .is_some_and(|end| end >= generation_date)
    }) {
        CoverageStatus::Yes
    } else {
        CoverageStatus::No
    }
}

pub fn original_evidence_file_name<'a>(
    evidence: &'a [EvidenceItem],
    role: EvidenceRole,
) -> Option<&'a str> {
    evidence
        .iter()
        .find(|item| verified_role(item, role))
        .and_then(|item| {
            let value = item.metadata.original_file_name.trim();
            (!value.is_empty()).then_some(value)
        })
}

pub fn automation_summary(track: &TrackRecord, evidence: &[EvidenceItem]) -> TrackAutomation {
    let suno = relevant_suno_export(evidence);
    let final_generation_origin = fact_origin(
        &track.fields.suno_final_generation_date,
        track.field_origins.suno_final_generation_date.as_ref(),
        suno,
    );
    let production_end_origin = fact_origin(
        &track.fields.production_end_date,
        track.field_origins.production_end_date.as_ref(),
        suno,
    );
    let byte_identical_pairs = byte_identical_pairs(evidence);
    let release_identical_to_suno_export = byte_identical_pairs.iter().any(|pair| {
        matches!(
            (pair.left_role, pair.right_role),
            (EvidenceRole::SunoFinalExport, EvidenceRole::ReleaseWav)
                | (EvidenceRole::ReleaseWav, EvidenceRole::SunoFinalExport)
        )
    });
    TrackAutomation {
        final_generation_origin,
        production_end_origin,
        suno_metadata_detected: suno.is_some_and(|item| item.metadata.suno_studio_detected),
        suno_created_timestamp: suno
            .map(|item| item.metadata.suno_created_timestamp.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        suno_id: suno
            .map(|item| item.metadata.suno_id.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        release_identical_to_suno_export,
        byte_identical_pairs,
        consistency_issues: consistency_issues(track, evidence),
    }
}

pub fn byte_identical_pairs(evidence: &[EvidenceItem]) -> Vec<ByteIdenticalPair> {
    let mut verified = evidence
        .iter()
        .filter(|item| {
            item.verified
                && item.verification_error.is_none()
                && item.sha256.as_deref().is_some_and(|hash| !hash.is_empty())
        })
        .collect::<Vec<_>>();
    verified.sort_by(|left, right| left.id.cmp(&right.id));
    let mut pairs = Vec::new();
    for (index, left) in verified.iter().enumerate() {
        for right in verified.iter().skip(index + 1) {
            if left.sha256 != right.sha256 {
                continue;
            }
            pairs.push(ByteIdenticalPair {
                left_evidence_id: left.id.clone(),
                left_role: left.role,
                right_evidence_id: right.id.clone(),
                right_role: right.role,
                sha256: left.sha256.clone().unwrap_or_default(),
            });
        }
    }
    pairs
}

pub fn consistency_issues(track: &TrackRecord, evidence: &[EvidenceItem]) -> Vec<ConsistencyIssue> {
    let mut issues = Vec::new();
    let suno = relevant_suno_export(evidence);
    if let Some(item) = suno {
        let metadata = &item.metadata;
        if !metadata.suno_created_date.trim().is_empty()
            && !track.fields.suno_final_generation_date.trim().is_empty()
            && track.fields.suno_final_generation_date.trim() != metadata.suno_created_date.trim()
        {
            issues.push(issue(
                "suno_generation_date_conflict",
                "Abweichung zwischen Benutzerangabe und WAV-Metadaten erkannt.",
                "suno",
            ));
        }
        let production_date_is_plausible = track.fields.production_start_date.trim().is_empty()
            || metadata.suno_created_date.as_str() >= track.fields.production_start_date.as_str();
        if track.fields.post_export_editing_performed == Some(false)
            && production_date_is_plausible
            && !metadata.suno_created_date.trim().is_empty()
            && !track.fields.production_end_date.trim().is_empty()
            && track.fields.production_end_date.trim() != metadata.suno_created_date.trim()
        {
            issues.push(issue(
                "production_end_date_conflict",
                "Das dokumentierte Produktionsende weicht vom erkannten Suno-Erzeugungsdatum ab.",
                "track",
            ));
        }
        let embedded_suno_values = metadata
            .embedded_metadata
            .iter()
            .map(|entry| entry.value.as_str())
            .filter(|value| has_suno_studio_marker(value))
            .collect::<Vec<_>>();
        let distinct_embedded_suno_values =
            embedded_suno_values.iter().copied().collect::<HashSet<_>>();
        let has_any_suno_state = metadata.suno_studio_detected
            || !metadata.suno_raw_metadata.is_empty()
            || !metadata.suno_created_timestamp.is_empty()
            || !metadata.suno_created_date.is_empty()
            || !metadata.suno_id.is_empty()
            || !embedded_suno_values.is_empty();
        if distinct_embedded_suno_values.len() > 1 {
            issues.push(issue(
                "suno_metadata_ambiguous",
                "Mehrere widersprüchliche Suno-Metadatensätze wurden erkannt; kein Datum wurde automatisch ausgewählt.",
                "suno",
            ));
        } else if has_any_suno_state
            && !stored_suno_metadata_matches(metadata, &embedded_suno_values)
        {
            issues.push(issue(
                "suno_stored_metadata_mismatch",
                "Gespeicherte Suno-Metadaten stimmen nicht mit dem erhaltenen eingebetteten Wert überein.",
                "suno",
            ));
        }
    }

    for (code, origin) in [
        (
            "suno_generation_origin_stale",
            track.field_origins.suno_final_generation_date.as_ref(),
        ),
        (
            "production_end_origin_stale",
            track.field_origins.production_end_date.as_ref(),
        ),
    ] {
        if origin.is_some_and(|origin| !derived_origin_matches(origin, suno)) {
            issues.push(issue(
                code,
                "Die gespeicherte Evidence-Herkunft verweist nicht mehr auf den aktuellen Suno-Export.",
                "suno",
            ));
        }
    }

    if evidence.iter().any(|item| {
        verified_role(item, EvidenceRole::HumanEditedArtwork)
            && !human_artwork_editing_documented(track)
    }) {
        issues.push(issue(
            "human_artwork_editing_undocumented",
            "Menschlich bearbeitetes Artwork ist vorhanden, aber die Bearbeitung ist nicht dokumentiert.",
            "artwork",
        ));
    }

    let ids = evidence
        .iter()
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    if evidence.iter().any(|item| {
        item.derived_from_evidence_id
            .as_deref()
            .is_some_and(|source_id| source_id == item.id || !ids.contains(source_id))
    }) {
        issues.push(issue(
            "referenced_evidence_missing",
            "Eine automatisch referenzierte Evidence-Datei fehlt.",
            "evidence_licenses",
        ));
    }

    issues.sort_by(|left, right| left.code.cmp(&right.code));
    issues.dedup_by(|left, right| left.code == right.code);
    issues
}

fn relevant_suno_export(evidence: &[EvidenceItem]) -> Option<&EvidenceItem> {
    evidence
        .iter()
        .find(|item| verified_role(item, EvidenceRole::SunoFinalExport))
}

fn stored_suno_metadata_matches(
    metadata: &crate::model::EvidenceMetadata,
    embedded_suno_values: &[&str],
) -> bool {
    if !metadata.suno_studio_detected
        || embedded_suno_values.is_empty()
        || embedded_suno_values
            .iter()
            .any(|value| *value != metadata.suno_raw_metadata)
    {
        return false;
    }
    let Some(parsed) = parse_suno_metadata(&metadata.suno_raw_metadata) else {
        return false;
    };
    metadata.suno_created_timestamp == parsed.created_timestamp
        && metadata.suno_created_date == parsed.created_date
        && metadata.suno_id == parsed.id
}

fn fact_origin(
    value: &str,
    origin: Option<&EvidenceDerivedField>,
    suno: Option<&EvidenceItem>,
) -> FactOrigin {
    if value.trim().is_empty() {
        FactOrigin::NotDocumented
    } else if origin
        .is_some_and(|origin| origin.value == value && derived_origin_matches(origin, suno))
    {
        FactOrigin::EvidenceDerivedMetadata
    } else {
        FactOrigin::UserConfirmedFact
    }
}

fn derived_origin_matches(origin: &EvidenceDerivedField, suno: Option<&EvidenceItem>) -> bool {
    suno.is_some_and(|item| {
        origin.evidence_id == item.id
            && origin.evidence_sha256 == item.sha256.clone().unwrap_or_default()
            && origin.original_value == item.metadata.suno_created_timestamp
            && origin.value == item.metadata.suno_created_date
    })
}

fn human_artwork_editing_documented(track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match fields.artwork_origin.as_str() {
        "human" => {
            fields
                .human_artwork_process_operations
                .iter()
                .any(|value| present(value))
                || present(&fields.human_artwork_process_notes)
        }
        "ai_assisted" => {
            fields
                .human_artwork_modifications
                .iter()
                .any(|value| present(value))
                || present(&fields.custom_artwork_change)
        }
        _ => false,
    }
}

fn issue(code: &str, message: &str, step_id: &str) -> ConsistencyIssue {
    ConsistencyIssue {
        code: code.into(),
        message: message.into(),
        step_id: step_id.into(),
        blocking: true,
    }
}

pub fn filename_matches_documented_title(title: &str, file_name: &str) -> bool {
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

fn filename_requirement_met(
    track: &TrackRecord,
    evidence: &[EvidenceItem],
    role: EvidenceRole,
    confirmed: Option<bool>,
) -> bool {
    original_evidence_file_name(evidence, role).is_some_and(|file_name| {
        filename_matches_documented_title(&track.fields.title, file_name) || confirmed == Some(true)
    })
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
        "source.code_audio_post_processed" => f.code_audio_post_processed.is_some(),
        "source.code_audio_post_processing_operations" => f
            .code_audio_post_processing_operations
            .iter()
            .any(|value| present(value)),
        "source.third_party_samples_uploaded" => f.third_party_samples_uploaded.is_some(),
        "source.third_party_sample_details" => {
            present(&f.third_party_sample_source) && present(&f.third_party_sample_ownership)
        }
        "suno.model" => present(&f.suno_model),
        "suno.project_url" => present(&f.suno_project_url),
        "suno.final_generation_date" => present(&f.suno_final_generation_date),
        "suno.download_export_date" => present(&f.suno_download_export_date),
        "suno.plan_at_creation" => present(&f.suno_plan_at_creation),
        "suno.final_export_date" => present(&f.final_export_date),
        "human_work.lyrics_source" => present(&f.lyrics_source),
        "human_work.instrumental_answer" => f.instrumental_track.is_some(),
        "human_work.instrumental_consistency" => match f.instrumental_track {
            Some(true) => {
                f.lyrics_source == "instrumental"
                    && f.lyrics_text.trim().is_empty()
                    && !(f.human_editing_performed == Some(true)
                        && f.human_editing_details
                            .split(',')
                            .any(|value| value.trim() == "Lyrics"))
            }
            Some(false) => f.lyrics_source != "instrumental",
            None => false,
        },
        "human_work.human_lyrics_details" => present(&f.lyrics_text),
        "human_work.suno_style_prompt" => present(&f.suno_style_prompt),
        "human_work.human_editing_performed" => f.human_editing_performed.is_some(),
        "human_work.human_editing_details" => present(&f.human_editing_details),
        "human_work.post_export_editing_performed" => f.post_export_editing_performed.is_some(),
        "human_work.post_export_editing_details" => present(&f.post_export_editing_details),
        "artwork.origin" => present(&f.artwork_origin),
        "artwork.human_modifications" => f
            .human_artwork_modifications
            .iter()
            .any(|value| present(value)),
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
            field_origins: Default::default(),
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
            metadata: Default::default(),
        }
    }

    fn suno_export(created_date: &str) -> EvidenceItem {
        let mut item = verified_evidence(EvidenceRole::SunoFinalExport);
        item.file_name = "suno.wav".into();
        item.relative_path = "02_SUNO/suno.wav".into();
        item.metadata.suno_studio_detected = true;
        item.metadata.suno_created_timestamp = format!("{created_date}T06:38:06Z");
        item.metadata.suno_created_date = created_date.into();
        item.metadata.suno_id = "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into();
        let raw = format!(
            "made with suno studio; created={created_date}T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828"
        );
        item.metadata.suno_raw_metadata = raw.clone();
        item.metadata.embedded_metadata = vec![crate::model::EmbeddedMetadata {
            key: "ICMT".into(),
            value: raw,
        }];
        item
    }

    #[test]
    fn byte_identity_is_a_system_verification_over_verified_hashes() {
        let suno = suno_export("2026-08-17");
        let mut release = verified_evidence(EvidenceRole::ReleaseWav);
        release.sha256 = suno.sha256.clone();
        let pairs = byte_identical_pairs(&[suno.clone(), release.clone()]);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].sha256, suno.sha256.clone().unwrap());

        release.verified = false;
        assert!(byte_identical_pairs(&[suno, release]).is_empty());
    }

    #[test]
    fn metadata_conflict_is_dynamic_and_blocks_the_existing_suno_step() {
        let mut track = disclosure_track("none", None);
        track.fields.suno_final_generation_date = "2026-08-16".into();
        let evidence = vec![suno_export("2026-08-17")];
        let issues = consistency_issues(&track, &evidence);
        assert!(issues.iter().any(|issue| {
            issue.code == "suno_generation_date_conflict"
                && issue.step_id == "suno"
                && issue.blocking
        }));

        track.fields.suno_final_generation_date = "2026-08-17".into();
        assert!(!consistency_issues(&track, &evidence)
            .iter()
            .any(|issue| issue.code == "suno_generation_date_conflict"));
    }

    #[test]
    fn stored_suno_metadata_requires_exact_embedded_raw_and_complete_parsed_fields() {
        fn mismatch(track: &TrackRecord, item: EvidenceItem) -> bool {
            consistency_issues(track, &[item])
                .iter()
                .any(|issue| issue.code == "suno_stored_metadata_mismatch" && issue.blocking)
        }

        let track = disclosure_track("none", None);
        let valid = suno_export("2026-08-17");
        assert!(!mismatch(&track, valid.clone()));

        let mut missing_embedded_raw = valid.clone();
        missing_embedded_raw.metadata.embedded_metadata.clear();
        assert!(mismatch(&track, missing_embedded_raw));

        let mut missing_timestamp = valid.clone();
        missing_timestamp.metadata.suno_created_timestamp.clear();
        assert!(mismatch(&track, missing_timestamp));

        let mut missing_detection_flag = valid.clone();
        missing_detection_flag.metadata.suno_studio_detected = false;
        assert!(mismatch(&track, missing_detection_flag));

        let mut wrong_date = valid.clone();
        wrong_date.metadata.suno_created_date = "2026-08-18".into();
        assert!(mismatch(&track, wrong_date));

        let mut raw_differs_from_embedded = valid.clone();
        raw_differs_from_embedded
            .metadata
            .suno_raw_metadata
            .push(' ');
        assert!(mismatch(&track, raw_differs_from_embedded));

        // The old contains-based check accepted this because the stored UUID
        // occurs in an unrelated note. The structured id= value is different.
        let mut misleading_uuid_substring = valid.clone();
        let misleading_raw = "made with suno studio; created=2026-08-17T06:38:06Z; id=180ee4f0-977b-4db8-8968-e93e3ac9d506; note=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
        misleading_uuid_substring.metadata.suno_raw_metadata = misleading_raw.into();
        misleading_uuid_substring.metadata.embedded_metadata[0].value = misleading_raw.into();
        assert!(mismatch(&track, misleading_uuid_substring));

        // A stored timestamp appearing as a substring is not enough when the
        // actual created= value is not strict RFC3339.
        let mut invalid_timestamp = valid;
        let invalid_raw = "made with suno studio; created=2026-08-17 06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828; note=2026-08-17T06:38:06Z";
        invalid_timestamp.metadata.suno_raw_metadata = invalid_raw.into();
        invalid_timestamp.metadata.embedded_metadata[0].value = invalid_raw.into();
        assert!(mismatch(&track, invalid_timestamp));

        let mut invalid_uuid = suno_export("2026-08-17");
        let invalid_uuid_raw = "made with suno studio; created=2026-08-17T06:38:06Z; id=not-a-uuid; note=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
        invalid_uuid.metadata.suno_raw_metadata = invalid_uuid_raw.into();
        invalid_uuid.metadata.embedded_metadata[0].value = invalid_uuid_raw.into();
        assert!(mismatch(&track, invalid_uuid));
    }

    #[test]
    fn distinct_embedded_suno_records_are_reported_as_ambiguous() {
        let track = disclosure_track("none", None);
        let mut item = suno_export("2026-08-17");
        item.metadata
            .embedded_metadata
            .push(crate::model::EmbeddedMetadata {
                key: "ICMT".into(),
                value: "made with suno studio; created=2026-08-18T06:38:06Z; id=180ee4f0-977b-4db8-8968-e93e3ac9d506".into(),
            });

        let issues = consistency_issues(&track, &[item]);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "suno_metadata_ambiguous" && issue.blocking));
        assert!(!issues
            .iter()
            .any(|issue| issue.code == "suno_stored_metadata_mismatch"));
    }

    #[test]
    fn production_end_conflict_applies_only_to_confirmed_no_post_editing() {
        let mut track = disclosure_track("none", None);
        track.fields.production_start_date = "2026-08-01".into();
        track.fields.production_end_date = "2026-08-18".into();
        track.fields.post_export_editing_performed = Some(false);
        let evidence = vec![suno_export("2026-08-17")];

        assert!(consistency_issues(&track, &evidence).iter().any(|issue| {
            issue.code == "production_end_date_conflict" && issue.step_id == "track"
        }));

        track.fields.post_export_editing_performed = Some(true);
        assert!(!consistency_issues(&track, &evidence)
            .iter()
            .any(|issue| issue.code == "production_end_date_conflict"));
    }

    #[test]
    fn automatic_fact_origin_requires_the_current_evidence_hash_and_timestamp() {
        let mut track = disclosure_track("none", None);
        let suno = suno_export("2026-08-17");
        track.fields.suno_final_generation_date = "2026-08-17".into();
        track.field_origins.suno_final_generation_date = Some(EvidenceDerivedField {
            value: "2026-08-17".into(),
            original_value: "2026-08-17T06:38:06Z".into(),
            evidence_id: suno.id.clone(),
            evidence_sha256: suno.sha256.clone().unwrap(),
        });
        assert_eq!(
            automation_summary(&track, std::slice::from_ref(&suno)).final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        track
            .field_origins
            .suno_final_generation_date
            .as_mut()
            .unwrap()
            .evidence_sha256 = "b".repeat(64);
        let summary = automation_summary(&track, &[suno]);
        assert_eq!(
            summary.final_generation_origin,
            FactOrigin::UserConfirmedFact
        );
        assert!(summary
            .consistency_issues
            .iter()
            .any(|issue| issue.code == "suno_generation_origin_stale"));
    }

    #[test]
    fn human_edited_artwork_requires_a_documented_artwork_process() {
        let mut track = disclosure_track("ai_generated", None);
        let evidence = vec![verified_evidence(EvidenceRole::HumanEditedArtwork)];
        assert!(consistency_issues(&track, &evidence)
            .iter()
            .any(|issue| issue.code == "human_artwork_editing_undocumented"));

        track.fields.artwork_origin = "ai_assisted".into();
        track.fields.human_artwork_modifications = vec!["Color correction".into()];
        assert!(!consistency_issues(&track, &evidence)
            .iter()
            .any(|issue| issue.code == "human_artwork_editing_undocumented"));
    }

    #[test]
    fn instrumental_contradictions_block_until_explicitly_corrected() {
        let mut track = disclosure_track("none", None);
        track.fields.instrumental_track = Some(true);
        track.fields.lyrics_source = "mixed".into();
        track.fields.lyrics_text = "Used lyric text".into();
        track.fields.human_editing_performed = Some(true);
        track.fields.human_editing_details = "Arrangement, Lyrics".into();
        assert!(!field_requirement_met(
            "human_work.instrumental_consistency",
            &track,
            &Profile::default(),
            &[]
        ));

        track.fields.lyrics_source = "instrumental".into();
        track.fields.lyrics_text.clear();
        track.fields.human_editing_details = "Arrangement".into();
        assert!(field_requirement_met(
            "human_work.instrumental_consistency",
            &track,
            &Profile::default(),
            &[]
        ));
    }

    #[test]
    fn commercial_generation_must_be_inside_verified_subscription_coverage() {
        let mut track = disclosure_track("none", None);
        track.fields.suno_final_generation_date = "2026-08-15".into();
        let mut subscription = verified_evidence(EvidenceRole::SubscriptionPayment);
        subscription.coverage_start = Some("2026-08-01".into());
        subscription.coverage_end = Some("2026-08-31".into());
        assert_eq!(
            subscription_generation_coverage(&track, &[subscription.clone()]),
            CoverageStatus::Yes
        );
        subscription.coverage_end = Some("2026-08-14".into());
        assert_eq!(
            subscription_generation_coverage(&track, &[subscription]),
            CoverageStatus::No
        );
        assert_eq!(
            subscription_generation_coverage(&track, &[]),
            CoverageStatus::NotVerified
        );
    }

    #[test]
    fn mismatching_evidence_filename_requires_explicit_confirmation() {
        let mut track = disclosure_track("none", None);
        track.fields.title = "Gravaty".into();
        let mut release = verified_evidence(EvidenceRole::ReleaseWav);
        release.metadata.original_file_name = "GRAVITY.wav".into();
        let evidence = vec![release];
        assert!(!filename_requirement_met(
            &track,
            &evidence,
            EvidenceRole::ReleaseWav,
            None
        ));
        assert!(filename_requirement_met(
            &track,
            &evidence,
            EvidenceRole::ReleaseWav,
            Some(true)
        ));
        assert_eq!(track.fields.title, "Gravaty");
    }

    #[test]
    fn commercial_terms_status_requires_real_evidence_or_explicit_unavailable_answer() {
        let mut track = disclosure_track("none", None);
        let requirement = WorkflowRequirement {
            key: "evidence_licenses.terms_or_unavailable".into(),
            step_id: "evidence_licenses".into(),
            kind: "field".into(),
            required: true,
            when: "commercial_use".into(),
            missing_message: "missing".into(),
            evidence_role: None,
            allow_explicit_unavailable: Some(true),
        };
        assert!(!requirement_met(
            &requirement,
            &track,
            &Profile::default(),
            &[],
            &HashSet::new(),
            &[]
        ));
        track.fields.suno_terms_evidence_not_available = Some(true);
        assert!(requirement_met(
            &requirement,
            &track,
            &Profile::default(),
            &[],
            &HashSet::new(),
            &[]
        ));
        let mut strict_requirement = requirement.clone();
        strict_requirement.allow_explicit_unavailable = Some(false);
        assert!(!requirement_met(
            &strict_requirement,
            &track,
            &Profile::default(),
            &[],
            &HashSet::new(),
            &[]
        ));
        track.fields.suno_terms_evidence_not_available = None;
        let terms = vec![verified_evidence(EvidenceRole::SunoTermsRights)];
        assert!(requirement_met(
            &requirement,
            &track,
            &Profile::default(),
            &terms,
            &HashSet::from(["suno_terms_rights"]),
            &[]
        ));
        track.fields.suno_terms_evidence_not_available = Some(true);
        assert!(!requirement_met(
            &requirement,
            &track,
            &Profile::default(),
            &terms,
            &HashSet::from(["suno_terms_rights"]),
            &[]
        ));
        track.fields.suno_terms_evidence_not_available = Some(false);
        assert!(requirement_met(
            &strict_requirement,
            &track,
            &Profile::default(),
            &terms,
            &HashSet::from(["suno_terms_rights"]),
            &[]
        ));
    }

    #[test]
    fn code_audio_post_processing_requirements_follow_both_controlling_answers() {
        let mut track = disclosure_track("none", None);
        track.fields.code_based_generation = Some(false);
        assert!(!condition_applies("code_audio_post_processed", &track));

        track.fields.code_based_generation = Some(true);
        track.fields.code_audio_post_processed = Some(false);
        assert!(!condition_applies("code_audio_post_processed", &track));
        assert!(field_requirement_met(
            "source.code_audio_post_processed",
            &track,
            &Profile::default(),
            &[]
        ));

        track.fields.code_audio_post_processed = Some(true);
        assert!(condition_applies("code_audio_post_processed", &track));
        assert!(!field_requirement_met(
            "source.code_audio_post_processing_operations",
            &track,
            &Profile::default(),
            &[]
        ));
        track.fields.code_audio_post_processing_operations =
            vec!["Mixing".into(), "EQ".into(), "Mastering".into()];
        assert!(field_requirement_met(
            "source.code_audio_post_processing_operations",
            &track,
            &Profile::default(),
            &[]
        ));
    }

    #[test]
    fn ai_assisted_artwork_requires_one_or_more_factual_human_changes() {
        let mut track = disclosure_track("ai_assisted", None);
        assert!(condition_applies("ai_assisted_artwork", &track));
        assert!(!field_requirement_met(
            "artwork.human_modifications",
            &track,
            &Profile::default(),
            &[]
        ));
        track.fields.human_artwork_modifications =
            vec!["Prompt written manually".into(), "Cropping".into()];
        assert!(field_requirement_met(
            "artwork.human_modifications",
            &track,
            &Profile::default(),
            &[]
        ));
        track.fields.artwork_origin = "human".into();
        assert!(!condition_applies("ai_assisted_artwork", &track));
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
    fn valid_version_1_4_configuration_is_accepted() {
        let config = embedded_config();

        validate_config(&config).expect("valid workflow 1.4");

        assert_eq!(config.schema_version, 1);
        assert_eq!(config.id, "suno-track");
        assert_eq!(config.version, "1.4");
        assert_eq!(config.steps.len(), 10);
        assert_eq!(config.steps.first().map(|step| step.order), Some(1));
        assert_eq!(config.steps.last().map(|step| step.order), Some(10));
        assert!(!config
            .requirements
            .iter()
            .any(|requirement| requirement.key == "suno.project_or_version_id"));
        assert!(!config
            .requirements
            .iter()
            .any(|requirement| requirement.key == "suno.download_export_date"));
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
