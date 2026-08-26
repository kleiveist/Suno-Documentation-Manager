use super::evaluation::evidence_role_from_str;
use super::*;

pub(super) const WORKFLOW_SOURCE: &str = include_str!("../../../workflows/suno-track.toml");

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

pub(super) fn validate_config(config: &WorkflowConfig) -> Result<()> {
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
    let allowed_conditions = allowed_conditions();
    let allowed_kinds = allowed_kinds();
    let ids = validate_steps(config, &allowed_conditions)?;
    validate_requirements(config, &ids, &allowed_conditions, &allowed_kinds)
}

fn allowed_conditions() -> HashSet<&'static str> {
    [
        "always",
        "external_audio",
        "own_audio",
        "code_based_generation",
        "code_audio_post_processed",
        "third_party_samples",
        "suno_content_non_empty",
        "suno_content_other",
        "human_editing",
        "post_export_editing",
        "artwork_present",
        "ai_artwork",
        "ai_assisted_artwork",
        "ai_transparency_required",
        "real_person",
        "real_event",
        "trademark_or_logo",
        "generative_ai_used",
        "audio_disclosure_yes",
        "commercial_use",
        "finalization_ready",
    ]
    .into_iter()
    .collect()
}

fn allowed_kinds() -> HashSet<&'static str> {
    [
        "field",
        "evidence",
        "generated_document",
        "hash_verification",
        "audio_screening",
    ]
    .into_iter()
    .collect()
}

fn validate_steps<'a>(
    config: &'a WorkflowConfig,
    allowed_conditions: &HashSet<&str>,
) -> Result<HashSet<&'a str>> {
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
    Ok(ids)
}

fn validate_requirements(
    config: &WorkflowConfig,
    ids: &HashSet<&str>,
    allowed_conditions: &HashSet<&str>,
    allowed_kinds: &HashSet<&str>,
) -> Result<()> {
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
