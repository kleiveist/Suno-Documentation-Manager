use crate::application::WorkspaceApp;
use crate::error::{AppError, Result};
use crate::model::{
    ActionResult, CreateTrackInput, DeviationInput, DocumentPreview, EvidenceMetadata,
    EvidencePreview, EvidenceRole, GlobalEvidenceItem, OperationProgress, Profile, StepStatus,
    SubscriptionBillingCycle, TrackCoverPreview, TrackDetail, TrackLibraryPlacement, TrackPatch,
    TrackSummary, ValidationResult, WorkspaceScan, WorkspaceSummary,
};
use crate::workflow::WorkflowDefinition;
use std::sync::{Mutex, MutexGuard};
use tauri::{ipc::Channel, State};

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
    coverage_start: String,
    billing_cycle: SubscriptionBillingCycle,
) -> Result<Option<GlobalEvidenceItem>> {
    let Some(source) = rfd::FileDialog::new()
        .set_title("Globalen Nachweis registrieren")
        .add_filter("Unterstützte Nachweise", role.allowed_extensions())
        .pick_file()
    else {
        return Ok(None);
    };
    with_workspace(&state, |app| {
        app.register_global_evidence_for_billing_cycle(
            role,
            &source,
            &coverage_start,
            billing_cycle,
        )
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
pub fn list_albums(state: State<'_, AppState>) -> Result<Vec<String>> {
    with_workspace(&state, WorkspaceApp::list_albums)
}

#[tauri::command]
pub fn create_album(state: State<'_, AppState>, title: String) -> Result<Vec<String>> {
    with_workspace(&state, |app| app.create_album(&title))
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
pub async fn load_track_cover(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Option<TrackCoverPreview>> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || workspace.track_cover(&track_id))
        .await
        .map_err(|error| AppError::Data(format!("Track cover task failed: {error}")))?
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
pub fn update_track_library(
    state: State<'_, AppState>,
    track_id: String,
    input: TrackLibraryPlacement,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.update_track_library(&track_id, input))
}

#[tauri::command]
pub fn rename_album(
    state: State<'_, AppState>,
    old_title: String,
    new_title: String,
) -> Result<Vec<TrackSummary>> {
    with_workspace(&state, |app| app.rename_album(&old_title, &new_title))
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
pub async fn import_evidence(
    state: State<'_, AppState>,
    track_id: String,
    role: EvidenceRole,
    replace_evidence_id: Option<String>,
    metadata: Option<EvidenceMetadata>,
) -> Result<Option<TrackDetail>> {
    let Some(source) = rfd::FileDialog::new()
        .set_title("Evidence importieren")
        .pick_file()
    else {
        return Ok(None);
    };
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || match replace_evidence_id {
        Some(evidence_id) => workspace
            .replace_evidence_with_metadata_from(
                &track_id,
                &evidence_id,
                role,
                &source,
                metadata.unwrap_or_default(),
            )
            .map(Some),
        None => workspace
            .import_evidence_with_metadata_from(
                &track_id,
                role,
                &source,
                metadata.unwrap_or_default(),
            )
            .map(Some),
    })
    .await
    .map_err(|error| AppError::Data(format!("Evidence import task failed: {error}")))?
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
pub async fn preview_evidence(
    state: State<'_, AppState>,
    track_id: String,
    evidence_id: String,
) -> Result<EvidencePreview> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        workspace.preview_evidence(&track_id, &evidence_id)
    })
    .await
    .map_err(|error| AppError::Data(format!("Evidence preview task failed: {error}")))?
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
pub async fn generate_documents(
    state: State<'_, AppState>,
    track_id: String,
    adopt_existing: bool,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        workspace.generate_documents_with_progress(&track_id, adopt_existing, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("Document generation task failed: {error}")))?
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
pub async fn calculate_hashes(
    state: State<'_, AppState>,
    track_id: String,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        workspace.calculate_hashes_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("SHA-256 calculation task failed: {error}")))?
}

#[tauri::command]
pub async fn verify_hashes(
    state: State<'_, AppState>,
    track_id: String,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        workspace.verify_hashes_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("SHA-256 verification task failed: {error}")))?
}

#[tauri::command]
pub fn validate_track(state: State<'_, AppState>, track_id: String) -> Result<ValidationResult> {
    with_workspace(&state, |app| app.validate_track(&track_id))
}

#[tauri::command]
pub async fn finalize_track(
    state: State<'_, AppState>,
    track_id: String,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = state
        .lock()?
        .as_ref()
        .cloned()
        .ok_or(AppError::NoWorkspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        workspace.finalize_track_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("Finalization task failed: {error}")))?
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
