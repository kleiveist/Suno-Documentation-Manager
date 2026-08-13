use crate::application::WorkspaceApp;
use crate::error::{AppError, Result};
use crate::model::{
    ActionResult, CreateTrackInput, DeviationInput, DocumentPreview, EvidenceRole,
    GlobalEvidenceItem, Profile, StepStatus, TrackDetail, TrackPatch, TrackSummary,
    ValidationResult, WorkspaceScan, WorkspaceSummary,
};
use crate::workflow::WorkflowDefinition;
use std::sync::{Mutex, MutexGuard};
use tauri::State;

#[derive(Default)]
pub struct AppState {
    workspace: Mutex<Option<WorkspaceApp>>,
}

impl AppState {
    fn lock(&self) -> Result<MutexGuard<'_, Option<WorkspaceApp>>> {
        self.workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))
    }
}

fn with_workspace<T>(
    state: &State<'_, AppState>,
    operation: impl FnOnce(&WorkspaceApp) -> Result<T>,
) -> Result<T> {
    let guard = state.lock()?;
    operation(guard.as_ref().ok_or(AppError::NoWorkspace)?)
}

#[tauri::command]
pub fn get_workflow() -> Result<WorkflowDefinition> {
    crate::workflow::definition()
}

#[tauri::command]
pub fn open_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceSummary>> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Workspace auswählen")
        .pick_folder()
    else {
        return Ok(None);
    };
    let app = WorkspaceApp::open(&path, false)?;
    let summary = app.summary()?;
    *state.lock()? = Some(app);
    Ok(Some(summary))
}

#[tauri::command]
pub fn create_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceSummary>> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Neuen Workspace anlegen")
        .pick_folder()
    else {
        return Ok(None);
    };
    let app = WorkspaceApp::open(&path, true)?;
    let summary = app.summary()?;
    *state.lock()? = Some(app);
    Ok(Some(summary))
}

#[tauri::command]
pub fn scan_workspace(state: State<'_, AppState>) -> Result<WorkspaceScan> {
    with_workspace(&state, WorkspaceApp::scan_workspace)
}

#[tauri::command]
pub fn get_profile(state: State<'_, AppState>) -> Result<Profile> {
    with_workspace(&state, WorkspaceApp::profile)
}

#[tauri::command]
pub fn update_profile(state: State<'_, AppState>, profile: Profile) -> Result<Profile> {
    with_workspace(&state, |app| app.update_profile(profile))
}

#[tauri::command]
pub fn list_global_evidence(state: State<'_, AppState>) -> Result<Vec<GlobalEvidenceItem>> {
    with_workspace(&state, WorkspaceApp::global_evidence)
}

#[tauri::command]
pub fn import_global_evidence(
    state: State<'_, AppState>,
    role: EvidenceRole,
    coverage_start: Option<String>,
    coverage_end: Option<String>,
) -> Result<Option<GlobalEvidenceItem>> {
    let Some(source) = rfd::FileDialog::new()
        .set_title("Globalen Nachweis registrieren")
        .pick_file()
    else {
        return Ok(None);
    };
    with_workspace(&state, |app| {
        app.register_global_evidence(role, &source, coverage_start, coverage_end)
            .map(Some)
    })
}

#[tauri::command]
pub fn remove_global_evidence(state: State<'_, AppState>, evidence_id: String) -> Result<()> {
    with_workspace(&state, |app| app.remove_global_evidence(&evidence_id))
}

#[tauri::command]
pub fn attach_global_evidence(
    state: State<'_, AppState>,
    track_id: String,
    evidence_id: String,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| {
        app.attach_global_evidence(&track_id, &evidence_id)
    })
}

#[tauri::command]
pub fn list_tracks(state: State<'_, AppState>) -> Result<Vec<TrackSummary>> {
    with_workspace(&state, WorkspaceApp::list_tracks)
}

#[tauri::command]
pub fn create_track(state: State<'_, AppState>, input: CreateTrackInput) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.create_track(input))
}

#[tauri::command]
pub fn load_track(state: State<'_, AppState>, track_id: String) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.load_track(&track_id))
}

#[tauri::command]
pub fn update_track(
    state: State<'_, AppState>,
    track_id: String,
    input: TrackPatch,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.update_track(&track_id, input))
}

#[tauri::command]
pub fn adopt_legacy_profile(state: State<'_, AppState>, track_id: String) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.adopt_legacy_profile(&track_id))
}

#[tauri::command]
pub fn add_deviation(
    state: State<'_, AppState>,
    track_id: String,
    input: DeviationInput,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.add_deviation(&track_id, input))
}

#[tauri::command]
pub fn resolve_deviation(
    state: State<'_, AppState>,
    track_id: String,
    deviation_id: String,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| {
        app.resolve_deviation(&track_id, &deviation_id)
    })
}

#[tauri::command]
pub fn remove_deviation(
    state: State<'_, AppState>,
    track_id: String,
    deviation_id: String,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.remove_deviation(&track_id, &deviation_id))
}

#[tauri::command]
pub fn set_step_status(
    state: State<'_, AppState>,
    track_id: String,
    step_id: String,
    status: StepStatus,
    na_reason: Option<String>,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| {
        app.set_step_status(&track_id, &step_id, status, na_reason)
    })
}

#[tauri::command]
pub fn import_evidence(
    state: State<'_, AppState>,
    track_id: String,
    role: EvidenceRole,
) -> Result<Option<TrackDetail>> {
    let Some(source) = rfd::FileDialog::new()
        .set_title("Evidence importieren")
        .pick_file()
    else {
        return Ok(None);
    };
    with_workspace(&state, |app| {
        app.import_evidence_from(&track_id, role, &source).map(Some)
    })
}

#[tauri::command]
pub fn remove_evidence(
    state: State<'_, AppState>,
    track_id: String,
    evidence_id: String,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.remove_evidence(&track_id, &evidence_id))
}

#[tauri::command]
pub fn verify_evidence(
    state: State<'_, AppState>,
    track_id: String,
    evidence_id: Option<String>,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| {
        app.verify_evidence(&track_id, evidence_id.as_deref())
    })
}

#[tauri::command]
pub fn preview_documents(state: State<'_, AppState>, track_id: String) -> Result<DocumentPreview> {
    with_workspace(&state, |app| app.preview_documents(&track_id))
}

#[tauri::command]
pub fn generate_documents(
    state: State<'_, AppState>,
    track_id: String,
    adopt_existing: bool,
) -> Result<ActionResult> {
    with_workspace(&state, |app| {
        app.generate_documents(&track_id, adopt_existing)
    })
}

#[tauri::command]
pub fn generate_artwork_disclosure(
    state: State<'_, AppState>,
    track_id: String,
    disclosure_text: Option<String>,
) -> Result<ActionResult> {
    with_workspace(&state, |app| {
        app.generate_artwork_disclosure(&track_id, disclosure_text)
    })
}

#[tauri::command]
pub fn calculate_hashes(state: State<'_, AppState>, track_id: String) -> Result<ActionResult> {
    with_workspace(&state, |app| app.calculate_hashes(&track_id))
}

#[tauri::command]
pub fn verify_hashes(state: State<'_, AppState>, track_id: String) -> Result<ActionResult> {
    with_workspace(&state, |app| app.verify_hashes(&track_id))
}

#[tauri::command]
pub fn validate_track(state: State<'_, AppState>, track_id: String) -> Result<ValidationResult> {
    with_workspace(&state, |app| app.validate_track(&track_id))
}

#[tauri::command]
pub fn finalize_track(state: State<'_, AppState>, track_id: String) -> Result<ActionResult> {
    with_workspace(&state, |app| app.finalize_track(&track_id))
}

#[tauri::command]
pub fn invalidate_certificate(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<ActionResult> {
    with_workspace(&state, |app| app.invalidate_certificate(&track_id))
}

#[tauri::command]
pub fn create_revision(state: State<'_, AppState>, track_id: String) -> Result<ActionResult> {
    with_workspace(&state, |app| app.create_revision(&track_id))
}

#[tauri::command]
pub fn re_evaluate_track(state: State<'_, AppState>, track_id: String) -> Result<ActionResult> {
    with_workspace(&state, |app| app.re_evaluate_track(&track_id))
}
