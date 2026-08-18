use crate::application::WorkspaceApp;
use crate::error::{AppError, Result};
use crate::folder_import::{FolderImportExecutionInput, FolderImportProposal};
use crate::model::{
    ActionResult, AudioScreeningProviderTestResult, AudioScreeningSecretInput,
    AudioScreeningSettings, CreateTrackInput, DeviationInput, DocumentPreview, EvidenceMetadata,
    EvidencePreview, EvidenceRole, FinalizeOptions, GlobalEvidenceItem, OperationProgress, Profile,
    StepStatus, SubscriptionBillingCycle, TimestampProviderTestResult, TimestampSecretInput,
    TimestampSettings, TrackCoverPreview, TrackDetail, TrackLibraryPlacement, TrackPatchRequest,
    TrackSummary, ValidationResult, WorkspaceScan, WorkspaceSummary,
};
use crate::workflow::WorkflowDefinition;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{ipc::Channel, State};

#[derive(Default)]
pub struct AppState {
    // Long-running native operations execute on blocking workers.  Keep the
    // workspace mutex shared with those workers rather than cloning the app
    // and releasing the coordinator: a release replacement must not race an
    // older screening/finalization save and overwrite the newer state.
    workspace: Arc<Mutex<Option<WorkspaceApp>>>,
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
pub fn get_timestamp_settings(state: State<'_, AppState>) -> Result<TimestampSettings> {
    with_workspace(&state, WorkspaceApp::timestamp_settings)
}

#[tauri::command]
pub fn update_timestamp_settings(
    state: State<'_, AppState>,
    settings: TimestampSettings,
) -> Result<TimestampSettings> {
    with_workspace(&state, |app| app.update_timestamp_settings(settings))
}

#[tauri::command]
pub fn update_timestamp_secret(
    state: State<'_, AppState>,
    input: TimestampSecretInput,
) -> Result<()> {
    with_workspace(&state, |app| app.update_timestamp_secret(input))
}

#[tauri::command]
pub async fn test_timestamp_provider(
    state: State<'_, AppState>,
) -> Result<TimestampProviderTestResult> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .test_timestamp_provider()
    })
    .await
    .map_err(|error| AppError::Data(format!("Timestamp provider test failed: {error}")))?
}

#[tauri::command]
pub fn get_audio_screening_settings(state: State<'_, AppState>) -> Result<AudioScreeningSettings> {
    with_workspace(&state, WorkspaceApp::audio_screening_settings)
}

#[tauri::command]
pub fn update_audio_screening_settings(
    state: State<'_, AppState>,
    settings: AudioScreeningSettings,
) -> Result<AudioScreeningSettings> {
    with_workspace(&state, |app| app.update_audio_screening_settings(settings))
}

#[tauri::command]
pub fn update_audio_screening_secret(
    state: State<'_, AppState>,
    input: AudioScreeningSecretInput,
) -> Result<()> {
    with_workspace(&state, |app| app.update_audio_screening_secret(input))
}

#[tauri::command]
pub async fn test_audio_screening_provider(
    state: State<'_, AppState>,
) -> Result<AudioScreeningProviderTestResult> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .test_audio_screening_provider()
    })
    .await
    .map_err(|error| AppError::Data(format!("Audio-screening provider test failed: {error}")))?
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
pub fn import_global_terms_evidence(
    state: State<'_, AppState>,
    metadata: EvidenceMetadata,
) -> Result<Option<GlobalEvidenceItem>> {
    let role = EvidenceRole::SunoTermsRights;
    let Some(source) = rfd::FileDialog::new()
        .set_title("Suno-Nutzungsbedingungen als PDF auswählen")
        .add_filter("PDF", role.allowed_extensions())
        .pick_file()
    else {
        return Ok(None);
    };
    with_workspace(&state, |app| {
        app.register_global_terms_evidence(&source, metadata)
            .map(Some)
    })
}

#[tauri::command]
pub fn update_global_terms_evidence_metadata(
    state: State<'_, AppState>,
    evidence_id: String,
    metadata: EvidenceMetadata,
) -> Result<GlobalEvidenceItem> {
    with_workspace(&state, |app| {
        app.update_global_terms_evidence_metadata(&evidence_id, metadata)
    })
}

#[tauri::command]
pub async fn attach_configured_external_timestamp(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<TrackDetail> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .attach_configured_external_timestamp(&track_id)
    })
    .await
    .map_err(|error| AppError::Data(format!("Timestamp attachment task failed: {error}")))?
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
pub fn scan_import_folder(state: State<'_, AppState>) -> Result<Option<FolderImportProposal>> {
    let Some(source) = rfd::FileDialog::new()
        .set_title("Musikprojekt-Ordner importieren")
        .pick_folder()
    else {
        return Ok(None);
    };
    with_workspace(&state, |app| app.scan_folder_import(&source).map(Some))
}

#[tauri::command]
pub async fn execute_folder_import(
    state: State<'_, AppState>,
    input: FolderImportExecutionInput,
) -> Result<Vec<TrackDetail>> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .import_folder(input)
    })
    .await
    .map_err(|error| AppError::Data(format!("Folder import task failed: {error}")))?
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .track_cover(&track_id)
    })
    .await
    .map_err(|error| AppError::Data(format!("Track cover task failed: {error}")))?
}

#[tauri::command]
pub fn update_track(
    state: State<'_, AppState>,
    track_id: String,
    input: TrackPatchRequest,
) -> Result<TrackDetail> {
    with_workspace(&state, |app| app.update_track_request(&track_id, input))
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || match replace_evidence_id {
        Some(evidence_id) => {
            let guard = workspace
                .lock()
                .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
            guard
                .as_ref()
                .ok_or(AppError::NoWorkspace)?
                .replace_evidence_with_metadata_from(
                    &track_id,
                    &evidence_id,
                    role,
                    &source,
                    metadata.unwrap_or_default(),
                )
                .map(Some)
        }
        None => {
            let guard = workspace
                .lock()
                .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
            guard
                .as_ref()
                .ok_or(AppError::NoWorkspace)?
                .import_evidence_with_metadata_from(
                    &track_id,
                    role,
                    &source,
                    metadata.unwrap_or_default(),
                )
                .map(Some)
        }
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        guard
            .as_ref()
            .ok_or(AppError::NoWorkspace)?
            .preview_evidence(&track_id, &evidence_id)
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.generate_documents_with_progress(&track_id, adopt_existing, &mut |progress| {
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.calculate_hashes_with_progress(&track_id, &mut |progress| {
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
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.verify_hashes_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("SHA-256 verification task failed: {error}")))?
}

#[tauri::command]
pub async fn run_local_audio_screening(
    state: State<'_, AppState>,
    track_id: String,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.run_local_audio_screening_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("Local audio-screening task failed: {error}")))?
}

#[tauri::command]
pub async fn run_external_audio_screening(
    state: State<'_, AppState>,
    track_id: String,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.run_external_audio_screening_with_progress(&track_id, &mut |progress| {
            let _ = on_progress.send(progress);
        })
    })
    .await
    .map_err(|error| AppError::Data(format!("External audio-screening task failed: {error}")))?
}

#[tauri::command]
pub fn validate_track(state: State<'_, AppState>, track_id: String) -> Result<ValidationResult> {
    with_workspace(&state, |app| app.validate_track(&track_id))
}

#[tauri::command]
pub async fn finalize_track(
    state: State<'_, AppState>,
    track_id: String,
    options: Option<FinalizeOptions>,
    on_progress: Channel<OperationProgress>,
) -> Result<ActionResult> {
    let workspace = Arc::clone(&state.workspace);
    tauri::async_runtime::spawn_blocking(move || {
        let guard = workspace
            .lock()
            .map_err(|_| AppError::Data("Workspace state lock is unavailable.".into()))?;
        let app = guard.as_ref().ok_or(AppError::NoWorkspace)?;
        app.finalize_track_with_options_and_progress(
            &track_id,
            options.unwrap_or_default(),
            &mut |progress| {
                let _ = on_progress.send(progress);
            },
        )
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
