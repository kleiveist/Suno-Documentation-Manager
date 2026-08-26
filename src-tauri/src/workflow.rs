use crate::audio_metadata::{has_suno_metadata_marker, parse_suno_metadata};
use crate::audio_screening;
use crate::error::{AppError, Result};
#[cfg(test)]
use crate::model::DocumentationAnswer;
use crate::model::{
    BlockingDeviation, ByteIdenticalPair, ConsistencyIssue, EvidenceDerivedField, EvidenceItem,
    EvidenceRole, FactOrigin, Profile, StepState, StepStatus, SunoContentClassification,
    TrackAutomation, TrackRecord,
};
use chrono::{Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod automation;
mod configuration;
mod coverage;
mod evaluation;

#[cfg(test)]
pub use automation::consistency_issues;
pub use automation::{
    automation_summary, byte_identical_pairs, human_edited_final_artwork_sha256_match,
    human_edited_final_artwork_status, original_evidence_file_name,
};
#[cfg(test)]
pub use configuration::config_with_version_for_test;
pub use configuration::{
    config, definition, WorkflowConfig, WorkflowDefinition, WorkflowEvaluation, WorkflowRequirement,
};
pub use coverage::{
    subscription_generation_coverage, subscription_production_coverage, CoverageStatus,
};
pub use evaluation::{can_mark_na, evaluate, evidence_role_from_str, progress};

#[cfg(test)]
use configuration::{validate_config, WORKFLOW_SOURCE};
#[cfg(test)]
use evaluation::{
    condition_applies, content_check_all_negative, disclosure_required, field_requirement_met,
    filename_requirement_met, requirement_met,
};

#[cfg(test)]
mod tests;
