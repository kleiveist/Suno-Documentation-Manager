use super::*;

pub(super) fn embedded_config() -> WorkflowConfig {
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
fn valid_version_1_9_configuration_is_accepted() {
    let config = embedded_config();

    validate_config(&config).expect("valid workflow 1.9");

    assert_eq!(config.schema_version, 1);
    assert_eq!(config.id, "suno-track");
    assert_eq!(config.version, "1.9");
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
    assert!(config.requirements.iter().any(|requirement| {
        requirement.key == "suno.final_export_date" && requirement.step_id == "release"
    }));
    assert!(config
        .requirements
        .iter()
        .any(|requirement| requirement.key == "suno.plan_at_generation"));
    assert!(!config
        .requirements
        .iter()
        .any(|requirement| requirement.key == "suno.plan_at_creation"));
    assert!(config.requirements.iter().any(|requirement| {
        requirement.key == "human_work.post_export_editing_performed"
            && requirement.step_id == "release"
    }));
    assert!(config.requirements.iter().any(|requirement| {
        requirement.key == "release.audio_screening_local"
            && requirement.kind == "audio_screening"
            && requirement.step_id == "release"
            && requirement.required
    }));
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
