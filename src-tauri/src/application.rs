use crate::audio_screening;
use crate::certificate;
use crate::documents;
use crate::error::{AppError, Result};
use crate::evidence;
use crate::external_timestamp;
use crate::folder_import::{self, FolderImportExecutionInput, FolderImportProposal};
use crate::integrity;
use crate::model::{
    ActionResult, AudioScreeningProviderStatus, AudioScreeningProviderTestResult,
    AudioScreeningSecretInput, AudioScreeningSettings, AudioScreeningStatus, AudioScreeningSummary,
    BlockingDeviation, CertificateRenderOptions, CertificateState, CreateTrackInput,
    DeviationInput, DocumentPreview, DocumentState, EvidenceDerivedField, EvidenceItem,
    EvidenceMetadata, EvidencePreview, EvidenceProvenance, EvidenceRole, ExternalTimestampInput,
    ExternalTimestampRecord, ExternalTimestampStatus, ExternalTimestampSummary, FinalizationAnchor,
    FinalizeOptions, GlobalEvidenceItem, IntegrityState, LegacyCandidate, OperationProgress,
    Profile, StepState, StepStatus, SubscriptionBillingCycle, TimestampProviderTestResult,
    TimestampSecretInput, TimestampSettings, TrackCoverPreview, TrackDetail, TrackLibraryPlacement,
    TrackLibrarySection, TrackPatch, TrackPatchRequest, TrackRecord, TrackStatus, TrackSummary,
    ValidationResult, WorkspaceScan, WorkspaceSummary,
};
use crate::persistence::Persistence;
use crate::security::{
    atomic_write, atomic_write_new, canonical_workspace, contained_path, copy_new,
    ensure_contained_directory, portable_relative, sha256_file, slugify,
};
use crate::workflow;
use base64::Engine;
use chrono::{Months, NaiveDate, Utc};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;
use walkdir::WalkDir;

pub const TRACK_FOLDERS: [&str; 8] = [
    ".archive",
    "01_RELEASE",
    "02_SUNO",
    "03_DOCUMENTATION",
    "04_LICENSES",
    "05_ARTWORK",
    "06_CERTIFICATE",
    ".archive/revisions",
];

const SINGLES_DIRECTORY: &str = "Singles";
const TRACK_IDENTITY_FILE: &str = ".summary/track.json";

#[derive(Debug)]
pub struct WorkspaceApp {
    root: PathBuf,
    persistence: Persistence,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FinalizationFailure {
    DatabaseCommit,
    PdfGeneration,
    PdfStaging,
    PdfPublication,
    PostPublishVerification,
}

#[cfg(test)]
impl FinalizationFailure {
    fn certificate_failure(self) -> Option<certificate::CertificateGenerationFailure> {
        match self {
            Self::DatabaseCommit => None,
            Self::PdfGeneration => Some(certificate::CertificateGenerationFailure::PdfGeneration),
            Self::PdfStaging => Some(certificate::CertificateGenerationFailure::PdfStaging),
            Self::PdfPublication => Some(certificate::CertificateGenerationFailure::PdfPublication),
            Self::PostPublishVerification => {
                Some(certificate::CertificateGenerationFailure::PostPublishVerification)
            }
        }
    }
}

impl WorkspaceApp {
    pub fn open(path: &Path, create: bool) -> Result<Self> {
        let root = canonical_workspace(path, create)?;
        let persistence = Persistence::initialize(&root)?;
        if persistence.get_meta("workspace_id")?.is_none() {
            persistence.set_meta("workspace_id", &Uuid::new_v4().to_string())?;
        }
        let app = Self { root, persistence };
        ensure_contained_directory(&app.root, Path::new(SINGLES_DIRECTORY))?;
        app.reconcile_physical_library()?;
        app.recover_interrupted_operations()?;
        let profile = app.profile()?;
        app.synchronize_open_track_profiles(&profile, false)?;
        Ok(app)
    }

    fn synchronize_open_track_profiles(&self, profile: &Profile, save_profile: bool) -> Result<()> {
        let mut tracks = self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
            .collect::<Vec<_>>();
        let updated_at = now();
        let mut changed = false;
        for track in &mut tracks {
            if matches!(
                track.status,
                TrackStatus::Finalized | TrackStatus::Superseded
            ) || track
                .profile_snapshot
                .same_track_documentation_profile(profile)
            {
                continue;
            }
            track.profile_snapshot = profile.clone();
            mark_content_changed(track);
            track.status = TrackStatus::Active;
            track.updated_at = updated_at.clone();
            changed = true;
        }
        if save_profile {
            self.persistence.save_profile_and_tracks(profile, &tracks)?;
        } else if changed {
            self.persistence.save_tracks(&tracks)?;
        }
        Ok(())
    }

    fn recover_interrupted_operations(&self) -> Result<()> {
        let mut recovered = false;
        for track in self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
        {
            if !self.root.join(&track.relative_path).is_dir() {
                continue;
            }
            let root = self.track_root(&track)?;
            let live = contained_path(&root, Path::new(certificate::CERTIFICATE_DIR), false)?;
            let live_pdf = contained_path(&root, Path::new(certificate::PDF_FILE), false)?;

            if track.status == TrackStatus::Finalized
                && finalized_artifacts_need_revision_restore(&root, &live, &live_pdf)?
            {
                let staging = contained_path(&root, Path::new(".archive/revision-staging"), false)?;
                if staging.is_dir() {
                    for entry in
                        fs::read_dir(&staging).map_err(|error| AppError::io(&staging, error))?
                    {
                        let entry = entry.map_err(|error| AppError::io(&staging, error))?;
                        let entry_path = entry.path();
                        let entry_metadata = fs::symlink_metadata(&entry_path)
                            .map_err(|error| AppError::io(&entry_path, error))?;
                        if entry_metadata.file_type().is_symlink() || !entry_metadata.is_dir() {
                            continue;
                        }
                        let entry_relative = entry_path
                            .strip_prefix(&root)
                            .map_err(|_| AppError::PathEscape)?
                            .to_owned();
                        if let Some(candidate) =
                            matching_revision_certificate(&root, &entry_relative, &track)?
                        {
                            if restore_revision_artifacts(&live, &live_pdf, &candidate)? {
                                let _ = fs::remove_dir_all(&entry_path);
                                recovered = true;
                                break;
                            }
                        }
                    }
                }
            }

            if track.status == TrackStatus::Finalized
                && finalized_artifacts_need_revision_restore(&root, &live, &live_pdf)?
            {
                let revisions = contained_path(&root, Path::new(".archive/revisions"), false)?;
                if revisions.is_dir() {
                    let mut entries = fs::read_dir(&revisions)
                        .map_err(|error| AppError::io(&revisions, error))?
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(|error| AppError::io(&revisions, error))?;
                    entries.sort_by_key(|entry| entry.file_name());
                    entries.reverse();
                    for entry in entries {
                        let entry_metadata = fs::symlink_metadata(entry.path())
                            .map_err(|error| AppError::io(entry.path(), error))?;
                        if entry_metadata.file_type().is_symlink() || !entry_metadata.is_dir() {
                            continue;
                        }
                        let entry_path = entry.path();
                        let entry_relative = entry_path
                            .strip_prefix(&root)
                            .map_err(|_| AppError::PathEscape)?;
                        let Some(candidate) =
                            matching_revision_certificate(&root, entry_relative, &track)?
                        else {
                            continue;
                        };
                        if restore_revision_artifacts(&live, &live_pdf, &candidate)? {
                            let _ = fs::remove_dir_all(&entry_path);
                            recovered = true;
                            break;
                        }
                    }
                }
            }

            // A non-finalized track may be an imported legacy folder whose historical
            // certificate bytes must remain untouched. Finalization crash recovery is
            // therefore performed only when a transaction marker created immediately
            // before certificate publication is present.
            let finalization_marker = contained_path(
                &root,
                Path::new(".archive/finalization-in-progress.json"),
                false,
            )?;
            if track.status == TrackStatus::Finalized && finalization_marker.is_file() {
                fs::remove_file(&finalization_marker)
                    .map_err(|error| AppError::io(&finalization_marker, error))?;
                recovered = true;
            }
            if track.status != TrackStatus::Finalized && finalization_marker.is_file() {
                let marker: serde_json::Value = serde_json::from_slice(
                    &fs::read(&finalization_marker)
                        .map_err(|error| AppError::io(&finalization_marker, error))?,
                )?;
                if marker.get("track_id").and_then(|value| value.as_str())
                    != Some(track.id.as_str())
                {
                    return Err(AppError::Data(
                        "Finalization recovery marker does not match its track.".into(),
                    ));
                }
                let recovery_id = marker
                    .get("transaction_id")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| {
                        AppError::Data("Finalization recovery marker has no transaction ID.".into())
                    })?;
                let staging_relative =
                    PathBuf::from(".archive/certificate-staging").join(recovery_id);
                let staging = contained_path(&root, &staging_relative, false)?;
                let live_needs_recovery = !directory_is_empty_or_missing(&live)?;
                let live_pdf_needs_recovery = live_pdf.exists();
                let staging_needs_recovery = staging.exists();
                if live_needs_recovery || live_pdf_needs_recovery || staging_needs_recovery {
                    let recovery_relative = PathBuf::from(".archive/recovery").join(recovery_id);
                    let recovery = ensure_contained_directory(&root, &recovery_relative)?;
                    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
                        "schema_version": 1,
                        "track_id": track.id,
                        "recovered_at": now(),
                        "reason": "certificate publication was interrupted before the database committed FINALIZED",
                        "live_certificate_recovered": live_needs_recovery,
                        "live_pdf_recovered": live_pdf_needs_recovery,
                        "staging_recovered": staging_needs_recovery,
                    }))?;
                    let recovery_metadata = recovery.join("recovery.json");
                    if !recovery_metadata.exists() {
                        atomic_write_new(&recovery_metadata, &metadata)?;
                    }
                    if live_needs_recovery {
                        fs::rename(&live, recovery.join("certificate"))
                            .map_err(|error| AppError::io(&live, error))?;
                    }
                    if live_pdf_needs_recovery {
                        fs::rename(&live_pdf, recovery.join(certificate::PDF_FILE))
                            .map_err(|error| AppError::io(&live_pdf, error))?;
                    }
                    if staging_needs_recovery {
                        fs::rename(&staging, recovery.join("certificate-staging"))
                            .map_err(|error| AppError::io(&staging, error))?;
                    }
                }
                // Recovery is idempotent across process exits between the
                // individual moves. Recreate the managed empty directory even
                // when a prior attempt already moved every correlated artifact.
                ensure_contained_directory(&root, Path::new(certificate::CERTIFICATE_DIR))?;
                fs::remove_file(&finalization_marker)
                    .map_err(|error| AppError::io(&finalization_marker, error))?;
                recovered = true;
            }

            let timestamp_records = self.persistence.external_timestamps(&track.id)?;
            if external_timestamp::reconcile_publications(&root, &timestamp_records)? {
                recovered = true;
            }
        }
        if recovered {
            self.persistence.set_meta("last_recovery_at", &now())?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn summary(&self) -> Result<WorkspaceSummary> {
        let id = self
            .persistence
            .get_meta("workspace_id")?
            .unwrap_or_else(|| "local-workspace".into());
        let name = self
            .root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workspace")
            .to_owned();
        Ok(WorkspaceSummary {
            id,
            name,
            path: self.root.display().to_string(),
            track_count: self
                .persistence
                .tracks()?
                .into_iter()
                .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
                .count() as u32,
            last_scanned_at: self.persistence.get_meta("last_scanned_at")?,
        })
    }

    pub fn profile(&self) -> Result<Profile> {
        self.persistence.profile()
    }

    pub fn update_profile(&self, profile: Profile) -> Result<Profile> {
        validate_profile(&profile, false)?;
        self.synchronize_open_track_profiles(&profile, true)?;
        Ok(profile)
    }

    /// Returns the workspace-global, non-secret timestamp configuration. It
    /// never travels through a track profile snapshot or a certificate.
    pub fn timestamp_settings(&self) -> Result<TimestampSettings> {
        let mut settings = self.persistence.timestamp_settings()?;
        let (status, message) = external_timestamp::settings_status(
            &settings,
            self.persistence.timestamp_secret_present()?,
        );
        settings.status = status;
        settings.status_message = message;
        Ok(settings)
    }

    pub fn update_timestamp_settings(
        &self,
        mut settings: TimestampSettings,
    ) -> Result<TimestampSettings> {
        // These fields are derived server-side. A UI payload must not be able
        // to persist a misleading green status or an arbitrary test timestamp.
        let previous = self.persistence.timestamp_settings()?;
        settings.status = ExternalTimestampStatus::NotRecorded;
        settings.status_message.clear();
        settings.last_tested_at = previous.last_tested_at;
        if settings.provider == crate::model::TimestampProviderKind::Disabled {
            settings.enabled = false;
        }
        let (status, message) = external_timestamp::settings_status(
            &settings,
            self.persistence.timestamp_secret_present()?,
        );
        settings.status = status;
        settings.status_message = message;
        self.persistence.save_timestamp_settings(&settings)?;
        Ok(settings)
    }

    /// Stores a write-only Custom RFC 3161 credential outside ordinary
    /// workspace JSON. The method intentionally returns no secret value.
    pub fn update_timestamp_secret(&self, input: TimestampSecretInput) -> Result<()> {
        self.persistence
            .save_timestamp_secret(input.secret.as_deref())
    }

    pub fn test_timestamp_provider(&self) -> Result<TimestampProviderTestResult> {
        let mut settings = self.persistence.timestamp_settings()?;
        let secret = self.persistence.timestamp_secret()?;
        let result = external_timestamp::test_provider(&settings, secret.as_deref());
        settings.status = result.status;
        settings.status_message = result.message.clone();
        settings.last_tested_at = Some(result.tested_at.clone());
        self.persistence.save_timestamp_settings(&settings)?;
        Ok(result)
    }

    /// Returns only global, non-secret ACRCloud configuration. The local
    /// engine status is checked from the app-controlled sidecar; credentials
    /// are represented only by a boolean derived from the private file.
    pub fn audio_screening_settings(&self) -> Result<AudioScreeningSettings> {
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        Ok(settings)
    }

    pub fn update_audio_screening_settings(
        &self,
        mut settings: AudioScreeningSettings,
    ) -> Result<AudioScreeningSettings> {
        // Never trust UI-provided state labels, credential flags, engine data,
        // or timestamps. All four are derived in the native layer.
        let previous = self.persistence.audio_screening_settings()?;
        settings.status = AudioScreeningProviderStatus::Disabled;
        settings.status_message.clear();
        settings.credentials_configured = false;
        settings.local_engine_available = false;
        settings.local_engine_version.clear();
        settings.last_tested_at = previous.last_tested_at;
        if settings.timeout_seconds == 0 {
            settings.timeout_seconds = 30;
        }
        if let Ok(normalized) = audio_screening::normalize_acrcloud_host(&settings.host) {
            settings.host = normalized;
        }
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        self.persistence.save_audio_screening_settings(&settings)?;
        Ok(settings)
    }

    /// Stores the pair outside all serializable workspace state. No credential
    /// is returned, logged, added to a profile, or copied to a track.
    pub fn update_audio_screening_secret(&self, input: AudioScreeningSecretInput) -> Result<()> {
        self.persistence.save_audio_screening_secret(input)?;
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        audio_screening::apply_provider_configuration_status(&mut settings, credentials_present);
        self.persistence.save_audio_screening_settings(&settings)
    }

    /// This sends no audio and no credentials. It is a bounded HTTPS
    /// reachability/configuration test for the explicitly configured host.
    pub fn test_audio_screening_provider(&self) -> Result<AudioScreeningProviderTestResult> {
        let mut settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        let result = audio_screening::test_acrcloud_provider(&settings, credentials_present);
        settings.status = result.status;
        settings.status_message = result.message.clone();
        settings.credentials_configured = credentials_present;
        settings.last_tested_at = Some(result.tested_at.clone());
        audio_screening::refresh_local_engine_status(&mut settings);
        self.persistence.save_audio_screening_settings(&settings)?;
        Ok(result)
    }

    pub fn create_track(&self, input: CreateTrackInput) -> Result<TrackDetail> {
        validate_track_title(&input.title)?;
        let profile = self.profile()?;
        validate_profile(&profile, true)?;
        validate_optional_date("Production start", &input.production_start_date)?;
        let library = self.existing_album_spelling(normalize_track_library(input.library)?)?;
        let relative_path = physical_track_relative(&library, &input.title)?;
        if self
            .persistence
            .track_by_relative_path(&relative_path)?
            .is_some()
        {
            return Err(AppError::Collision(relative_path));
        }
        let target_relative = Path::new(&relative_path);
        let parent_relative = target_relative
            .parent()
            .ok_or_else(|| AppError::Validation("A track folder needs a parent.".into()))?;
        let parent_existed = self.root.join(parent_relative).is_dir();
        let parent = ensure_contained_directory(&self.root, parent_relative)?;
        if parent_relative != Path::new(SINGLES_DIRECTORY) && looks_like_track_root(&parent) {
            return Err(AppError::Collision(portable_relative(parent_relative)));
        }
        let target = contained_path(&self.root, target_relative, false)?;
        if target.exists() {
            return Err(AppError::Collision(relative_path));
        }
        if let Err(error) = (|| -> Result<()> {
            fs::create_dir(&target).map_err(|error| AppError::io(&target, error))?;
            for folder in TRACK_FOLDERS {
                ensure_contained_directory(&target, Path::new(folder))?;
            }
            Ok(())
        })() {
            let _ = fs::remove_dir_all(&target);
            if !parent_existed {
                let _ = fs::remove_dir(&parent);
            }
            return Err(error);
        }
        let now = now();
        let fields = crate::model::TrackFields {
            title: input.title.trim().to_owned(),
            production_start_date: input.production_start_date,
            commercial_use_intended: input.commercial_use_intended,
            ai_image_service: profile.default_ai_image_service.clone(),
            disclosure_text: profile.disclosure_text.clone(),
            ..Default::default()
        };
        validate_track_fields(&fields)?;
        let config = workflow::config()?;
        let track = TrackRecord {
            id: Uuid::new_v4().to_string(),
            relative_path,
            status: TrackStatus::Draft,
            workflow_id: config.id,
            workflow_version: config.version,
            profile_snapshot: profile,
            library,
            field_origins: Default::default(),
            fields,
            audio_screening: Default::default(),
            documents: DocumentState::default(),
            integrity: IntegrityState::default(),
            certificate: CertificateState::default(),
            created_at: now.clone(),
            updated_at: now,
            legacy: false,
        };
        if let Err(error) = self.persistence.save_track(&track) {
            let _ = fs::remove_dir_all(&target);
            if !parent_existed {
                let _ = fs::remove_dir(&parent);
            }
            return Err(error);
        }
        if let Err(error) = self.write_track_identity(&track) {
            let _ = self.persistence.delete_track(&track.id);
            let _ = fs::remove_dir_all(&target);
            if !parent_existed {
                let _ = fs::remove_dir(&parent);
            }
            return Err(error);
        }
        for global in self
            .persistence
            .global_evidence()?
            .into_iter()
            .filter(|item| item.evidence.role == EvidenceRole::SunoTermsRights)
        {
            if let Err(error) = self.attach_global_evidence(&track.id, &global.evidence.id) {
                let _ = self.persistence.delete_track(&track.id);
                let _ = fs::remove_dir_all(&target);
                if !parent_existed {
                    let _ = fs::remove_dir(&parent);
                }
                return Err(error);
            }
        }
        self.load_track(&track.id)
    }

    pub fn scan_folder_import(&self, source: &Path) -> Result<FolderImportProposal> {
        folder_import::plans(source).map(|(proposal, _)| proposal)
    }

    /// Imports only files with a unique, validated role into freshly created
    /// ordinary tracks. The source directory is never written to.
    pub fn import_folder(&self, input: FolderImportExecutionInput) -> Result<Vec<TrackDetail>> {
        let (proposal, plans) = folder_import::plans(Path::new(&input.source_path))?;
        if proposal.kind != input.expected_kind {
            return Err(AppError::Validation(
                "Der Quellordner hat sich seit der Vorschau geändert. Bitte erneut analysieren."
                    .into(),
            ));
        }
        let profile = self.profile()?;
        let commercial_use_intended = input
            .commercial_use_intended
            .unwrap_or(profile.default_commercial_use);
        let source_root = Path::new(&input.source_path)
            .canonicalize()
            .map_err(|error| AppError::io(&input.source_path, error))?;
        let mut prepared = Vec::new();
        let mut target_paths = HashSet::new();
        for plan in plans {
            let (title, production_start_date, library) = match proposal.kind {
                folder_import::FolderImportKind::Single => (
                    input
                        .single_track_title
                        .clone()
                        .unwrap_or_else(|| plan.title.clone()),
                    input.production_start_date.clone(),
                    input.single_track_library.clone().unwrap_or_default(),
                ),
                folder_import::FolderImportKind::Album => (
                    plan.title.clone(),
                    String::new(),
                    TrackLibraryPlacement {
                        section: TrackLibrarySection::Album,
                        album_title: proposal.album_title.clone(),
                    },
                ),
            };
            validate_track_title(&title)?;
            validate_optional_date("Production start", &production_start_date)?;
            let library = self.existing_album_spelling(normalize_track_library(library)?)?;
            let relative_path = physical_track_relative(&library, &title)?;
            let target = self.root.join(&relative_path);
            if target.starts_with(&source_root) {
                return Err(AppError::Validation(
                    "Der Ziel-Track würde innerhalb des ausgewählten Quellordners liegen. Wähle einen getrennten Quellordner, damit die Quelle unverändert bleibt."
                        .into(),
                ));
            }
            if !target_paths.insert(relative_path.clone())
                || target.exists()
                || self
                    .persistence
                    .track_by_relative_path(&relative_path)?
                    .is_some()
            {
                return Err(AppError::Collision(relative_path));
            }
            prepared.push((
                plan,
                CreateTrackInput {
                    title,
                    production_start_date,
                    commercial_use_intended,
                    library,
                },
            ));
        }

        let mut imported = Vec::new();
        for (plan, create_input) in prepared {
            let created = self.create_track(create_input)?;
            let mut track = created;
            for assignment in plan.assignments {
                track = self.import_evidence_with_metadata_from(
                    &track.id,
                    assignment.role,
                    &assignment.source,
                    EvidenceMetadata::default(),
                )?;
            }
            if plan.lyrics.is_some() || plan.style.is_some() || plan.has_source_code {
                let patch = TrackPatch {
                    lyrics_text: plan.lyrics,
                    suno_style_prompt: plan.style,
                    code_based_generation: plan.has_source_code.then_some(true),
                    ..Default::default()
                };
                track = self.update_track(&track.id, patch)?;
            }
            imported.push(track);
        }
        Ok(imported)
    }

    pub fn list_tracks(&self) -> Result<Vec<TrackSummary>> {
        self.reconcile_physical_library()?;
        let mut result = Vec::new();
        for track in self.persistence.tracks()? {
            if is_hidden_workspace_path(Path::new(&track.relative_path)) {
                continue;
            }
            let detail = self.detail_from_record(track, true)?;
            result.push(summary_from_detail(&detail));
        }
        result.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        Ok(result)
    }

    pub fn list_albums(&self) -> Result<Vec<String>> {
        let mut albums = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|error| AppError::io(&self.root, error))? {
            let entry = entry.map_err(|error| AppError::io(&self.root, error))?;
            let Some(title) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if title.starts_with('.') || title == SINGLES_DIRECTORY {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| AppError::io(entry.path(), error))?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || looks_like_track_root(&entry.path())
                || safe_album_directory(&title).is_err()
            {
                continue;
            }
            albums.push(title);
        }
        albums.sort_by(|left, right| {
            left.to_lowercase()
                .cmp(&right.to_lowercase())
                .then_with(|| left.cmp(right))
        });
        Ok(albums)
    }

    pub fn create_album(&self, title: &str) -> Result<Vec<String>> {
        let library = normalize_track_library(TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some(title.to_owned()),
        })?;
        let title = library.album_title.as_deref().expect("normalized album");
        if let Some(existing) = self
            .list_albums()?
            .into_iter()
            .find(|existing| existing.eq_ignore_ascii_case(title))
        {
            return Err(AppError::Collision(existing));
        }
        ensure_contained_directory(&self.root, Path::new(SINGLES_DIRECTORY))?;
        let relative = PathBuf::from(safe_album_directory(title)?);
        let album = contained_path(&self.root, &relative, false)?;
        if album.exists() {
            return Err(AppError::Collision(portable_relative(&relative)));
        }
        fs::create_dir(&album).map_err(|error| AppError::io(&album, error))?;
        self.list_albums()
    }

    pub fn load_track(&self, id: &str) -> Result<TrackDetail> {
        self.reconcile_physical_library()?;
        let track = self.persistence.track(id)?;
        if is_hidden_workspace_path(Path::new(&track.relative_path)) {
            return Err(AppError::TrackNotFound(id.to_owned()));
        }
        self.detail_from_record(track, true)
    }

    pub fn update_track(&self, id: &str, patch: TrackPatch) -> Result<TrackDetail> {
        self.update_track_with_explicit_nulls(id, patch, &[])
    }

    pub fn update_track_request(
        &self,
        id: &str,
        request: TrackPatchRequest,
    ) -> Result<TrackDetail> {
        self.update_track_with_explicit_nulls(id, request.patch, &request.explicit_null_fields)
    }

    fn update_track_with_explicit_nulls(
        &self,
        id: &str,
        patch: TrackPatch,
        explicit_null_fields: &[String],
    ) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let previous_path = track.relative_path.clone();
        let previous_fields = track.fields.clone();
        let previous_origins = track.field_origins.clone();
        let terms_unavailable_requested = patch.suno_terms_evidence_not_available == Some(true);
        apply_patch_with_explicit_nulls(&mut track.fields, patch, explicit_null_fields);
        let evidence = self.persistence.evidence(id)?;
        if terms_unavailable_requested
            && evidence.iter().any(|item| {
                item.role == EvidenceRole::SunoTermsRights
                    && item.verified
                    && item.sha256.is_some()
                    && item.verification_error.is_none()
            })
        {
            return Err(AppError::Validation(
                "Terms evidence cannot be marked unavailable while a verified local Terms evidence file is attached."
                    .into(),
            ));
        }
        reconcile_evidence_derived_fields(&mut track, &evidence);
        if track.fields.title != previous_fields.title {
            track.fields.release_filename_difference_confirmed = None;
            track.fields.suno_export_filename_difference_confirmed = None;
        }
        validate_track_fields(&track.fields)?;
        let mut authoritative_release_renamed = false;
        if track.fields != previous_fields || track.field_origins != previous_origins {
            let mut release_renames = Vec::new();
            if track.fields.title != previous_fields.title {
                let target = physical_track_relative(&track.library, &track.fields.title)?;
                self.move_track_directory(&previous_path, &target, false)?;
                track.relative_path = target;
                match self.rename_managed_release_evidence(&track, &track.fields.title) {
                    Ok(renamed) => release_renames = renamed,
                    Err(error) => {
                        if let Err(rollback) =
                            self.rollback_track_move(&track.relative_path, &previous_path)
                        {
                            return Err(AppError::Data(format!(
                                "Release rename failed ({error}); folder rollback failed: {rollback}"
                            )));
                        }
                        return Err(error);
                    }
                }
            }
            authoritative_release_renamed = release_renames
                .iter()
                .any(|(_, updated)| updated.role == EvidenceRole::ReleaseWav);
            if authoritative_release_renamed {
                // A managed filename is part of the screening source binding.
                // The bytes may be identical, but an old path must never keep
                // a prior fingerprint or provider result current.
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            track.status = TrackStatus::Active;
            track.updated_at = now();
            if let Err(error) = self.persistence.save_track(&track) {
                let release_rollback =
                    self.rollback_release_evidence_renames(&track, &release_renames);
                if track.relative_path != previous_path {
                    if let Err(rollback) =
                        self.rollback_track_move(&track.relative_path, &previous_path)
                    {
                        return Err(AppError::Data(format!(
                            "Track update failed ({error}); folder rollback failed: {rollback}"
                        )));
                    }
                }
                if let Err(rollback) = release_rollback {
                    return Err(AppError::Data(format!(
                        "Track update failed ({error}); release rollback failed: {rollback}"
                    )));
                }
                return Err(error);
            }
            self.write_track_identity(&track)?;
        }
        if authoritative_release_renamed {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        let detail = self.detail_from_record(track, false)?;
        if authoritative_release_renamed {
            return self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                // The rename itself is already durable. If a local sidecar
                // cannot be produced immediately, persist STALE rather than
                // presenting the old binding as current.
                .or(Ok(detail));
        }
        Ok(detail)
    }

    pub fn update_track_library(
        &self,
        id: &str,
        input: TrackLibraryPlacement,
    ) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let library = self.existing_album_spelling(normalize_track_library(input)?)?;
        let previous_path = track.relative_path.clone();
        let target_path = physical_track_relative(&library, &track.fields.title)?;
        if previous_path != target_path {
            self.move_track_directory(&previous_path, &target_path, false)?;
            track.relative_path = target_path;
        }
        if track.library != library || track.relative_path != previous_path {
            track.library = library;
            if let Err(error) = self.persistence.save_track(&track) {
                if track.relative_path != previous_path {
                    if let Err(rollback) =
                        self.rollback_track_move(&track.relative_path, &previous_path)
                    {
                        return Err(AppError::Data(format!(
                            "Library update failed ({error}); folder rollback failed: {rollback}"
                        )));
                    }
                }
                return Err(error);
            }
            self.write_track_identity(&track)?;
        }
        self.detail_from_stored_record(track)
    }

    pub fn rename_album(&self, old_title: &str, new_title: &str) -> Result<Vec<TrackSummary>> {
        self.reconcile_physical_library()?;
        let old_library = normalize_track_library(TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some(old_title.to_owned()),
        })?;
        let new_library = normalize_track_library(TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some(new_title.to_owned()),
        })?;
        let requested_old_title = old_library
            .album_title
            .as_deref()
            .expect("normalized album");
        let old_title = self
            .list_albums()?
            .into_iter()
            .find(|existing| existing.eq_ignore_ascii_case(requested_old_title))
            .ok_or_else(|| {
                AppError::Validation(format!("Album not found: {requested_old_title}"))
            })?;
        let new_title = new_library
            .album_title
            .as_deref()
            .expect("normalized album");
        if old_title == new_title {
            return self.list_tracks();
        }

        let mut tracks = self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| {
                track.library.section == TrackLibrarySection::Album
                    && track
                        .library
                        .album_title
                        .as_deref()
                        .is_some_and(|title| title.eq_ignore_ascii_case(&old_title))
            })
            .collect::<Vec<_>>();

        let source_relative = PathBuf::from(safe_album_directory(&old_title)?);
        let target_relative = PathBuf::from(safe_album_directory(new_title)?);
        let source = contained_path(&self.root, &source_relative, true)?;
        let target = contained_path(&self.root, &target_relative, false)?;
        if target.exists()
            && fs::canonicalize(&target).map_err(|error| AppError::io(&target, error))?
                != fs::canonicalize(&source).map_err(|error| AppError::io(&source, error))?
        {
            return Err(AppError::Collision(portable_relative(&target_relative)));
        }
        for track in &tracks {
            let relative = Path::new(&track.relative_path);
            if !relative.starts_with(&source_relative) || relative.components().count() != 2 {
                return Err(AppError::Validation(format!(
                    "Track {} is not stored inside album folder {}.",
                    track.fields.title, old_title
                )));
            }
        }

        fs::rename(&source, &target).map_err(|error| AppError::io(&target, error))?;
        for track in &mut tracks {
            let leaf = Path::new(&track.relative_path)
                .file_name()
                .ok_or_else(|| AppError::Data("Stored track path has no folder name.".into()))?;
            track.relative_path = portable_relative(&target_relative.join(leaf));
            track.library = new_library.clone();
        }
        if let Err(error) = self.persistence.save_tracks(&tracks) {
            if let Err(rollback) = fs::rename(&target, &source) {
                return Err(AppError::Data(format!(
                    "Album rename failed ({error}); folder rollback failed: {rollback}"
                )));
            }
            return Err(error);
        }
        for track in &tracks {
            self.write_track_identity(track)?;
        }
        self.list_tracks()
    }

    pub fn adopt_legacy_profile(&self, id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        if !track.legacy {
            return Err(AppError::Validation(
                "Only an imported legacy track needs profile-snapshot adoption.".into(),
            ));
        }
        let profile = self.profile()?;
        validate_profile(&profile, true)?;
        track.profile_snapshot = profile;
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn set_step_status(
        &self,
        id: &str,
        step_id: &str,
        status: StepStatus,
        na_reason: Option<String>,
    ) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        match status {
            StepStatus::NotApplicable => {
                let reason = na_reason
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| AppError::Validation("N/A requires a reason.".into()))?;
                if !workflow::can_mark_na(step_id, &track)? {
                    return Err(AppError::Validation(
                        "This step has applicable mandatory requirements and cannot be N/A.".into(),
                    ));
                }
                validate_short_text("N/A reason", &reason, 1000, true)?;
                self.persistence.save_step(
                    id,
                    &StepState {
                        id: step_id.into(),
                        status,
                        na_reason: Some(reason.trim().into()),
                        updated_at: Some(now()),
                    },
                )?;
            }
            StepStatus::Fail | StepStatus::Blocked | StepStatus::NotVerified => {
                self.persistence.save_step(
                    id,
                    &StepState {
                        id: step_id.into(),
                        status,
                        na_reason,
                        updated_at: Some(now()),
                    },
                )?;
            }
            StepStatus::NotRun | StepStatus::Pass => self.persistence.clear_step(id, step_id)?,
        }
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn add_deviation(&self, id: &str, input: DeviationInput) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        validate_short_text("Deviation", &input.description, 4000, true)?;
        let deviation = BlockingDeviation {
            id: Uuid::new_v4().to_string(),
            title: if input.blocking {
                "Blocking deviation"
            } else {
                "Note"
            }
            .into(),
            description: input.description.trim().into(),
            blocking: input.blocking,
            resolved: false,
            created_at: now(),
            resolved_at: None,
        };
        self.persistence.save_deviation(id, &deviation)?;
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn resolve_deviation(&self, id: &str, deviation_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let mut deviation = self
            .persistence
            .deviations(id)?
            .into_iter()
            .find(|value| value.id == deviation_id)
            .ok_or_else(|| AppError::Validation("Deviation not found.".into()))?;
        deviation.resolved = true;
        deviation.resolved_at = Some(now());
        self.persistence.save_deviation(id, &deviation)?;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn remove_deviation(&self, id: &str, deviation_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        self.persistence.remove_deviation(id, deviation_id)?;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn global_evidence(&self) -> Result<Vec<GlobalEvidenceItem>> {
        self.persistence.global_evidence()
    }

    pub fn register_global_evidence(
        &self,
        role: EvidenceRole,
        source: &Path,
        coverage_start: Option<String>,
        coverage_end: Option<String>,
    ) -> Result<GlobalEvidenceItem> {
        if role == EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Register the Suno terms PDF with the dedicated global importer.".into(),
            ));
        }
        if role == EvidenceRole::SubscriptionPayment {
            let start = coverage_start.as_deref().ok_or_else(|| {
                AppError::Validation("Subscription coverage start is required.".into())
            })?;
            let end = coverage_end.as_deref().ok_or_else(|| {
                AppError::Validation("Subscription coverage end is required.".into())
            })?;
            validate_date_range("Subscription coverage", start, end)?;
        } else {
            if let Some(start) = coverage_start.as_deref() {
                validate_optional_date("Evidence coverage start", start)?;
            }
            if let Some(end) = coverage_end.as_deref() {
                validate_optional_date("Evidence coverage end", end)?;
            }
        }
        let mut item = evidence::register_global(&self.root, role, source)?;
        let mut metadata =
            evidence::capture_automatic_metadata(source, EvidenceMetadata::default());
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        item.evidence.metadata = metadata;
        item.evidence.coverage_start = coverage_start;
        item.evidence.coverage_end = coverage_end;
        if let Err(error) = self.persistence.save_global_evidence(&item) {
            if let Ok(path) =
                contained_path(&self.root, Path::new(&item.evidence.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        Ok(item)
    }

    pub fn register_global_terms_evidence(
        &self,
        source: &Path,
        mut metadata: EvidenceMetadata,
    ) -> Result<GlobalEvidenceItem> {
        validate_evidence_metadata(&EvidenceRole::SunoTermsRights, &metadata)?;
        let mut item =
            evidence::register_global(&self.root, EvidenceRole::SunoTermsRights, source)?;
        metadata = evidence::capture_automatic_metadata(source, metadata);
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        item.evidence.metadata = metadata;
        if let Err(error) = self.persistence.save_global_evidence(&item) {
            if let Ok(path) =
                contained_path(&self.root, Path::new(&item.evidence.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }

        for track in self.persistence.tracks()?.into_iter().filter(|track| {
            !is_hidden_workspace_path(Path::new(&track.relative_path))
                && !matches!(
                    track.status,
                    TrackStatus::Finalized | TrackStatus::Superseded
                )
        }) {
            self.attach_global_evidence(&track.id, &item.evidence.id)?;
        }
        Ok(item)
    }

    pub fn update_global_terms_evidence_metadata(
        &self,
        evidence_id: &str,
        supplied: EvidenceMetadata,
    ) -> Result<GlobalEvidenceItem> {
        let mut global = self.persistence.global_evidence_item(evidence_id)?;
        if global.evidence.role != EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Only Suno terms/rights evidence has editable descriptive metadata here.".into(),
            ));
        }
        apply_descriptive_evidence_metadata(&mut global.evidence.metadata, &supplied);
        validate_evidence_metadata(&EvidenceRole::SunoTermsRights, &global.evidence.metadata)?;

        let mut copies = Vec::new();
        for mut track in self.persistence.tracks()?.into_iter().filter(|track| {
            !is_hidden_workspace_path(Path::new(&track.relative_path))
                && !matches!(
                    track.status,
                    TrackStatus::Finalized | TrackStatus::Superseded
                )
        }) {
            let Some(mut copy) = self
                .persistence
                .evidence(&track.id)?
                .into_iter()
                .find(|item| {
                    item.role == EvidenceRole::SunoTermsRights
                        && item.provenance == EvidenceProvenance::GlobalCopy
                        && item.source_global_evidence_id.as_deref() == Some(evidence_id)
                })
            else {
                continue;
            };
            copy.metadata = global.evidence.metadata.clone();
            mark_content_changed(&mut track);
            track.status = TrackStatus::Active;
            track.updated_at = now();
            copies.push((track, copy));
        }
        self.persistence
            .save_global_evidence_and_copies(&global, &copies)?;
        Ok(global)
    }

    pub fn register_global_evidence_for_billing_cycle(
        &self,
        role: EvidenceRole,
        source: &Path,
        coverage_start: &str,
        billing_cycle: SubscriptionBillingCycle,
    ) -> Result<GlobalEvidenceItem> {
        if role != EvidenceRole::SubscriptionPayment {
            return Err(AppError::Validation(
                "A billing cycle can only be used for subscription/payment evidence.".into(),
            ));
        }
        let coverage_end = subscription_coverage_end(coverage_start, billing_cycle)?;
        self.register_global_evidence(
            role,
            source,
            Some(coverage_start.to_owned()),
            Some(coverage_end),
        )
    }

    pub fn remove_global_evidence(&self, evidence_id: &str) -> Result<()> {
        let item = self.persistence.global_evidence_item(evidence_id)?;
        let path = contained_path(&self.root, Path::new(&item.evidence.relative_path), false)?;
        let mut removal_dir = None;
        let mut archived = None;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let removal_relative =
                    PathBuf::from(".suno-doc/removals").join(Uuid::new_v4().to_string());
                let directory = ensure_contained_directory(&self.root, &removal_relative)?;
                let archive_path = directory.join(
                    path.file_name()
                        .ok_or_else(|| AppError::Data("Evidence path has no file name.".into()))?,
                );
                fs::rename(&path, &archive_path).map_err(|error| AppError::io(&path, error))?;
                removal_dir = Some(directory);
                archived = Some(archive_path);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::Symlink(path.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::io(&path, error)),
        }
        if let Err(error) = self.persistence.remove_global_evidence(evidence_id) {
            return Err(match archived.as_deref() {
                Some(archive_path) => rollback_removed_file(archive_path, &path, error),
                None => error,
            });
        }
        if let Some(archive_path) = archived.as_deref() {
            let _ = fs::remove_file(archive_path);
        }
        if let Some(directory) = removal_dir.as_deref() {
            let _ = fs::remove_dir(directory);
        }
        Ok(())
    }

    pub fn attach_global_evidence(&self, id: &str, evidence_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let global = self.persistence.global_evidence_item(evidence_id)?;
        if self.persistence.evidence(id)?.iter().any(|item| {
            item.source_global_evidence_id.as_deref() == Some(global.evidence.id.as_str())
        }) {
            return self.detail_from_record(track, false);
        }

        if global.evidence.role == EvidenceRole::SubscriptionPayment {
            validate_required_production_range(&track)?;
            let start = global.evidence.coverage_start.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage start.".into())
            })?;
            let end = global.evidence.coverage_end.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage end.".into())
            })?;
            validate_date_range("Subscription coverage", start, end)?;
            let overlaps_production = start <= track.fields.production_end_date.as_str()
                && end >= track.fields.production_start_date.as_str();
            let covers_generation = !track.fields.suno_final_generation_date.trim().is_empty()
                && start <= track.fields.suno_final_generation_date.as_str()
                && end >= track.fields.suno_final_generation_date.as_str();
            if !overlaps_production && !covers_generation {
                return Err(AppError::Validation(
                    "The selected subscription evidence neither overlaps the recorded production period nor covers the recorded final-generation date."
                        .into(),
                ));
            }
        } else if global.evidence.role != EvidenceRole::SunoTermsRights {
            return Err(AppError::Validation(
                "Only subscription/payment or Suno terms/rights evidence can be attached to a track."
                    .into(),
            ));
        }
        let source = contained_path(&self.root, Path::new(&global.evidence.relative_path), true)?;
        let track_root = self.track_root(&track)?;
        let item = evidence::portable_global_copy(&track_root, &global, &source)?;
        if let Err(error) = self.persistence.save_evidence(id, &item) {
            if let Ok(path) = contained_path(&track_root, Path::new(&item.relative_path), true) {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        let all_evidence = self.persistence.evidence(id)?;
        reconcile_evidence_derived_fields(&mut track, &all_evidence);
        mark_content_changed(&mut track);
        if global.evidence.role == EvidenceRole::SunoTermsRights {
            track.fields.suno_terms_evidence_not_available = Some(false);
        }
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    #[cfg(test)]
    pub fn import_evidence_from(
        &self,
        id: &str,
        role: EvidenceRole,
        source: &Path,
    ) -> Result<TrackDetail> {
        self.import_evidence_with_metadata_from(id, role, source, EvidenceMetadata::default())
    }

    pub fn import_evidence_with_metadata_from(
        &self,
        id: &str,
        role: EvidenceRole,
        source: &Path,
        mut metadata: EvidenceMetadata,
    ) -> Result<TrackDetail> {
        if role == EvidenceRole::ExternalTimestamp {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached after technical finalization."
                    .into(),
            ));
        }
        if matches!(
            role,
            EvidenceRole::SubscriptionPayment | EvidenceRole::SunoTermsRights
        ) {
            return Err(AppError::Validation(
                "Register subscription and Suno terms/rights evidence globally in Settings and attach a portable copy."
                    .into(),
            ));
        }
        let mut track = self.mutable_track(id)?;
        if matches!(
            role,
            EvidenceRole::ReleaseWav | EvidenceRole::SunoFinalExport | EvidenceRole::FinalArtwork
        ) && self
            .persistence
            .evidence(id)?
            .iter()
            .any(|item| item.role == role)
        {
            return Err(AppError::Validation(format!(
                "Die Evidence-Rolle '{}' ist bereits belegt. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen.",
                role.as_str()
            )));
        }
        let track_root = self.track_root(&track)?;
        let planned_relative = evidence::managed_relative_path(&track.fields.title, &role, source)?;
        let planned_portable = portable_relative(&planned_relative);
        if self
            .persistence
            .evidence_by_relative_path(id, &planned_portable)?
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "Unter {planned_portable} ist bereits Evidence registriert. Verwende den Upload-Button an der vorhandenen Evidence zum sicheren Ersetzen."
            )));
        }
        let original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let mut item = evidence::import(&track_root, &track.fields.title, role, source)?;
        let managed_path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        let persist_result = (|| -> Result<()> {
            metadata = evidence::capture_automatic_metadata(&managed_path, metadata);
            metadata.original_file_name = original_file_name;
            validate_evidence_metadata(&role, &metadata)?;
            item.metadata = metadata;

            let mut all_evidence = self.persistence.evidence(id)?;
            all_evidence.push(item.clone());
            reconcile_evidence_derived_fields(&mut track, &all_evidence);
            if role == EvidenceRole::ReleaseWav {
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            if role == EvidenceRole::ReleaseWav {
                track.fields.release_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoFinalExport {
                track.fields.suno_export_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoTermsRights {
                track.fields.suno_terms_evidence_not_available = Some(false);
            }
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence
                .save_track_and_evidence(&track, std::slice::from_ref(&item))
        })();
        if let Err(error) = persist_result {
            let _ = fs::remove_file(&managed_path);
            return Err(error);
        }
        if role == EvidenceRole::ReleaseWav {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        let detail = self.detail_from_record(track, false)?;
        if role == EvidenceRole::ReleaseWav {
            return self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                // The evidence import remains durable if a subsequent local
                // best-effort run hits an unexpected filesystem error. The
                // unfulfilled workflow requirement then makes the condition
                // visible rather than presenting an old fingerprint as PASS.
                .or(Ok(detail));
        }
        Ok(detail)
    }

    #[cfg(test)]
    pub fn replace_evidence_from(
        &self,
        id: &str,
        evidence_id: &str,
        role: EvidenceRole,
        source: &Path,
    ) -> Result<TrackDetail> {
        self.replace_evidence_with_metadata_from(
            id,
            evidence_id,
            role,
            source,
            EvidenceMetadata::default(),
        )
    }

    pub fn replace_evidence_with_metadata_from(
        &self,
        id: &str,
        evidence_id: &str,
        role: EvidenceRole,
        source: &Path,
        metadata: EvidenceMetadata,
    ) -> Result<TrackDetail> {
        if role == EvidenceRole::ExternalTimestamp {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached after technical finalization."
                    .into(),
            ));
        }
        if matches!(
            role,
            EvidenceRole::SubscriptionPayment | EvidenceRole::SunoTermsRights
        ) {
            return Err(AppError::Validation(
                "Replace subscription and Suno terms/rights evidence in the global evidence register."
                    .into(),
            ));
        }
        let mut track = self.mutable_track(id)?;
        let previous = self.persistence.evidence_item(id, evidence_id)?;
        if previous.role != role {
            return Err(AppError::Validation(
                "The selected replacement role does not match the existing evidence.".into(),
            ));
        }
        let planned_relative = evidence::managed_relative_path(&track.fields.title, &role, source)?;
        let planned_portable = portable_relative(&planned_relative);
        if self
            .persistence
            .evidence_by_relative_path(id, &planned_portable)?
            .is_some_and(|item| item.id != previous.id)
        {
            return Err(AppError::Validation(format!(
                "Another evidence record already uses {planned_portable}. Remove that record before replacing this file."
            )));
        }
        let track_root = self.track_root(&track)?;
        let original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        let track_title = track.fields.title.clone();
        evidence::replace(&track_root, &track_title, role, source, &previous, |item| {
            let mut item = item.clone();
            let managed_path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
            let mut captured =
                evidence::capture_automatic_metadata(&managed_path, metadata.clone());
            captured.original_file_name = original_file_name.clone();
            validate_evidence_metadata(&role, &captured)?;
            item.metadata = captured;

            let mut all_evidence = self.persistence.evidence(id)?;
            let stored = all_evidence
                .iter_mut()
                .find(|stored| stored.id == item.id)
                .ok_or_else(|| AppError::EvidenceNotFound(item.id.clone()))?;
            *stored = item.clone();
            reconcile_evidence_derived_fields(&mut track, &all_evidence);
            if role == EvidenceRole::ReleaseWav {
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            if role == EvidenceRole::ReleaseWav {
                track.fields.release_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoFinalExport {
                track.fields.suno_export_filename_difference_confirmed = None;
            } else if role == EvidenceRole::SunoTermsRights {
                track.fields.suno_terms_evidence_not_available = Some(false);
            }
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence
                .save_track_and_evidence(&track, std::slice::from_ref(&item))
        })?;
        if role == EvidenceRole::ReleaseWav {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        let detail = self.detail_from_record(track, false)?;
        if role == EvidenceRole::ReleaseWav {
            return self
                .run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .or(Ok(detail));
        }
        Ok(detail)
    }

    pub fn remove_evidence(&self, id: &str, evidence_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let item = self.persistence.evidence_item(id, evidence_id)?;
        let removed_authoritative_release = item.role == EvidenceRole::ReleaseWav;
        if item.provenance == EvidenceProvenance::IndexedLegacy {
            let track_root = self.track_root(&track)?;
            let path = contained_path(&track_root, Path::new(&item.relative_path), false)?;
            let removal_id = Uuid::new_v4().to_string();
            let removal_relative = PathBuf::from(".archive/removals").join(&removal_id);
            let directory = ensure_contained_directory(&track_root, &removal_relative)?;
            let archived =
                directory.join(path.file_name().ok_or_else(|| {
                    AppError::Data("Legacy evidence path has no file name.".into())
                })?);
            let moved = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(AppError::Symlink(path.display().to_string()));
                }
                Ok(metadata) if metadata.is_file() => {
                    fs::rename(&path, &archived).map_err(|error| AppError::io(&path, error))?;
                    true
                }
                Ok(_) => {
                    return Err(AppError::Validation(
                        "Indexed legacy evidence is not a regular file.".into(),
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(AppError::io(&path, error)),
            };
            let metadata = serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "removal_id": removal_id,
                "track_id": track.id,
                "removed_at": now(),
                "reason": "legacy evidence removed from the application index",
                "original_relative_path": item.relative_path,
                "evidence": item,
            }))?;
            if let Err(error) = atomic_write_new(&directory.join("removal.json"), &metadata) {
                if moved {
                    let _ = fs::rename(&archived, &path);
                }
                let _ = fs::remove_dir(&directory);
                return Err(error);
            }
            if let Err(error) = self.persistence.remove_evidence(id, evidence_id) {
                if moved {
                    let _ = fs::rename(&archived, &path);
                }
                let _ = fs::remove_file(directory.join("removal.json"));
                let _ = fs::remove_dir(&directory);
                return Err(error);
            }
            let remaining_evidence = self.persistence.evidence(id)?;
            reconcile_evidence_derived_fields(&mut track, &remaining_evidence);
            if item.role == EvidenceRole::ReleaseWav {
                audio_screening::mark_screening_stale(&mut track.audio_screening);
            }
            mark_content_changed(&mut track);
            if item.role == EvidenceRole::ReleaseWav {
                track.fields.release_filename_difference_confirmed = None;
            } else if item.role == EvidenceRole::SunoFinalExport {
                track.fields.suno_export_filename_difference_confirmed = None;
            }
            track.updated_at = now();
            if let Err(error) = self.persistence.save_track(&track) {
                let database_rollback = self.persistence.save_evidence(id, &item);
                let file_rollback = if moved {
                    fs::rename(&archived, &path)
                } else {
                    Ok(())
                };
                if database_rollback.is_ok() && file_rollback.is_ok() {
                    let _ = fs::remove_file(directory.join("removal.json"));
                    let _ = fs::remove_dir(&directory);
                    return Err(error);
                }
                return Err(AppError::Data(format!(
                    "Legacy evidence removal failed ({error}); rollback was incomplete."
                )));
            }
            if removed_authoritative_release {
                self.archive_current_audio_screening_artifacts(&track)?;
            }
            return self.detail_from_record(track, false);
        }
        let track_root = self.track_root(&track)?;
        let path = contained_path(&track_root, Path::new(&item.relative_path), false)?;
        let mut removal_dir = None;
        let mut archived = None;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let removal_relative =
                    PathBuf::from(".archive/removals").join(Uuid::new_v4().to_string());
                let directory = ensure_contained_directory(&track_root, &removal_relative)?;
                let archive_path = directory.join(
                    path.file_name()
                        .ok_or_else(|| AppError::Data("Evidence path has no file name.".into()))?,
                );
                fs::rename(&path, &archive_path).map_err(|error| AppError::io(&path, error))?;
                removal_dir = Some(directory);
                archived = Some(archive_path);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AppError::Symlink(path.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(AppError::io(&path, error)),
        }
        if let Err(error) = self.persistence.remove_evidence(id, evidence_id) {
            return Err(match archived.as_deref() {
                Some(archive_path) => rollback_removed_file(archive_path, &path, error),
                None => error,
            });
        }
        let remaining_evidence = self.persistence.evidence(id)?;
        reconcile_evidence_derived_fields(&mut track, &remaining_evidence);
        if item.role == EvidenceRole::ReleaseWav {
            audio_screening::mark_screening_stale(&mut track.audio_screening);
        }
        mark_content_changed(&mut track);
        if item.provenance == EvidenceProvenance::GeneratedDisclosure {
            track.fields.disclosure_applied = None;
        }
        if item.role == EvidenceRole::ReleaseWav {
            track.fields.release_filename_difference_confirmed = None;
        } else if item.role == EvidenceRole::SunoFinalExport {
            track.fields.suno_export_filename_difference_confirmed = None;
        }
        track.updated_at = now();
        if let Err(error) = self.persistence.save_track(&track) {
            let database_rollback = self.persistence.save_evidence(id, &item);
            let file_rollback = archived
                .as_deref()
                .map_or(Ok(()), |archive_path| fs::rename(archive_path, &path));
            if let (Ok(()), Ok(())) = (database_rollback, file_rollback) {
                if let Some(directory) = removal_dir.as_deref() {
                    let _ = fs::remove_dir(directory);
                }
                return Err(error);
            }
            return Err(AppError::Data(format!(
                "Evidence removal failed ({error}); rollback was incomplete."
            )));
        }
        if let Some(archive_path) = archived.as_deref() {
            let _ = fs::remove_file(archive_path);
        }
        if let Some(directory) = removal_dir.as_deref() {
            let _ = fs::remove_dir(directory);
        }
        if removed_authoritative_release {
            self.archive_current_audio_screening_artifacts(&track)?;
        }
        self.detail_from_record(track, false)
    }

    pub fn preview_evidence(&self, id: &str, evidence_id: &str) -> Result<EvidencePreview> {
        const IMAGE_PREVIEW_LIMIT: u64 = 16 * 1024 * 1024;
        const TEXT_PREVIEW_LIMIT: u64 = 512 * 1024;

        let track = self.persistence.track(id)?;
        let item = self.persistence.evidence_item(id, evidence_id)?;
        let track_root = self.track_root(&track)?;
        let path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        let metadata = fs::symlink_metadata(&path).map_err(|error| AppError::io(&path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::Validation(
                "Evidence preview requires a regular managed file.".into(),
            ));
        }
        evidence::validate_type(&item.role, &path)?;
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let image_mime = match extension.as_str() {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "webp" => Some("image/webp"),
            _ => None,
        };
        let text_mime = match extension.as_str() {
            "txt" => Some("text/plain"),
            "md" => Some("text/markdown"),
            "json" => Some("application/json"),
            _ if item.role == EvidenceRole::SourceCodeFile => Some("text/plain"),
            _ => None,
        };
        let (mime_type, data_url, text_content, message) = if let Some(mime) = image_mime {
            if metadata.len() <= IMAGE_PREVIEW_LIMIT {
                let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                (
                    Some(mime.into()),
                    Some(format!("data:{mime};base64,{encoded}")),
                    None,
                    None,
                )
            } else {
                (
                    Some(mime.into()),
                    None,
                    None,
                    Some("Das Bild ist größer als 16 MB und wird deshalb nicht in den Arbeitsspeicher geladen.".into()),
                )
            }
        } else if let Some(mime) = text_mime {
            if metadata.len() <= TEXT_PREVIEW_LIMIT {
                let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
                let text = String::from_utf8_lossy(&bytes).into_owned();
                (Some(mime.into()), None, Some(text), None)
            } else {
                (
                    Some(mime.into()),
                    None,
                    None,
                    Some("Die Textdatei ist größer als 512 KB und wird deshalb nicht vollständig geladen.".into()),
                )
            }
        } else {
            let message = if extension == "zip" {
                "ZIP-Dateien werden für die Vorschau nicht entpackt oder in den Arbeitsspeicher geladen."
            } else {
                "Für diesen Dateityp ist keine sichere Vorschau innerhalb der App verfügbar."
            };
            (None, None, None, Some(message.into()))
        };
        Ok(EvidencePreview {
            evidence_id: item.id,
            role: item.role,
            file_name: item.file_name,
            relative_path: item.relative_path,
            size_bytes: metadata.len(),
            mime_type,
            data_url,
            text_content,
            message,
        })
    }

    pub fn track_cover(&self, id: &str) -> Result<Option<TrackCoverPreview>> {
        let track = self.persistence.track(id)?;
        let Some(item) = self.persistence.evidence(id)?.into_iter().find(|item| {
            item.role == EvidenceRole::FinalArtwork
                && item.verified
                && item.sha256.is_some()
                && item.verification_error.is_none()
        }) else {
            return Ok(None);
        };
        let track_root = self.track_root(&track)?;
        let path = contained_path(&track_root, Path::new(&item.relative_path), true)?;
        evidence::validate_type(&item.role, &path)?;
        let encoded = crate::artwork::centered_cover_thumbnail(&path)?;
        Ok(Some(TrackCoverPreview {
            evidence_id: item.id,
            data_url: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(encoded)
            ),
        }))
    }

    pub fn verify_evidence(&self, id: &str, evidence_id: Option<&str>) -> Result<TrackDetail> {
        let mut track = self.persistence.track(id)?;
        let track_root = self.track_root(&track)?;
        let items = self.persistence.evidence(id)?;
        if let Some(wanted) = evidence_id {
            if !items.iter().any(|item| item.id == wanted) {
                return Err(AppError::EvidenceNotFound(wanted.into()));
            }
        }
        let mut mismatch = false;
        let mut authoritative_release_mismatch = false;
        for item in items {
            if evidence_id.is_none() || evidence_id == Some(item.id.as_str()) {
                let is_authoritative_release = item.role == EvidenceRole::ReleaseWav;
                let verified = evidence::verify(&track_root, item)?;
                mismatch |= !verified.verified;
                authoritative_release_mismatch |= is_authoritative_release && !verified.verified;
                self.persistence.save_evidence(id, &verified)?;
            }
        }
        if authoritative_release_mismatch
            && !matches!(
                track.status,
                TrackStatus::Finalized | TrackStatus::Superseded
            )
        {
            // The byte-integrity verifier discovered that the authoritative
            // source no longer matches the recorded release.  Preserve old
            // screening artifacts below `.archive` and make both levels
            // visibly stale; do not leave a prior positive record live.
            audio_screening::mark_screening_stale(&mut track.audio_screening);
            mark_content_changed(&mut track);
            track.status = TrackStatus::Active;
            track.updated_at = now();
            self.persistence.save_track(&track)?;
            self.archive_current_audio_screening_artifacts(&track)?;
        } else if mismatch && track.status == TrackStatus::Finalized {
            invalidate_state(
                &mut track,
                "Evidence integrity mismatch detected after finalization",
            );
            self.persistence.save_track(&track)?;
        }
        self.detail_from_record(track, false)
    }

    pub fn preview_documents(&self, id: &str) -> Result<DocumentPreview> {
        let track = self.persistence.track(id)?;
        documents::preview(&self.track_root(&track)?)
    }

    #[cfg(test)]
    pub fn generate_documents(&self, id: &str, adopt_existing: bool) -> Result<ActionResult> {
        self.generate_documents_with_progress(id, adopt_existing, &mut |_| {})
    }

    pub fn generate_documents_with_progress(
        &self,
        id: &str,
        adopt_existing: bool,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        on_progress(OperationProgress {
            stage: "preparing_documents".into(),
            ..OperationProgress::default()
        });
        let mut track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
        validate_track_fields(&track.fields)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
        self.ensure_current_audio_screening_artifacts(&track, &evidence)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let track_root = self.track_root(&track)?;
        let files = documents::generate_with_progress(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
            adopt_existing,
            on_progress,
        )?;
        let generated_file_count = files.len() as u32;
        track.documents = DocumentState {
            generated: true,
            current: true,
            generated_at: Some(now()),
            template_version: documents::TEMPLATE_VERSION.into(),
            files,
            input_fingerprint: documents::input_fingerprint(
                &track,
                &track.profile_snapshot,
                &evidence,
            )?,
        };
        track.documents.current = documents::is_current(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
        )?;
        track.integrity = IntegrityState::default();
        track.status = TrackStatus::Active;
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: generated_file_count,
            total_files: generated_file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let detail = self.detail_from_record(track, false)?;
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: generated_file_count,
            total_files: generated_file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!("{} documents generated.", detail.documents.files.len()),
            track: Some(detail),
        })
    }

    pub fn generate_artwork_disclosure(
        &self,
        id: &str,
        disclosure_text: Option<String>,
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        if !matches!(
            track.fields.artwork_origin.as_str(),
            "ai_generated" | "ai_assisted"
        ) {
            return Err(AppError::Validation(
                "Visible disclosure is available only for AI-generated or AI-assisted artwork."
                    .into(),
            ));
        }
        if track.fields.depicts_real_person == Some(false)
            && track.fields.depicts_real_event == Some(false)
            && track.fields.contains_trademark == Some(false)
        {
            return Err(AppError::Validation(
                "AI Transparency ist deaktiviert, weil alle drei Content-Checks mit Nein beantwortet wurden."
                    .into(),
            ));
        }
        let evidence_items = self.verified_evidence(&track)?;
        let source = evidence_items
            .iter()
            .rev()
            .find(|item| item.role == EvidenceRole::AiArtworkOriginal)
            .ok_or_else(|| AppError::Validation("Import the AI artwork original first.".into()))?;
        let text = disclosure_text
            .as_deref()
            .unwrap_or(&track.fields.disclosure_text)
            .trim()
            .to_owned();
        if evidence_items.iter().any(|item| {
            item.role == EvidenceRole::AiArtworkEdited
                && item.provenance == EvidenceProvenance::GeneratedDisclosure
                && item.verified
                && item.verification_error.is_none()
                && item.derived_from_evidence_id.as_deref() == Some(source.id.as_str())
                && item.imported_at.as_str() >= source.imported_at.as_str()
                && item.generator_version.as_deref()
                    == Some(crate::artwork::DISCLOSURE_GENERATOR_VERSION)
                && item.generated_disclosure_text.as_deref() == Some(text.as_str())
        }) {
            if track.fields.disclosure_applied != Some(true) || track.fields.disclosure_text != text
            {
                track.fields.disclosure_applied = Some(true);
                track.fields.disclosure_text = text;
                mark_content_changed(&mut track);
                track.updated_at = now();
                self.persistence.save_track(&track)?;
            }
            return Ok(ActionResult {
                message: "Der aktuelle sichtbare KI-Hinweis ist bereits vorhanden; es wurde keine doppelte Datei erzeugt."
                    .into(),
                track: Some(self.detail_from_record(track, false)?),
            });
        }
        let track_root = self.track_root(&track)?;
        let generated =
            crate::artwork::generate_disclosure(&track_root, &track.fields.title, source, &text)?;
        if let Err(error) = self.persistence.save_evidence(id, &generated) {
            if let Ok(path) = contained_path(&track_root, Path::new(&generated.relative_path), true)
            {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        track.fields.disclosure_applied = Some(true);
        track.fields.disclosure_text = text;
        mark_content_changed(&mut track);
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: "Visible AI disclosure generated locally; select a final artwork separately."
                .into(),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    #[cfg(test)]
    pub fn calculate_hashes(&self, id: &str) -> Result<ActionResult> {
        self.calculate_hashes_with_progress(id, &mut |_| {})
    }

    pub fn calculate_hashes_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
        self.ensure_current_audio_screening_artifacts(&track, &evidence)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let track_root = self.track_root(&track)?;
        track.documents.current = documents::is_current(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &evaluation.steps,
        )?;
        if !track.documents.current {
            return Err(AppError::Validation(
                "Generate the current managed documents before calculating hashes.".into(),
            ));
        }
        track.integrity = integrity::calculate_with_progress(&track_root, on_progress)?;
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: track.integrity.file_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let count = track.integrity.file_count;
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: count,
            total_files: count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!("{count} files hashed and re-verified."),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    pub fn verify_hashes_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.persistence.track(id)?;
        let track_root = self.track_root(&track)?;
        track.integrity = integrity::verify_with_progress(&track_root, on_progress)?;
        if !track.integrity.verified && track.status == TrackStatus::Finalized {
            invalidate_state(&mut track, "Track integrity changed after finalization");
        }
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_result".into(),
            processed_files: track.integrity.verified_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        self.persistence.save_track(&track)?;
        let message = format!(
            "{} of {} files verified.",
            track.integrity.verified_count, track.integrity.file_count
        );
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: track.integrity.verified_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message,
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    /// Explicit retry endpoint for the fully local stage. Imports and
    /// replacements also call this after persisting new release evidence; it
    /// never performs a network request.
    pub fn run_local_audio_screening_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        let evidence = self.persistence.evidence(id)?;
        let release = evidence
            .iter()
            .find(|item| {
                item.role == EvidenceRole::ReleaseWav
                    && item.verified
                    && item.verification_error.is_none()
                    && item.sha256.is_some()
            })
            .cloned()
            .ok_or_else(|| {
                AppError::Validation(
                    "Import and verify the authoritative final release audio before audio screening."
                        .into(),
                )
            })?;
        let root = self.track_root(&track)?;
        let record = audio_screening::local_fingerprint(
            &contained_path(&root, Path::new(&release.relative_path), true)?,
            &track.id,
            &release,
            &root,
            |stage, message| {
                on_progress(OperationProgress {
                    stage: stage.into(),
                    current_file: Some(message.into()),
                    ..OperationProgress::default()
                });
            },
        )?;
        track.audio_screening.local = record;
        self.ensure_release_unchanged_after_screening(&mut track, &release, &root)?;
        // A fresh local result belongs to this release only.  Do not carry an
        // old provider result (or its STALE invalidation marker) across an
        // explicit release replacement; the optional level starts clean and
        // is then represented from the current global configuration.
        if track.audio_screening.local.status == AudioScreeningStatus::FingerprintGenerated
            && (track.audio_screening.external.status == AudioScreeningStatus::Stale
                || !audio_screening::external_record_matches_source(
                    &track.audio_screening.external,
                    &track.id,
                    &release,
                ))
        {
            track.audio_screening.external = Default::default();
        }
        self.record_external_configuration_status_if_unrun(&mut track)?;
        // `local_fingerprint` writes the initial portable summary together
        // with its JSON record. The optional-provider status is resolved only
        // afterwards, so refresh just the Markdown summary to keep the
        // portable record and the persisted state in lockstep.
        audio_screening::refresh_screening_markdown(
            &root,
            &track.audio_screening.local,
            &track.audio_screening.external,
        )?;
        // A local record changes portable documentation artifacts. It has no
        // bearing on a valid external result for the same source bytes.
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: format!(
                "Local Chromaprint screening recorded: {}.",
                audio_screening::audio_screening_status_label(track.audio_screening.local.status)
            ),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    /// A missing optional provider is a documented skip, not a failed local
    /// screening or a legal conclusion. Once a record exists (including a
    /// stale record after release replacement), it is never overwritten here.
    fn record_external_configuration_status_if_unrun(&self, track: &mut TrackRecord) -> Result<()> {
        if track.audio_screening.external.status != AudioScreeningStatus::NotRun {
            return Ok(());
        }
        let settings = self.persistence.audio_screening_settings()?;
        let credentials_present = self.persistence.audio_screening_credentials_present()?;
        let (status, message) =
            audio_screening::provider_configuration_status(&settings, credentials_present);
        match status {
            AudioScreeningProviderStatus::Ready => {}
            AudioScreeningProviderStatus::Disabled
            | AudioScreeningProviderStatus::NotConfigured => {
                track.audio_screening.external.status = AudioScreeningStatus::SkippedNotConfigured;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::ConfigurationInvalid => {
                track.audio_screening.external.status = AudioScreeningStatus::ConfigurationInvalid;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::AuthenticationFailed => {
                track.audio_screening.external.status = AudioScreeningStatus::AuthenticationFailed;
                track.audio_screening.external.message = message;
            }
            AudioScreeningProviderStatus::ProviderUnavailable => {
                track.audio_screening.external.status = AudioScreeningStatus::ProviderUnavailable;
                track.audio_screening.external.message = message;
            }
        }
        Ok(())
    }

    /// The portable screening directory is system-owned.  Once the source
    /// binding is no longer current, preserve the old technical record below
    /// `.archive` instead of leaving a positive-looking result in the live
    /// track tree.  A subsequent local run publishes a fresh directory.
    fn archive_current_audio_screening_artifacts(&self, track: &TrackRecord) -> Result<()> {
        let root = self.track_root(track)?;
        audio_screening::archive_current_screening_artifacts(&root)?;
        Ok(())
    }

    /// `audio_screening` operates on a private, byte-verified snapshot so an
    /// external edit cannot change the audio consumed by fpcalc/ACRCloud.
    /// Re-verify the managed source immediately before accepting the produced
    /// record, too: otherwise an out-of-band edit during the long operation
    /// could make its old snapshot appear current in the database.
    fn ensure_release_unchanged_after_screening(
        &self,
        track: &mut TrackRecord,
        release: &EvidenceItem,
        root: &Path,
    ) -> Result<()> {
        let verification = evidence::verify(root, release.clone());
        let still_current = verification.as_ref().is_ok_and(|item| {
            item.verified
                && item.verification_error.is_none()
                && item.id == release.id
                && item.relative_path == release.relative_path
                && item.sha256 == release.sha256
                && item.size_bytes == release.size_bytes
        });
        if still_current {
            return Ok(());
        }

        // Keep the evidence record honest when verification produced a
        // controlled mismatch/missing result.  A low-level filesystem error
        // still results in a stale screening state below, without exposing
        // its raw path or decoder details to the user.
        if let Ok(verified) = verification {
            self.persistence.save_evidence(&track.id, &verified)?;
        }
        audio_screening::mark_screening_stale(&mut track.audio_screening);
        mark_content_changed(track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(track)?;
        self.archive_current_audio_screening_artifacts(track)?;
        Err(AppError::Validation(
            "The authoritative release audio changed while audio screening was running. The result was discarded; verify or replace the release file and run screening again."
                .into(),
        ))
    }

    /// Track state stores the expected hashes for the portable local record
    /// and (when retained) the raw provider response.  Hash generation must
    /// never make a modified artifact look current merely by hashing its new
    /// bytes into SHA256SUMS, so verify these state-to-artifact anchors before
    /// documents, hashes, or finalization use them.
    fn ensure_current_audio_screening_artifacts(
        &self,
        track: &TrackRecord,
        evidence: &[EvidenceItem],
    ) -> Result<()> {
        let root = self.track_root(track)?;

        // Stale state is a durable invalidation marker. If a filesystem
        // failure prevented the best-effort archival move, never let
        // document/hash generation re-adopt the still-live old directory.
        // A local rerun (or a fresh release import) repairs it by archiving
        // and publishing a new source-bound artifact set.
        if matches!(
            track.audio_screening.local.status,
            AudioScreeningStatus::Stale
        ) || matches!(
            track.audio_screening.external.status,
            AudioScreeningStatus::Stale
        ) {
            let live_directory = contained_path(
                &root,
                Path::new(audio_screening::AUDIO_SCREENING_DIR),
                false,
            )?;
            match fs::symlink_metadata(&live_directory) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(AppError::Symlink(live_directory.display().to_string()));
                }
                Ok(_) => {
                    return Err(AppError::Validation(
                        "Stale audio-screening artifacts are still present in the live track directory. Run the local screening again after restoring or replacing the release audio."
                            .into(),
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::io(&live_directory, error)),
            }
        }

        let Some(release) = evidence.iter().find(|item| {
            item.role == EvidenceRole::ReleaseWav
                && item.verified
                && item.verification_error.is_none()
                && item.sha256.is_some()
        }) else {
            return Ok(());
        };

        if audio_screening::local_record_matches_source(
            &track.audio_screening.local,
            &track.id,
            release,
        ) && !audio_screening::local_artifact_is_current(&root, &track.audio_screening.local)
            .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The local audio-screening record no longer matches its portable artifact. Run the local Chromaprint screening again before continuing."
                    .into(),
            ));
        }

        if audio_screening::external_record_matches_source(
            &track.audio_screening.external,
            &track.id,
            release,
        ) && !audio_screening::external_response_artifact_is_current(
            &root,
            &track.audio_screening.external,
        )
        .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The external audio-screening response no longer matches its portable artifact. Run the explicit ACRCloud screening again before continuing."
                    .into(),
            ));
        }

        Ok(())
    }

    /// The sole call path that can upload an audio sample. The Tauri command is
    /// reached only from the explicit Step-09 user action; it is never invoked
    /// by opening a workspace, importing evidence, generating hashes, or
    /// finalizing a track.
    pub fn run_external_audio_screening_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        let mut track = self.mutable_track(id)?;
        let evidence = self.persistence.evidence(id)?;
        let release = evidence
            .iter()
            .find(|item| {
                item.role == EvidenceRole::ReleaseWav
                    && item.verified
                    && item.verification_error.is_none()
                    && item.sha256.is_some()
            })
            .cloned()
            .ok_or_else(|| {
                AppError::Validation(
                    "Import and verify the authoritative final release audio before external screening."
                        .into(),
                )
            })?;
        if !audio_screening::local_record_matches_source(
            &track.audio_screening.local,
            &track.id,
            &release,
        ) {
            return Err(AppError::Validation(
                "Generate a current local Chromaprint fingerprint for the authoritative release audio before starting external screening."
                    .into(),
            ));
        }
        let root = self.track_root(&track)?;
        if !audio_screening::local_artifact_is_current(&root, &track.audio_screening.local)
            .unwrap_or(false)
        {
            return Err(AppError::Validation(
                "The current local Chromaprint record is missing or has changed. Run the local screening again before starting external screening."
                    .into(),
            ));
        }
        let settings = self.persistence.audio_screening_settings()?;
        let credentials = self.persistence.audio_screening_credentials()?;
        let record = audio_screening::run_external_audio_screening_with_credentials(
            &settings,
            credentials
                .as_ref()
                .map(|(access_key, access_secret)| (access_key.as_str(), access_secret.as_str())),
            &contained_path(&root, Path::new(&release.relative_path), true)?,
            &track.id,
            &release,
            &root,
            Some(&track.audio_screening.local),
            |stage, message| {
                on_progress(OperationProgress {
                    stage: stage.into(),
                    current_file: Some(message.into()),
                    ..OperationProgress::default()
                });
            },
        )?;
        track.audio_screening.external = record;
        self.ensure_release_unchanged_after_screening(&mut track, &release, &root)?;
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: format!(
                "External ACRCloud screening recorded: {}.",
                audio_screening::audio_screening_status_label(
                    track.audio_screening.external.status
                )
            ),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    pub fn validate_track(&self, id: &str) -> Result<ValidationResult> {
        let track = self.persistence.track(id)?;
        self.validation_for(track)
    }

    #[cfg(test)]
    pub fn finalize_track(&self, id: &str) -> Result<ActionResult> {
        self.finalize_track_with_options(id, FinalizeOptions::default())
    }

    #[cfg(test)]
    pub fn finalize_track_with_options(
        &self,
        id: &str,
        options: FinalizeOptions,
    ) -> Result<ActionResult> {
        self.finalize_track_with_options_and_progress(id, options, &mut |_| {})
    }

    #[cfg(test)]
    pub fn finalize_track_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_with_options_and_progress(id, FinalizeOptions::default(), on_progress)
    }

    pub fn finalize_track_with_options_and_progress(
        &self,
        id: &str,
        options: FinalizeOptions,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_impl_with_options(
            id,
            options,
            #[cfg(test)]
            None,
            on_progress,
        )
    }

    #[cfg(test)]
    fn finalize_track_impl(
        &self,
        id: &str,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_impl_with_options(
            id,
            FinalizeOptions::default(),
            #[cfg(test)]
            failure,
            on_progress,
        )
    }

    fn finalize_track_impl_with_options(
        &self,
        id: &str,
        options: FinalizeOptions,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        on_progress(OperationProgress {
            stage: "validating_finalization_gate".into(),
            ..OperationProgress::default()
        });
        let mut track = self.mutable_track(id)?;
        ensure_current_workflow(&track)?;
        let validation = self.validation_for(track.clone())?;
        if !validation.valid {
            return Err(AppError::Validation(
                validation
                    .missing_items
                    .iter()
                    .chain(validation.blocking_items.iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        on_progress(OperationProgress {
            stage: "collecting_final_snapshot".into(),
            ..OperationProgress::default()
        });
        // Re-read all state after validation so the certificate uses exactly the gate input.
        track = self.persistence.track(id)?;
        // Freeze the optional provider's actual configuration state only in
        // the immutable certificate snapshot. Persisting this presentation
        // fact on the editable track after the documentation-currentness gate
        // would immediately make the documentation stale.
        let mut certificate_track = track.clone();
        let screening_settings = self.persistence.audio_screening_settings()?;
        let screening_credentials_present =
            self.persistence.audio_screening_credentials_present()?;
        let (screening_provider_status, screening_provider_message) =
            audio_screening::provider_configuration_status(
                &screening_settings,
                screening_credentials_present,
            );
        certificate_track
            .audio_screening
            .external
            .configured_at_snapshot = Some(matches!(
            screening_provider_status,
            AudioScreeningProviderStatus::Ready
        ));
        if certificate_track.audio_screening.external.status == AudioScreeningStatus::NotRun
            && !matches!(
                screening_provider_status,
                AudioScreeningProviderStatus::Ready
            )
        {
            certificate_track.audio_screening.external.status = match screening_provider_status {
                AudioScreeningProviderStatus::Disabled
                | AudioScreeningProviderStatus::NotConfigured => {
                    AudioScreeningStatus::SkippedNotConfigured
                }
                AudioScreeningProviderStatus::ConfigurationInvalid => {
                    AudioScreeningStatus::ConfigurationInvalid
                }
                AudioScreeningProviderStatus::AuthenticationFailed => {
                    AudioScreeningStatus::AuthenticationFailed
                }
                AudioScreeningProviderStatus::ProviderUnavailable => {
                    AudioScreeningStatus::ProviderUnavailable
                }
                AudioScreeningProviderStatus::Ready => AudioScreeningStatus::NotRun,
            };
            certificate_track.audio_screening.external.message = screening_provider_message;
        }
        // Certificate language is intentionally resolved from the current
        // workspace setting rather than the editable track profile snapshot.
        // A language-only settings update must not invalidate documents or
        // hashes; the selected value is frozen below with the final snapshot.
        let certificate_render_options = CertificateRenderOptions {
            language: self.profile()?.certificate_language,
            bilingual: options.bilingual,
        };
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
        self.ensure_current_audio_screening_artifacts(&track, &evidence)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let finalized_at = now();
        let certificate_id = format!("SDM-{}", Uuid::new_v4());
        let finalization_snapshot_id = Uuid::new_v4().to_string();
        let transaction_id = Uuid::new_v4().to_string();
        let track_root = self.track_root(&track)?;
        let live_certificate =
            contained_path(&track_root, Path::new(certificate::CERTIFICATE_DIR), false)?;
        let live_pdf = contained_path(&track_root, Path::new(certificate::PDF_FILE), false)?;
        if !directory_is_empty_or_missing(&live_certificate)? {
            return Err(AppError::Collision(
                "The certificate directory already contains files. Preserve or archive them before finalizing."
                    .into(),
            ));
        }
        if live_pdf.exists() {
            return Err(AppError::Collision(format!(
                "The technical documentation PDF already exists: {}",
                certificate::PDF_FILE
            )));
        }
        let certificate_staging = contained_path(
            &track_root,
            &PathBuf::from(".archive/certificate-staging").join(&transaction_id),
            false,
        )?;
        let finalization_marker = contained_path(
            &track_root,
            Path::new(".archive/finalization-in-progress.json"),
            false,
        )?;
        let marker = serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "transaction_id": transaction_id,
            "track_id": track.id,
            "certificate_id": certificate_id,
            "finalization_snapshot_id": finalization_snapshot_id,
            "certificate_render_options": certificate_render_options,
            "started_at": now(),
        }))?;
        on_progress(OperationProgress {
            stage: "writing_finalization_marker".into(),
            ..OperationProgress::default()
        });
        atomic_write_new(&finalization_marker, &marker)?;
        on_progress(OperationProgress {
            stage: "generating_certificate".into(),
            processed_files: evidence.len() as u32,
            total_files: evidence.len() as u32,
            ..OperationProgress::default()
        });
        #[cfg(test)]
        let publication = match failure.and_then(FinalizationFailure::certificate_failure) {
            Some(certificate_failure) => certificate::generate_with_failure(
                &track_root,
                &certificate_track,
                &certificate_track.profile_snapshot,
                &evaluation.steps,
                &evidence,
                &deviations,
                &certificate_id,
                &finalized_at,
                &transaction_id,
                certificate_render_options,
                certificate_failure,
            ),
            None => certificate::generate(
                &track_root,
                &certificate_track,
                &certificate_track.profile_snapshot,
                &evaluation.steps,
                &evidence,
                &deviations,
                &certificate_id,
                &finalized_at,
                &transaction_id,
                certificate_render_options,
            ),
        };
        #[cfg(not(test))]
        let publication = certificate::generate(
            &track_root,
            &certificate_track,
            &certificate_track.profile_snapshot,
            &evaluation.steps,
            &evidence,
            &deviations,
            &certificate_id,
            &finalized_at,
            &transaction_id,
            certificate_render_options,
        );
        if let Err(error) = publication {
            if directory_is_empty_or_missing(&live_certificate).unwrap_or(false)
                && !live_pdf.exists()
                && !certificate_staging.exists()
            {
                let _ = fs::remove_file(&finalization_marker);
            }
            return Err(error);
        }
        on_progress(OperationProgress {
            stage: "verifying_certificate".into(),
            ..OperationProgress::default()
        });
        if let Err(error) = certificate::verify(&track_root) {
            let rolled_back = rollback_certificate_set(&track_root, error);
            if rolled_back.complete {
                let _ = fs::remove_file(&finalization_marker);
            }
            return Err(rolled_back.error);
        }
        on_progress(OperationProgress {
            stage: "verifying_final_snapshot".into(),
            processed_files: 0,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        let post_publish_integrity = match integrity::verify_with_progress(&track_root, on_progress)
        {
            Ok(state) if state.verified => state,
            Ok(state) => {
                let rolled_back = rollback_certificate_set(
                    &track_root,
                    AppError::Validation(format!(
                        "Track files changed during finalization: {}",
                        state.mismatch_files.join(", ")
                    )),
                );
                if rolled_back.complete {
                    let _ = fs::remove_file(&finalization_marker);
                }
                return Err(rolled_back.error);
            }
            Err(error) => {
                let rolled_back = rollback_certificate_set(&track_root, error);
                if rolled_back.complete {
                    let _ = fs::remove_file(&finalization_marker);
                }
                return Err(rolled_back.error);
            }
        };
        track.integrity = post_publish_integrity;
        track.status = TrackStatus::Finalized;
        track.certificate = CertificateState {
            valid: true,
            certificate_id: Some(certificate_id),
            finalization_snapshot_id: Some(finalization_snapshot_id),
            finalized_at: Some(finalized_at),
            workflow_version: Some(track.workflow_version.clone()),
            certificate_language: certificate_render_options.language,
            bilingual: certificate_render_options.bilingual,
            invalidated_at: None,
            invalidation_reason: None,
        };
        track.updated_at = now();
        on_progress(OperationProgress {
            stage: "saving_final_snapshot".into(),
            processed_files: track.integrity.verified_count,
            total_files: track.integrity.file_count,
            ..OperationProgress::default()
        });
        #[cfg(test)]
        let database_commit = if failure == Some(FinalizationFailure::DatabaseCommit) {
            Err(AppError::Data(
                "Injected finalization database commit failure.".into(),
            ))
        } else {
            self.persistence.save_track(&track)
        };
        #[cfg(not(test))]
        let database_commit = self.persistence.save_track(&track);
        if let Err(error) = database_commit {
            let rolled_back = rollback_certificate_set(&track_root, error);
            if rolled_back.complete {
                let _ = fs::remove_file(&finalization_marker);
            }
            return Err(rolled_back.error);
        }
        // The database commit is authoritative. A stale marker is harmless and is
        // removed during the next workspace recovery if this best-effort cleanup fails.
        let _ = fs::remove_file(&finalization_marker);
        let detail = self.detail_from_record(track, false)?;
        // Phase two is deliberately attempted only after the immutable phase
        // one snapshot has been published and committed. Every outcome is
        // represented as timestamp status; no provider failure can roll back
        // DOCUMENTATION COMPLETE.
        let detail = if self
            .persistence
            .timestamp_settings()
            .is_ok_and(|settings| settings.enabled && settings.auto_after_finalization)
        {
            self.attach_configured_external_timestamp(&detail.id)
                .unwrap_or(detail)
        } else {
            detail
        };
        on_progress(OperationProgress {
            stage: "complete".into(),
            processed_files: detail.integrity.verified_count,
            total_files: detail.integrity.file_count,
            ..OperationProgress::default()
        });
        Ok(ActionResult {
            message: format!(
                "Documentation finalized and certificate set verified. Technical documentation certificate created: {}",
                certificate::PDF_FILE
            ),
            track: Some(detail),
        })
    }

    pub fn attach_external_timestamp_from(
        &self,
        id: &str,
        source: &Path,
        input: ExternalTimestampInput,
    ) -> Result<TrackDetail> {
        let track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized
            || !track.certificate.valid
            || track.certificate.certificate_id.is_none()
        {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached to a valid technically finalized snapshot."
                    .into(),
            ));
        }
        let track_root = self.track_root(&track)?;
        certificate::verify(&track_root)?;
        let integrity = integrity::verify(&track_root)?;
        if !integrity.verified {
            return Err(AppError::Validation(format!(
                "The finalized track integrity check failed: {}",
                integrity.mismatch_files.join(", ")
            )));
        }
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?;
        let anchors_before = external_timestamp::finalization_anchors(&track_root)?;
        let staged = external_timestamp::stage(&track_root, certificate_id, source, input)?;
        let record = staged.record.clone();

        // Register the complete stage first. A process exit from this point on
        // leaves a database-visible pending record that workspace recovery can
        // deterministically publish; it cannot leave an invisible live orphan.
        if let Err(error) = self.persistence.save_external_timestamp(id, &record) {
            return Err(match external_timestamp::discard_staged(&track_root, &staged) {
                Ok(()) => error,
                Err(cleanup) => AppError::Data(format!(
                    "External timestamp database registration failed ({error}); staging cleanup also failed ({cleanup})."
                )),
            });
        }

        let post_publish_check = (|| -> Result<()> {
            external_timestamp::publish(&track_root, &staged)?;
            external_timestamp::verify_published_record(&track_root, &record)?;
            certificate::verify(&track_root)?;
            let post_integrity = integrity::verify(&track_root)?;
            if !post_integrity.verified {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed the phase-one integrity set.".into(),
                ));
            }
            let anchors_after = external_timestamp::finalization_anchors(&track_root)?;
            if anchors_after != anchors_before {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed a finalized anchor.".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = post_publish_check {
            let filesystem_cleanup =
                external_timestamp::remove_published_record(&track_root, &record)
                    .and_then(|()| external_timestamp::discard_staged(&track_root, &staged));
            if let Err(cleanup) = filesystem_cleanup {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); filesystem cleanup also failed ({cleanup}). The registered record was retained for recovery."
                )));
            }
            if let Err(cleanup) = self.persistence.remove_external_timestamp(id, &record.id) {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); database rollback also failed ({cleanup}). The registered failed record remains visible."
                )));
            }
            return Err(error);
        }
        self.detail_from_record(track, false)
    }

    /// Automatically request and attach a provider response for the fixed
    /// phase-one `EVIDENCE_MANIFEST.json` anchor. No timestamp metadata or
    /// source file is accepted from the UI.
    pub fn attach_configured_external_timestamp(&self, id: &str) -> Result<TrackDetail> {
        let track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized
            || !track.certificate.valid
            || track.certificate.certificate_id.is_none()
        {
            return Err(AppError::Validation(
                "External timestamp evidence can only be attached to a valid technically finalized snapshot."
                    .into(),
            ));
        }
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?
            .to_owned();
        let track_root = self.track_root(&track)?;
        let settings = self.persistence.timestamp_settings()?;
        let provider = external_timestamp::provider_display_name(&settings);
        let secret = self.persistence.timestamp_secret()?;
        let (configuration_status, configuration_message) = external_timestamp::settings_status(
            &settings,
            secret
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
        );
        if configuration_status != ExternalTimestampStatus::Ready {
            self.save_timestamp_attachment_summary(
                &track,
                ExternalTimestampSummary {
                    status: configuration_status,
                    message: configuration_message,
                    provider,
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return self.detail_from_record(track, false);
        }

        let existing = self
            .persistence
            .timestamp_attachment_summary(&track.id, &certificate_id)?;
        let records = self.persistence.external_timestamps(&track.id)?;
        let already_verified = existing
            .as_ref()
            .is_some_and(|summary| summary.status == ExternalTimestampStatus::Verified)
            || records.iter().any(|record| {
                record.certificate_id == certificate_id
                    && record.provider_metadata.as_ref().is_some_and(|metadata| {
                        metadata.verification_result == ExternalTimestampStatus::Verified
                            && metadata.signature_verified == Some(true)
                            && metadata.trust_chain_verified == Some(true)
                    })
            });
        if already_verified {
            self.save_timestamp_attachment_summary(
                &track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::Verified,
                    message: "A technically verified external timestamp is already attached to this finalized snapshot."
                        .into(),
                    provider,
                    record_id: existing.and_then(|summary| summary.record_id),
                    updated_at: Some(now()),
                },
            )?;
            return self.detail_from_record(track, false);
        }

        let anchor = match self.verified_timestamp_anchor(&track_root) {
            Ok(anchor) => anchor,
            Err(error) => {
                self.save_timestamp_attachment_summary(
                    &track,
                    ExternalTimestampSummary {
                        status: ExternalTimestampStatus::AnchorMismatch,
                        message: "The selected timestamp anchor no longer matches the finalized snapshot."
                            .into(),
                        provider,
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                let _ = error;
                return self.detail_from_record(track, false);
            }
        };
        self.save_timestamp_attachment_summary(
            &track,
            ExternalTimestampSummary {
                status: ExternalTimestampStatus::Requesting,
                message:
                    "Requesting external timestamp evidence for the finalized manifest anchor."
                        .into(),
                provider: provider.clone(),
                record_id: None,
                updated_at: Some(now()),
            },
        )?;
        let response = match external_timestamp::request_timestamp(
            &settings,
            secret.as_deref(),
            &anchor.sha256,
        ) {
            Ok(response) => response,
            Err(failure) => {
                self.save_timestamp_attachment_summary(
                    &track,
                    ExternalTimestampSummary {
                        status: failure.status,
                        message: failure.message,
                        provider,
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                return self.detail_from_record(track, false);
            }
        };

        // Recheck immediately after the network call. A timestamp response is
        // never published for a manifest that changed while the provider was
        // handling the digest request.
        if self.verified_timestamp_anchor(&track_root).is_err() {
            self.save_timestamp_attachment_summary(
                &track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::AnchorMismatch,
                    message: "The selected timestamp anchor changed while the provider request was in progress."
                        .into(),
                    provider: response.provider,
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return self.detail_from_record(track, false);
        }

        let temporary_source =
            provider_response_staging_path(&track_root, &response.evidence_extension)?;
        atomic_write_new(&temporary_source, &response.evidence_bytes)?;
        let finalization_snapshot_id = referenced_finalization_snapshot_id(&track, &certificate_id);
        let response_status = response.status;
        let response_message = response.message.clone();
        let response_provider = response.provider.clone();
        let staged = external_timestamp::stage_provider_response(
            &track_root,
            &certificate_id,
            &finalization_snapshot_id,
            &anchor.sha256,
            &temporary_source,
            response,
        );
        let _ = fs::remove_file(&temporary_source);
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                let status = if self.verified_timestamp_anchor(&track_root).is_err() {
                    ExternalTimestampStatus::AnchorMismatch
                } else {
                    ExternalTimestampStatus::VerificationFailed
                };
                self.save_timestamp_attachment_summary(
                    &track,
                    ExternalTimestampSummary {
                        status,
                        message: "Timestamp provider response could not be archived safely.".into(),
                        provider: response_provider,
                        record_id: None,
                        updated_at: Some(now()),
                    },
                )?;
                return Err(error);
            }
        };
        let record = staged.record.clone();
        let anchors_before = external_timestamp::finalization_anchors(&track_root)?;
        if let Err(error) = self.persistence.save_external_timestamp(id, &record) {
            return Err(match external_timestamp::discard_staged(&track_root, &staged) {
                Ok(()) => error,
                Err(cleanup) => AppError::Data(format!(
                    "External timestamp database registration failed ({error}); staging cleanup also failed ({cleanup})."
                )),
            });
        }
        let post_publish_check = (|| -> Result<()> {
            external_timestamp::publish(&track_root, &staged)?;
            external_timestamp::verify_published_record(&track_root, &record)?;
            certificate::verify(&track_root)?;
            let post_integrity = integrity::verify(&track_root)?;
            if !post_integrity.verified {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed the phase-one integrity set.".into(),
                ));
            }
            let anchors_after = external_timestamp::finalization_anchors(&track_root)?;
            if anchors_after != anchors_before {
                return Err(AppError::Validation(
                    "Attaching timestamp evidence changed a finalized anchor.".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = post_publish_check {
            let filesystem_cleanup =
                external_timestamp::remove_published_record(&track_root, &record)
                    .and_then(|()| external_timestamp::discard_staged(&track_root, &staged));
            if let Err(cleanup) = filesystem_cleanup {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); filesystem cleanup also failed ({cleanup}). The registered record was retained for recovery."
                )));
            }
            if let Err(cleanup) = self.persistence.remove_external_timestamp(id, &record.id) {
                return Err(AppError::Data(format!(
                    "External timestamp publication failed ({error}); database rollback also failed ({cleanup}). The registered failed record remains visible."
                )));
            }
            self.save_timestamp_attachment_summary(
                &track,
                ExternalTimestampSummary {
                    status: ExternalTimestampStatus::VerificationFailed,
                    message: "Timestamp provider response could not be published safely.".into(),
                    provider: response_provider,
                    record_id: None,
                    updated_at: Some(now()),
                },
            )?;
            return Err(error);
        }
        self.save_timestamp_attachment_summary(
            &track,
            ExternalTimestampSummary {
                status: response_status,
                message: response_message,
                provider: response_provider,
                record_id: Some(record.id),
                updated_at: Some(now()),
            },
        )?;
        self.detail_from_record(track, false)
    }

    fn verified_timestamp_anchor(&self, track_root: &Path) -> Result<FinalizationAnchor> {
        let anchor = external_timestamp::finalized_manifest_anchor(track_root)?;
        // The certificate hash set itself must also remain valid. The specific
        // manifest mismatch above has already produced the user-facing anchor
        // status; this prevents a request when another phase-one certificate
        // artifact was modified.
        certificate::verify(track_root)?;
        let integrity = integrity::verify(track_root)?;
        if !integrity.verified {
            return Err(AppError::Validation(
                "The finalized track integrity check failed before timestamp attachment.".into(),
            ));
        }
        Ok(anchor)
    }

    fn save_timestamp_attachment_summary(
        &self,
        track: &TrackRecord,
        summary: ExternalTimestampSummary,
    ) -> Result<()> {
        let certificate_id = track
            .certificate
            .certificate_id
            .as_deref()
            .ok_or_else(|| AppError::Data("Finalized track has no certificate ID.".into()))?;
        self.persistence
            .save_timestamp_attachment_summary(&track.id, certificate_id, &summary)
    }

    pub fn invalidate_certificate(&self, id: &str) -> Result<ActionResult> {
        let mut track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized {
            return Err(AppError::Validation("The track is not finalized.".into()));
        }
        invalidate_state(&mut track, "Certificate invalidated by the user");
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        Ok(ActionResult {
            message: "Certificate marked invalid; certificate files were not overwritten.".into(),
            track: Some(self.detail_from_record(track, false)?),
        })
    }

    pub fn create_revision(&self, id: &str) -> Result<ActionResult> {
        let mut track = self.persistence.track(id)?;
        if track.status != TrackStatus::Finalized {
            return Err(AppError::Validation(
                "Only a finalized track can start a new revision.".into(),
            ));
        }
        let root = self.track_root(&track)?;
        // Analyze legacy Suno evidence before any certificate artifacts are
        // moved. Missing, changed, or unreadable bytes remain a non-fatal
        // verification concern for the new revision instead of leaving a
        // half-created revision behind.
        let (prepared_track, analyzed_evidence) = self.prepare_revision_suno_analysis(track)?;
        track = prepared_track;
        let certificate_integrity = if certificate::verify(&root).is_ok() {
            "valid"
        } else {
            "invalid_or_incomplete"
        };
        let revision_id = Uuid::new_v4().to_string();
        // Older and imported track layouts can predate the managed revisions
        // directory. Resolve and create the publish parent before staging or
        // moving any live certificate artifacts so the final rename cannot fail
        // merely because `.archive/revisions` is absent.
        ensure_contained_directory(&root, Path::new(".archive/revisions"))?;
        let archive_relative = PathBuf::from(".archive/revisions").join(&revision_id);
        let archive = contained_path(&root, &archive_relative, false)?;
        if archive.exists() {
            return Err(AppError::Collision(archive.display().to_string()));
        }
        let live_certificate =
            contained_path(&root, Path::new(certificate::CERTIFICATE_DIR), false)?;
        let certificate_existed = live_certificate.exists();
        let live_pdf = contained_path(&root, Path::new(certificate::PDF_FILE), false)?;
        let pdf_existed = regular_file_if_present(&live_pdf, "The technical documentation PDF")?;
        // Screening artifacts are part of the integrity-protected phase-one
        // snapshot. Move them with the certificate rather than leaving an old
        // fingerprint or provider response in the live revision workspace.
        let live_audio_screening =
            contained_path(&root, Path::new("03_DOCUMENTATION/AUDIO_SCREENING"), false)?;
        let audio_screening_existed = regular_directory_if_present(
            &live_audio_screening,
            "The pre-release audio-screening directory",
        )?;
        let stage_relative = PathBuf::from(".archive/revision-staging").join(&revision_id);
        let stage = ensure_contained_directory(&root, &stage_relative)?;
        let live_hashes = contained_path(&root, Path::new(integrity::HASH_FILE), false)?;
        let stage_preparation = (|| -> Result<()> {
            if live_hashes.is_file() {
                let archived_hashes = stage.join(integrity::HASH_FILE);
                if let Some(parent) = archived_hashes.parent() {
                    fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
                }
                copy_new(&live_hashes, &archived_hashes)?;
                if sha256_file(&live_hashes)? != sha256_file(&archived_hashes)? {
                    return Err(AppError::Validation(
                        "The revision SHA256SUMS archive copy could not be verified.".into(),
                    ));
                }
            }
            let metadata = serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "revision_id": revision_id,
                "track_id": track.id,
                "archived_at": now(),
                "previous_certificate": track.certificate,
                "certificate_integrity_at_archive": certificate_integrity,
            }))?;
            atomic_write_new(&stage.join("revision.json"), &metadata)?;
            Ok(())
        })();
        if let Err(error) = stage_preparation {
            return Err(cleanup_revision_staging(&stage, error));
        }
        let staged_certificate = stage.join("certificate");
        let staged_pdf = stage.join(certificate::PDF_FILE);
        let staged_audio_screening = stage.join("03_DOCUMENTATION/AUDIO_SCREENING");
        let certificate_staging = if certificate_existed {
            fs::rename(&live_certificate, &staged_certificate)
                .map_err(|error| AppError::io(&live_certificate, error))
        } else {
            fs::create_dir(&staged_certificate)
                .map_err(|error| AppError::io(&staged_certificate, error))
        };
        if let Err(error) = certificate_staging {
            return Err(cleanup_revision_staging(&stage, error));
        }
        let mut pdf_moved = false;
        if pdf_existed {
            if let Err(error) = fs::rename(&live_pdf, &staged_pdf) {
                return Err(rollback_revision_state(
                    &live_certificate,
                    &staged_certificate,
                    &live_pdf,
                    &staged_pdf,
                    &stage,
                    certificate_existed,
                    pdf_moved,
                    AppError::io(&live_pdf, error),
                ));
            }
            pdf_moved = true;
        }
        let mut audio_screening_moved = false;
        if audio_screening_existed {
            if let Some(parent) = staged_audio_screening.parent() {
                fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
            }
            if let Err(error) = fs::rename(&live_audio_screening, &staged_audio_screening) {
                return Err(rollback_revision_state(
                    &live_certificate,
                    &staged_certificate,
                    &live_pdf,
                    &staged_pdf,
                    &stage,
                    certificate_existed,
                    pdf_moved,
                    AppError::io(&live_audio_screening, error),
                ));
            }
            audio_screening_moved = true;
        }
        if let Err(error) =
            ensure_contained_directory(&root, Path::new(certificate::CERTIFICATE_DIR))
        {
            let restore_error = if audio_screening_moved {
                restore_audio_screening_directory(&staged_audio_screening, &live_audio_screening)
                    .err()
            } else {
                None
            };
            if let Some(restore_error) = restore_error {
                return Err(AppError::Data(format!(
                    "Revision failed ({error}); audio-screening rollback failed: {restore_error}"
                )));
            }
            return Err(rollback_revision_state(
                &live_certificate,
                &staged_certificate,
                &live_pdf,
                &staged_pdf,
                &stage,
                certificate_existed,
                pdf_moved,
                error,
            ));
        }
        if let Err(error) = fs::rename(&stage, &archive) {
            let restore_error = if audio_screening_moved {
                restore_audio_screening_directory(&staged_audio_screening, &live_audio_screening)
                    .err()
            } else {
                None
            };
            if let Some(restore_error) = restore_error {
                return Err(AppError::Data(format!(
                    "Revision archive publication failed ({error}); audio-screening rollback failed: {restore_error}"
                )));
            }
            return Err(rollback_revision_state(
                &live_certificate,
                &staged_certificate,
                &live_pdf,
                &staged_pdf,
                &stage,
                certificate_existed,
                pdf_moved,
                AppError::io(&archive, error),
            ));
        }
        track.status = TrackStatus::Active;
        track.certificate = CertificateState::default();
        track.documents.current = false;
        track.integrity = IntegrityState::default();
        track.audio_screening = Default::default();
        track.updated_at = now();
        if let Err(error) = self
            .persistence
            .save_track_and_evidence(&track, &analyzed_evidence)
        {
            let archived_audio_screening = archive.join("03_DOCUMENTATION/AUDIO_SCREENING");
            let restore_error = if audio_screening_moved {
                restore_audio_screening_directory(&archived_audio_screening, &live_audio_screening)
                    .err()
            } else {
                None
            };
            if let Some(restore_error) = restore_error {
                return Err(AppError::Data(format!(
                    "Revision database save failed ({error}); audio-screening rollback failed: {restore_error}"
                )));
            }
            return Err(rollback_revision_state(
                &live_certificate,
                &archive.join("certificate"),
                &live_pdf,
                &archive.join(certificate::PDF_FILE),
                &archive,
                certificate_existed,
                pdf_moved,
                error,
            ));
        }
        let detail = self.detail_from_record(track, false)?;
        let detail = if analyzed_evidence
            .iter()
            .any(|item| item.role == EvidenceRole::ReleaseWav)
        {
            self.run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .unwrap_or(detail)
        } else {
            detail
        };
        Ok(ActionResult {
            message: format!("Previous certificate archived as revision {revision_id}."),
            track: Some(detail),
        })
    }

    pub fn re_evaluate_track(&self, id: &str) -> Result<ActionResult> {
        let current = workflow::config()?;
        self.re_evaluate_track_with_workflow(id, &current)
    }

    fn re_evaluate_track_with_workflow(
        &self,
        id: &str,
        current: &workflow::WorkflowConfig,
    ) -> Result<ActionResult> {
        let previous = self.persistence.track(id)?;
        if previous.status == TrackStatus::Superseded {
            return Err(AppError::Finalized);
        }
        if previous.workflow_id == current.id && previous.workflow_version == current.version {
            return Err(AppError::Validation(
                "The track already uses the current workflow version.".into(),
            ));
        }

        let archived = previous.status == TrackStatus::Finalized;
        if archived {
            self.create_revision(id)?;
        }

        let mut track = self.persistence.track(id)?;
        let evidence = self.persistence.evidence(id)?;
        reconcile_evidence_derived_fields(&mut track, &evidence);
        track.workflow_id = current.id.clone();
        track.workflow_version = current.version.clone();
        track.status = TrackStatus::Active;
        track.documents.current = false;
        track.integrity = IntegrityState::default();
        track.certificate = CertificateState::default();
        track.updated_at = now();

        let root = self.track_root(&track)?;
        let live_hashes = contained_path(&root, Path::new(integrity::HASH_FILE), false)?;
        if live_hashes.is_file() {
            fs::remove_file(&live_hashes).map_err(|error| AppError::io(&live_hashes, error))?;
        } else if live_hashes.exists() {
            return Err(AppError::Validation(
                "The current SHA256SUMS path is not a regular file.".into(),
            ));
        }
        self.persistence.save_track_clearing_steps(&track)?;

        // A workflow upgrade can make an older editable track subject to the
        // new local-screening requirement even though no release file was
        // imported in this invocation.  Keep that path automatic and local;
        // archived finalizations have already run this as part of
        // `create_revision` above.
        let has_verified_release = evidence.iter().any(|item| {
            item.role == EvidenceRole::ReleaseWav
                && item.verified
                && item.verification_error.is_none()
                && item.sha256.is_some()
        });
        let detail = self.detail_from_record(track, false)?;
        let detail = if !archived && has_verified_release {
            self.run_local_audio_screening_with_progress(id, &mut |_| {})
                .map(|result| result.track.unwrap_or(detail.clone()))
                .unwrap_or(detail)
        } else {
            detail
        };

        Ok(ActionResult {
            message: if archived {
                format!(
                    "Previous certificate archived; track is ready for reevaluation with workflow {} {}.",
                    current.id, current.version
                )
            } else {
                format!(
                    "Track is ready for reevaluation with workflow {} {}.",
                    current.id, current.version
                )
            },
            track: Some(detail),
        })
    }

    pub fn scan_workspace(&self) -> Result<WorkspaceScan> {
        self.reconcile_physical_library()?;
        let mut candidates = Vec::new();
        let mut warnings = Vec::new();
        let mut indexed = 0_u32;
        let mut unchanged = 0_u32;
        for (name, relative, library) in discover_workspace_tracks(&self.root, &mut warnings)? {
            let track_root = contained_path(&self.root, &relative, true)?;
            let inspection = inspect_legacy(&track_root)?;
            let existing = self
                .persistence
                .track_by_relative_path(&portable_relative(&relative))?;
            let track = if let Some(track) = existing {
                unchanged += 1;
                track
            } else {
                let created_at = now();
                let config = workflow::config()?;
                let fields = crate::model::TrackFields {
                    title: name.clone(),
                    ..Default::default()
                };
                let track = TrackRecord {
                    id: Uuid::new_v4().to_string(),
                    relative_path: portable_relative(&relative),
                    status: TrackStatus::Active,
                    workflow_id: config.id,
                    workflow_version: config.version,
                    profile_snapshot: Profile::default(),
                    library,
                    field_origins: Default::default(),
                    fields,
                    audio_screening: Default::default(),
                    documents: DocumentState {
                        generated: false,
                        current: false,
                        generated_at: None,
                        template_version: documents::TEMPLATE_VERSION.into(),
                        files: inspection.documents.clone(),
                        input_fingerprint: String::new(),
                    },
                    integrity: IntegrityState::default(),
                    certificate: CertificateState::default(),
                    created_at: created_at.clone(),
                    updated_at: created_at,
                    legacy: true,
                };
                self.persistence.save_track(&track)?;
                indexed += 1;
                track
            };
            // Track folders remain the portable source of truth. Reconcile files that
            // are present without SQLite metadata for both historical folders and
            // managed tracks (for example after a process exit between copy and commit).
            self.reconcile_unindexed_evidence(&track, &track_root, &inspection.evidence_files)?;
            let detail = self.detail_from_record(track, false)?;
            candidates.push(LegacyCandidate {
                name,
                relative_path: detail.relative_path,
                status: if detail.legacy.unwrap_or(false) {
                    "NOT_VERIFIED".into()
                } else {
                    "INDEXED".into()
                },
                missing_items: detail.missing_items,
                has_managed_document_collision: inspection.has_managed_document_collision,
                recognized_folders: inspection.recognized_folders,
                documents: inspection.documents,
                evidence_files: inspection.evidence_files,
                hash_manifest_present: inspection.hash_manifest_present,
            });
        }
        candidates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let scanned_at = now();
        self.persistence.set_meta("last_scanned_at", &scanned_at)?;
        Ok(WorkspaceScan {
            discovered: candidates.len() as u32,
            indexed,
            unchanged,
            warnings,
            candidates,
        })
    }

    fn mutable_track(&self, id: &str) -> Result<TrackRecord> {
        self.reconcile_physical_library()?;
        let track = self.persistence.track(id)?;
        if matches!(
            track.status,
            TrackStatus::Finalized | TrackStatus::Superseded
        ) {
            return Err(AppError::Finalized);
        }
        Ok(track)
    }

    fn reconcile_unindexed_evidence(
        &self,
        track: &TrackRecord,
        track_root: &Path,
        files: &[String],
    ) -> Result<()> {
        let indexed: HashSet<String> = self
            .persistence
            .evidence(&track.id)?
            .into_iter()
            .map(|item| item.relative_path)
            .collect();
        let mut singular_roles = self
            .persistence
            .evidence(&track.id)?
            .into_iter()
            .filter(|item| {
                matches!(
                    item.role,
                    EvidenceRole::ReleaseWav
                        | EvidenceRole::SunoFinalExport
                        | EvidenceRole::FinalArtwork
                )
            })
            .map(|item| item.role)
            .collect::<HashSet<_>>();
        let mut unindexed_singular_candidates = HashMap::<EvidenceRole, usize>::new();
        for file in files.iter().filter(|file| !indexed.contains(*file)) {
            let role = infer_legacy_role(file);
            if matches!(
                role,
                EvidenceRole::ReleaseWav
                    | EvidenceRole::SunoFinalExport
                    | EvidenceRole::FinalArtwork
            ) {
                *unindexed_singular_candidates.entry(role).or_default() += 1;
            }
        }
        for file in files.iter().filter(|file| !indexed.contains(*file)) {
            let inferred_role = infer_legacy_role(file);
            let singular = matches!(
                inferred_role,
                EvidenceRole::ReleaseWav
                    | EvidenceRole::SunoFinalExport
                    | EvidenceRole::FinalArtwork
            );
            let ambiguous_singular = singular
                && (singular_roles.contains(&inferred_role)
                    || unindexed_singular_candidates
                        .get(&inferred_role)
                        .is_some_and(|count| *count > 1));
            if singular && !ambiguous_singular {
                singular_roles.insert(inferred_role);
            }
            let role = if ambiguous_singular {
                EvidenceRole::Other
            } else {
                inferred_role
            };
            let path = contained_path(track_root, Path::new(file), true)?;
            let metadata = fs::metadata(&path).map_err(|error| AppError::io(&path, error))?;
            let item = EvidenceItem {
                id: Uuid::new_v4().to_string(),
                role,
                file_name: path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("legacy-evidence")
                    .to_owned(),
                relative_path: file.clone(),
                sha256: Some(sha256_file(&path)?),
                size_bytes: metadata.len(),
                imported_at: now(),
                verified: false,
                verification_error: Some(if ambiguous_singular {
                    "Imported legacy evidence is an ambiguous duplicate final/release candidate; classify or remove it explicitly.".into()
                } else if track.legacy {
                    "Imported legacy evidence has not been independently verified.".into()
                } else {
                    "Recovered unindexed track evidence has not been independently verified.".into()
                }),
                source_global_evidence_id: None,
                coverage_start: None,
                coverage_end: None,
                provenance: EvidenceProvenance::IndexedLegacy,
                derived_from_evidence_id: None,
                generator_version: None,
                generated_disclosure_text: None,
                metadata: Default::default(),
            };
            self.persistence.save_evidence(&track.id, &item)?;
        }
        Ok(())
    }

    fn write_track_identity(&self, track: &TrackRecord) -> Result<()> {
        let root = contained_path(&self.root, Path::new(&track.relative_path), true)?;
        let identity_relative = Path::new(TRACK_IDENTITY_FILE);
        let identity = contained_path(&root, identity_relative, false)?;
        if identity.is_file() {
            let bytes = fs::read(&identity).map_err(|error| AppError::io(&identity, error))?;
            if serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|value| value.get("trackId")?.as_str().map(str::to_owned))
                .as_deref()
                == Some(track.id.as_str())
            {
                return Ok(());
            }
            return Err(AppError::Collision(identity.display().to_string()));
        }
        let mut bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "trackId": track.id,
        }))?;
        bytes.push(b'\n');
        atomic_write(&identity, &bytes)
    }

    fn existing_album_spelling(
        &self,
        library: TrackLibraryPlacement,
    ) -> Result<TrackLibraryPlacement> {
        if library.section != TrackLibrarySection::Album {
            return Ok(library);
        }
        let requested = library.album_title.as_deref().expect("normalized album");
        let comparison = requested.to_lowercase();
        if let Some(existing) = self
            .list_albums()?
            .into_iter()
            .find(|existing| existing.to_lowercase() == comparison)
        {
            return Ok(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some(existing),
            });
        }
        for track in self.persistence.tracks()? {
            if is_hidden_workspace_path(Path::new(&track.relative_path)) {
                continue;
            }
            let Some(existing) = track.library.album_title.as_deref() else {
                continue;
            };
            if track.library.section == TrackLibrarySection::Album
                && existing.to_lowercase() == comparison
            {
                return Ok(TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some(existing.to_owned()),
                });
            }
        }
        Ok(library)
    }

    fn move_track_directory(
        &self,
        source_relative: &str,
        target_relative: &str,
        remove_empty_source: bool,
    ) -> Result<()> {
        if source_relative == target_relative {
            return Ok(());
        }
        let source_relative = Path::new(source_relative);
        let target_relative = Path::new(target_relative);
        let source = contained_path(&self.root, source_relative, true)?;
        if !source.is_dir() {
            return Err(AppError::Validation(format!(
                "Managed track path is not a directory: {}",
                source.display()
            )));
        }
        let target = contained_path(&self.root, target_relative, false)?;
        if target.exists()
            && fs::canonicalize(&target).map_err(|error| AppError::io(&target, error))?
                != fs::canonicalize(&source).map_err(|error| AppError::io(&source, error))?
        {
            return Err(AppError::Collision(portable_relative(target_relative)));
        }
        let target_parent_relative = target_relative
            .parent()
            .ok_or_else(|| AppError::Validation("A track folder needs a parent.".into()))?;
        let target_parent = ensure_contained_directory(&self.root, target_parent_relative)?;
        if target_parent_relative != Path::new(SINGLES_DIRECTORY)
            && looks_like_track_root(&target_parent)
        {
            return Err(AppError::Collision(portable_relative(
                target_parent_relative,
            )));
        }
        fs::rename(&source, &target).map_err(|error| AppError::io(&target, error))?;
        if remove_empty_source {
            let source_parent = source_relative
                .parent()
                .ok_or_else(|| AppError::Validation("A track folder needs a parent.".into()))?;
            let _ = self.remove_empty_library_directory(source_parent);
        }
        Ok(())
    }

    fn rollback_track_move(&self, source_relative: &str, target_relative: &str) -> Result<()> {
        self.move_track_directory(source_relative, target_relative, true)
    }

    fn remove_empty_library_directory(&self, relative: &Path) -> Result<()> {
        if relative.as_os_str().is_empty()
            || relative == Path::new(SINGLES_DIRECTORY)
            || relative.components().count() != 1
        {
            return Ok(());
        }
        let directory = contained_path(&self.root, relative, false)?;
        if directory.is_dir() {
            match fs::remove_dir(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
                Err(error) => return Err(AppError::io(&directory, error)),
            }
        }
        Ok(())
    }

    fn reconcile_physical_library(&self) -> Result<()> {
        let identities = discover_track_identities(&self.root)?;
        let tracks = self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| !is_hidden_workspace_path(Path::new(&track.relative_path)))
            .collect::<Vec<_>>();
        let mut claimed = tracks
            .iter()
            .filter(|track| self.root.join(&track.relative_path).is_dir())
            .map(|track| track.relative_path.clone())
            .collect::<HashSet<_>>();

        for mut track in tracks {
            let original_path = track.relative_path.clone();
            let stored_exists = self.root.join(&original_path).is_dir();
            let mut moved_from = None;

            if stored_exists && !track.legacy && Path::new(&original_path).components().count() == 1
            {
                let leaf = Path::new(&original_path).file_name().ok_or_else(|| {
                    AppError::Data("Stored track path has no folder name.".into())
                })?;
                let parent = physical_library_parent(&track.library)?;
                let target = portable_relative(&parent.join(leaf));
                self.move_track_directory(&original_path, &target, false)?;
                claimed.remove(&original_path);
                claimed.insert(target.clone());
                moved_from = Some(original_path.clone());
                track.relative_path = target;
            } else if !stored_exists {
                let discovered = identities.get(&track.id).cloned().or_else(|| {
                    find_unclaimed_track_in_library(&self.root, &track.library, &claimed)
                        .ok()
                        .flatten()
                });
                if let Some(relative_path) = discovered {
                    claimed.insert(relative_path.clone());
                    track.relative_path = relative_path;
                    if track.legacy {
                        if let Some(title) = Path::new(&track.relative_path)
                            .file_name()
                            .and_then(|value| value.to_str())
                        {
                            track.fields.title = title.to_owned();
                        }
                    }
                } else {
                    continue;
                }
            }

            if let Some(inferred) = physical_library_from_relative(&track.relative_path)? {
                track.library = inferred;
            }
            if track.relative_path != original_path
                || track.library != self.persistence.track(&track.id)?.library
            {
                if let Err(error) = self.persistence.save_track(&track) {
                    if let Some(source) = moved_from {
                        if let Err(rollback) =
                            self.rollback_track_move(&track.relative_path, &source)
                        {
                            return Err(AppError::Data(format!(
                                "Library migration failed ({error}); folder rollback failed: {rollback}"
                            )));
                        }
                    }
                    return Err(error);
                }
            }
            if !track.legacy {
                self.write_track_identity(&track)?;
            }
        }
        Ok(())
    }

    fn rename_managed_release_evidence(
        &self,
        track: &TrackRecord,
        title: &str,
    ) -> Result<Vec<(EvidenceItem, EvidenceItem)>> {
        let root = self.track_root(track)?;
        let mut renamed = Vec::new();
        for previous in self
            .persistence
            .evidence(&track.id)?
            .into_iter()
            .filter(|item| {
                item.provenance == EvidenceProvenance::ManagedCopy
                    && matches!(
                        item.role,
                        EvidenceRole::ReleaseWav
                            | EvidenceRole::ReleaseMp3
                            | EvidenceRole::ReleaseMp4
                    )
            })
        {
            let result = (|| -> Result<Option<(EvidenceItem, EvidenceItem)>> {
                let planned = evidence::managed_relative_path(
                    title,
                    &previous.role,
                    Path::new(&previous.file_name),
                )?;
                let planned_portable = portable_relative(&planned);
                if planned_portable == previous.relative_path {
                    return Ok(None);
                }
                if self
                    .persistence
                    .evidence_by_relative_path(&track.id, &planned_portable)?
                    .is_some()
                {
                    return Err(AppError::Collision(planned_portable));
                }
                let source = contained_path(&root, Path::new(&previous.relative_path), true)?;
                let target = contained_path(&root, &planned, false)?;
                if target.exists() {
                    return Err(AppError::Collision(planned_portable));
                }
                fs::rename(&source, &target).map_err(|error| AppError::io(&target, error))?;
                let update_result = (|| -> Result<EvidenceItem> {
                    let mut updated = previous.clone();
                    updated.relative_path = planned_portable;
                    updated.file_name = target
                        .file_name()
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| {
                            AppError::Validation("Managed release file name is invalid.".into())
                        })?
                        .to_owned();
                    self.persistence.save_evidence(&track.id, &updated)?;
                    Ok(updated)
                })();
                match update_result {
                    Ok(updated) => Ok(Some((previous.clone(), updated))),
                    Err(error) => {
                        fs::rename(&target, &source).map_err(|rollback| {
                            AppError::Data(format!(
                                "Release metadata update failed ({error}); file rollback failed: {rollback}"
                            ))
                        })?;
                        Err(error)
                    }
                }
            })();
            match result {
                Ok(Some(pair)) => renamed.push(pair),
                Ok(None) => {}
                Err(error) => {
                    if let Err(rollback) = self.rollback_release_evidence_renames(track, &renamed) {
                        return Err(AppError::Data(format!(
                            "Release rename failed ({error}); earlier release rollback failed: {rollback}"
                        )));
                    }
                    return Err(error);
                }
            }
        }
        Ok(renamed)
    }

    fn rollback_release_evidence_renames(
        &self,
        track: &TrackRecord,
        renamed: &[(EvidenceItem, EvidenceItem)],
    ) -> Result<()> {
        if renamed.is_empty() {
            return Ok(());
        }
        let root = self.track_root(track)?;
        let mut errors = Vec::new();
        for (previous, updated) in renamed.iter().rev() {
            let source = contained_path(&root, Path::new(&updated.relative_path), false)?;
            let target = contained_path(&root, Path::new(&previous.relative_path), false)?;
            if source.is_file() && !target.exists() {
                if let Err(error) = fs::rename(&source, &target) {
                    errors.push(format!("{}: {error}", updated.relative_path));
                    continue;
                }
            }
            if let Err(error) = self.persistence.save_evidence(&track.id, previous) {
                errors.push(format!("{} metadata: {error}", previous.relative_path));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::Data(errors.join("; ")))
        }
    }

    fn migrate_legacy_release_evidence(&self, track: &TrackRecord) -> Result<bool> {
        if matches!(
            track.status,
            TrackStatus::Finalized | TrackStatus::Superseded
        ) {
            return Ok(false);
        }
        let root = self.track_root(track)?;
        let mut migrated = false;
        for previous in self
            .persistence
            .evidence(&track.id)?
            .into_iter()
            .filter(|item| {
                let relative = Path::new(&item.relative_path);
                let extension = relative
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(str::to_ascii_lowercase);
                item.provenance == EvidenceProvenance::ManagedCopy
                    && matches!(
                        item.role,
                        EvidenceRole::ReleaseWav
                            | EvidenceRole::ReleaseMp3
                            | EvidenceRole::ReleaseMp4
                    )
                    && relative.parent() == Some(Path::new("01_RELEASE"))
                    && relative.file_name().and_then(|value| value.to_str())
                        == Some(item.file_name.as_str())
                    && relative.file_stem().and_then(|value| value.to_str())
                        == Some("suno_final_export")
                    && extension.as_deref().is_some_and(|extension| {
                        item.role.allowed_extensions().contains(&extension)
                    })
            })
        {
            let Ok(planned) = evidence::managed_relative_path(
                &track.fields.title,
                &previous.role,
                Path::new(&previous.file_name),
            ) else {
                continue;
            };
            let planned_portable = portable_relative(&planned);
            if planned_portable == previous.relative_path
                || contained_path(&root, &planned, false)?.exists()
                || self
                    .persistence
                    .evidence_by_relative_path(&track.id, &planned_portable)?
                    .is_some()
            {
                continue;
            }
            let source = contained_path(&root, Path::new(&previous.relative_path), false)?;
            let metadata = match fs::symlink_metadata(&source) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(AppError::io(&source, error)),
            };
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                continue;
            }
            let target = contained_path(&root, &planned, false)?;
            fs::rename(&source, &target).map_err(|error| AppError::io(&target, error))?;
            let mut updated = previous.clone();
            updated.relative_path = planned_portable;
            updated.file_name = target
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    AppError::Validation("Managed release file name is invalid.".into())
                })?
                .to_owned();
            if let Err(error) = self.persistence.save_evidence(&track.id, &updated) {
                let _ = fs::rename(&target, &source);
                return Err(error);
            }
            migrated = true;
        }
        Ok(migrated)
    }

    fn track_root(&self, track: &TrackRecord) -> Result<PathBuf> {
        contained_path(&self.root, Path::new(&track.relative_path), true)
    }

    fn verified_evidence(&self, track: &TrackRecord) -> Result<Vec<EvidenceItem>> {
        let root = self.track_root(track)?;
        let mut result = Vec::new();
        for item in self.persistence.evidence(&track.id)? {
            let previous_verification = (
                item.sha256.clone(),
                item.size_bytes,
                item.verified,
                item.verification_error.clone(),
            );
            let verified = evidence::inspect(&root, item)?;
            let current_verification = (
                verified.sha256.clone(),
                verified.size_bytes,
                verified.verified,
                verified.verification_error.clone(),
            );
            // A read-only load must not reserialize legacy metadata_json just
            // because additive serde-default fields exist in a newer build.
            if current_verification != previous_verification {
                self.persistence.save_evidence(&track.id, &verified)?;
            }
            result.push(verified);
        }
        Ok(result)
    }

    /// Explicit revision creation is the sole backfill path for evidence that
    /// predates automatic WAV inspection. Normal loading intentionally never
    /// enriches historical/finalized records.
    fn prepare_revision_suno_analysis(
        &self,
        mut track: TrackRecord,
    ) -> Result<(TrackRecord, Vec<EvidenceItem>)> {
        let root = self.track_root(&track)?;
        let mut evidence_items = self.persistence.evidence(&track.id)?;
        let mut changed_items = Vec::new();
        for item in evidence_items
            .iter_mut()
            .filter(|item| item.role == EvidenceRole::SunoFinalExport)
        {
            let path = contained_path(&root, Path::new(&item.relative_path), false)?;
            let bytes_match = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    item.sha256.as_deref().is_some_and(|expected| {
                        sha256_file(&path).is_ok_and(|actual| actual == expected)
                    })
                }
                _ => false,
            };
            if !item.verified || item.verification_error.is_some() || !bytes_match {
                // Keep the persisted snapshot untouched, but exclude stale
                // evidence from facts prepared for the mutable revision.
                item.verified = false;
                continue;
            }
            let captured = evidence::capture_automatic_metadata(&path, item.metadata.clone());
            if validate_evidence_metadata(&item.role, &captured).is_err() {
                item.verified = false;
                continue;
            }
            if captured != item.metadata {
                item.metadata = captured;
                changed_items.push(item.clone());
            }
        }
        reconcile_evidence_derived_fields(&mut track, &evidence_items);
        Ok((track, changed_items))
    }

    fn validation_for(&self, mut track: TrackRecord) -> Result<ValidationResult> {
        validate_profile(&track.profile_snapshot, true)?;
        validate_track_fields(&track.fields)?;
        validate_required_production_range(&track)?;
        let root = self.track_root(&track)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let screening_artifact_error = self
            .ensure_current_audio_screening_artifacts(&track, &evidence)
            .err();
        let first = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        track.documents.current = documents::is_current(
            &root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &first.steps,
        )?;
        track.integrity = if root.join(integrity::HASH_FILE).is_file() {
            integrity::verify(&root)?
        } else {
            IntegrityState::default()
        };
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        self.persistence.save_track(&track)?;
        let mut blocking_items = Vec::new();
        if let Some(message) = workflow_version_mismatch(&track)? {
            blocking_items.push(message);
        }
        if let Some(error) = screening_artifact_error {
            blocking_items.push(format!("Pre-release audio screening: {error}"));
        }
        for step in &evaluation.steps {
            if matches!(
                step.status,
                StepStatus::Fail | StepStatus::Blocked | StepStatus::NotVerified
            ) {
                blocking_items.push(format!("{}: {:?}", step.id, step.status));
            }
            if step.status == StepStatus::NotApplicable
                && step
                    .na_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                blocking_items.push(format!("{}: N/A requires a reason", step.id));
            }
        }
        for deviation in &deviations {
            if deviation.blocking && !deviation.resolved {
                blocking_items.push(format!("Deviation: {}", deviation.description));
            }
        }
        for mismatch in &track.integrity.mismatch_files {
            blocking_items.push(format!("Integrity mismatch: {mismatch}"));
        }
        Ok(ValidationResult {
            valid: evaluation.missing.is_empty() && blocking_items.is_empty(),
            missing_items: evaluation.missing,
            blocking_items,
        })
    }

    fn detail_from_record(
        &self,
        mut track: TrackRecord,
        inspect_finalized: bool,
    ) -> Result<TrackDetail> {
        let immutable_snapshot = matches!(
            track.status,
            TrackStatus::Finalized | TrackStatus::Superseded
        );
        let stored_track_state = serde_json::to_vec(&track)?;
        if !immutable_snapshot && self.migrate_legacy_release_evidence(&track)? {
            mark_content_changed(&mut track);
            track.updated_at = now();
        }
        let root = self.track_root(&track)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let first = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        if !immutable_snapshot {
            track.documents.current = documents::is_current(
                &root,
                &track,
                &track.profile_snapshot,
                &evidence,
                &first.steps,
            )?;
        }
        let contains_large_evidence = evidence
            .iter()
            .any(|item| item.size_bytes > evidence::AUTOMATIC_HASH_LIMIT_BYTES);
        let inspected_integrity =
            if root.join(integrity::HASH_FILE).is_file() && !contains_large_evidence {
                integrity::verify(&root).unwrap_or_else(|_| IntegrityState {
                    generated: true,
                    verified: false,
                    file_count: 0,
                    verified_count: 0,
                    generated_at: None,
                    verified_at: Some(now()),
                    mismatch_files: vec![
                        "03_DOCUMENTATION/SHA256SUMS.txt (invalid or unreadable)".into()
                    ],
                })
            } else if !root.join(integrity::HASH_FILE).is_file() {
                IntegrityState::default()
            } else {
                track.integrity.clone()
            };
        if !immutable_snapshot || !inspected_integrity.verified {
            track.integrity = inspected_integrity.clone();
        }
        if inspect_finalized && track.status == TrackStatus::Finalized {
            let certificate_valid = certificate::verify(&root).is_ok();
            let evidence_valid = evidence.iter().all(|item| {
                item.verified && item.sha256.is_some() && item.verification_error.is_none()
            });
            if (!inspected_integrity.verified || !certificate_valid || !evidence_valid)
                && track.certificate.valid
            {
                invalidate_state(&mut track, "Documentation changed after finalization");
            }
        }
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let progress = workflow::progress(&track, &track.profile_snapshot, &evidence, &deviations)?;
        if track.status != TrackStatus::Finalized && track.status != TrackStatus::Superseded {
            track.status = if evaluation.missing.is_empty()
                && evaluation
                    .steps
                    .iter()
                    .all(|step| matches!(step.status, StepStatus::Pass | StepStatus::NotApplicable))
            {
                TrackStatus::Ready
            } else if track.status == TrackStatus::Draft {
                TrackStatus::Draft
            } else {
                TrackStatus::Active
            };
        }
        // Avoid normalizing old JSON records on a read-only load merely due to
        // new serde-default fields. Persist only a real semantic state change.
        if serde_json::to_vec(&track)? != stored_track_state {
            self.persistence.save_track(&track)?;
        }
        let cover_evidence_id = final_artwork_evidence_id(&evidence);
        let automation = workflow::automation_summary(&track, &evidence);
        let (external_timestamps, finalization_anchors, external_timestamp_summary) =
            self.external_timestamp_context(&track)?;
        Ok(TrackDetail {
            id: track.id.clone(),
            title: track.fields.title.clone(),
            relative_path: track.relative_path.clone(),
            status: track.status.clone(),
            updated_at: track.updated_at.clone(),
            progress,
            missing_count: evaluation.missing.len() as u32,
            certificate_valid: Some(track.certificate.valid),
            legacy: Some(track.legacy),
            cover_evidence_id,
            library: track.library.clone(),
            workflow_id: track.workflow_id.clone(),
            workflow_version: track.workflow_version.clone(),
            profile_snapshot: track.profile_snapshot.clone(),
            automation,
            fields: track.fields.clone(),
            audio_screening: AudioScreeningSummary::from(&track.audio_screening),
            steps: evaluation.steps,
            evidence,
            external_timestamps,
            external_timestamp_summary,
            finalization_anchors,
            documents: track.documents.clone(),
            integrity: track.integrity.clone(),
            certificate: track.certificate.clone(),
            blocking_deviations: deviations,
            missing_items: evaluation.missing,
        })
    }

    fn detail_from_stored_record(&self, track: TrackRecord) -> Result<TrackDetail> {
        let evidence = self.persistence.evidence(&track.id)?;
        let deviations = self.persistence.deviations(&track.id)?;
        let stored = self.persistence.stored_steps(&track.id)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let progress = workflow::progress(&track, &track.profile_snapshot, &evidence, &deviations)?;
        let cover_evidence_id = final_artwork_evidence_id(&evidence);
        let automation = workflow::automation_summary(&track, &evidence);
        let (external_timestamps, finalization_anchors, external_timestamp_summary) =
            self.external_timestamp_context(&track)?;
        Ok(TrackDetail {
            id: track.id.clone(),
            title: track.fields.title.clone(),
            relative_path: track.relative_path.clone(),
            status: track.status.clone(),
            updated_at: track.updated_at.clone(),
            progress,
            missing_count: evaluation.missing.len() as u32,
            certificate_valid: Some(track.certificate.valid),
            legacy: Some(track.legacy),
            cover_evidence_id,
            library: track.library.clone(),
            workflow_id: track.workflow_id.clone(),
            workflow_version: track.workflow_version.clone(),
            profile_snapshot: track.profile_snapshot.clone(),
            automation,
            fields: track.fields.clone(),
            audio_screening: AudioScreeningSummary::from(&track.audio_screening),
            steps: evaluation.steps,
            evidence,
            external_timestamps,
            external_timestamp_summary,
            finalization_anchors,
            documents: track.documents.clone(),
            integrity: track.integrity.clone(),
            certificate: track.certificate.clone(),
            blocking_deviations: deviations,
            missing_items: evaluation.missing,
        })
    }

    fn external_timestamp_context(
        &self,
        track: &TrackRecord,
    ) -> Result<(
        Vec<ExternalTimestampRecord>,
        Vec<FinalizationAnchor>,
        ExternalTimestampSummary,
    )> {
        let root = self.track_root(track)?;
        let mut records = self.persistence.external_timestamps(&track.id)?;
        for record in &mut records {
            match external_timestamp::verify_published_record(&root, record) {
                Ok(()) => {
                    record.integrity_verified = true;
                    record.integrity_issues.clear();
                }
                Err(error) => {
                    record.integrity_verified = false;
                    record.integrity_issues = vec![error.to_string()];
                }
            }
        }
        if track.status != TrackStatus::Finalized || !track.certificate.valid {
            return Ok((records, Vec::new(), ExternalTimestampSummary::default()));
        }
        if let Some(certificate_id) = track.certificate.certificate_id.as_deref() {
            // A track can retain archived addenda from prior revisions. The
            // live finalized view must never present those as evidence for the
            // current certificate snapshot.
            records.retain(|record| record.certificate_id == certificate_id);
        }
        let summary = self.timestamp_summary_for_record(track, &records)?;
        let anchors = if certificate::verify(&root).is_ok() {
            external_timestamp::finalization_anchors(&root).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok((records, anchors, summary))
    }

    fn timestamp_summary_for_record(
        &self,
        track: &TrackRecord,
        records: &[ExternalTimestampRecord],
    ) -> Result<ExternalTimestampSummary> {
        let Some(certificate_id) = track.certificate.certificate_id.as_deref() else {
            return Ok(ExternalTimestampSummary::default());
        };
        if let Some(summary) = self
            .persistence
            .timestamp_attachment_summary(&track.id, certificate_id)?
        {
            return Ok(summary);
        }
        let Some(record) = records
            .iter()
            .rev()
            .find(|record| record.certificate_id == certificate_id)
        else {
            return Ok(ExternalTimestampSummary::default());
        };
        // Historical sidecars only record addendum-file integrity, not a
        // provider cryptographic result. A missing summary therefore never
        // promotes legacy or recovered evidence to VERIFIED.
        let status = record
            .provider_metadata
            .as_ref()
            .map(|metadata| match metadata.verification_result {
                ExternalTimestampStatus::Verified => ExternalTimestampStatus::Attached,
                status => status,
            })
            .unwrap_or(ExternalTimestampStatus::Attached);
        Ok(ExternalTimestampSummary {
            status,
            message: if record.provider_metadata.is_some() {
                "External timestamp evidence is attached; provider verification status was recovered from its immutable record."
                    .into()
            } else {
                "Legacy manually recorded timestamp evidence is attached; it has not been automatically promoted to provider-verified evidence."
                    .into()
            },
            provider: record.provider.clone(),
            record_id: Some(record.id.clone()),
            updated_at: Some(record.imported_at.clone()),
        })
    }
}

struct LegacyInspection {
    recognized_folders: Vec<String>,
    documents: Vec<String>,
    evidence_files: Vec<String>,
    hash_manifest_present: bool,
    has_managed_document_collision: bool,
}

fn inspect_legacy(root: &Path) -> Result<LegacyInspection> {
    let known: HashSet<&str> = [
        "01_RELEASE",
        "02_SUNO",
        "03_DOCUMENTATION",
        "04_LICENSES",
        "05_ARTWORK",
        "06_CERTIFICATE",
    ]
    .into_iter()
    .collect();
    let mut recognized_folders = Vec::new();
    let mut documents_found = Vec::new();
    let mut evidence_files = Vec::new();
    let mut has_collision = false;
    for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_type().is_symlink() {
            return Err(AppError::Symlink(entry.path().display().to_string()));
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| AppError::PathEscape)?;
        let portable = portable_relative(relative);
        if entry.depth() == 1 && entry.file_type().is_dir() && known.contains(portable.as_str()) {
            recognized_folders.push(portable.clone());
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if documents::DOCUMENT_PATHS.contains(&portable.as_str()) {
            documents_found.push(portable.clone());
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            if !content.starts_with("<!-- suno-documentation-manager:template-v1 -->\n")
                && !content.starts_with("# suno-documentation-manager:template-v1\n")
            {
                has_collision = true;
            }
        } else if portable != integrity::HASH_FILE
            && portable != certificate::PDF_FILE
            && !portable.starts_with(".archive/")
            && !portable.starts_with(".summary/")
            && !portable.starts_with("06_CERTIFICATE/")
        {
            evidence_files.push(portable);
        }
    }
    recognized_folders.sort();
    documents_found.sort();
    evidence_files.sort();
    Ok(LegacyInspection {
        recognized_folders,
        documents: documents_found,
        evidence_files,
        hash_manifest_present: root.join(integrity::HASH_FILE).is_file(),
        has_managed_document_collision: has_collision,
    })
}

fn infer_legacy_role(relative: &str) -> EvidenceRole {
    let value = relative.to_ascii_lowercase();
    if value.contains("ai_original") {
        EvidenceRole::AiArtworkOriginal
    } else if value.contains("ai_edited") {
        EvidenceRole::AiArtworkEdited
    } else if value.contains("_final")
        && [".png", ".jpg", ".jpeg"]
            .iter()
            .any(|extension| value.ends_with(extension))
    {
        EvidenceRole::FinalArtwork
    } else if value.starts_with("01_release/") && value.ends_with(".wav") {
        EvidenceRole::ReleaseWav
    } else if value.starts_with("01_release/") && value.ends_with(".mp3") {
        EvidenceRole::ReleaseMp3
    } else if value.starts_with("01_release/") && value.ends_with(".mp4") {
        EvidenceRole::ReleaseMp4
    } else if value.starts_with("02_suno/") && value.ends_with(".zip") {
        EvidenceRole::SunoProjectZip
    } else if value.starts_with("02_suno/")
        && [".wav", ".mp3", ".flac", ".m4a", ".aiff", ".aif", ".ogg"]
            .iter()
            .any(|extension| value.ends_with(extension))
    {
        EvidenceRole::SunoFinalExport
    } else {
        EvidenceRole::Other
    }
}

fn summary_from_detail(detail: &TrackDetail) -> TrackSummary {
    TrackSummary {
        id: detail.id.clone(),
        title: detail.title.clone(),
        relative_path: detail.relative_path.clone(),
        status: detail.status.clone(),
        updated_at: detail.updated_at.clone(),
        progress: detail.progress,
        missing_count: detail.missing_count,
        certificate_valid: detail.certificate_valid,
        legacy: detail.legacy,
        cover_evidence_id: detail.cover_evidence_id.clone(),
        library: detail.library.clone(),
    }
}

fn final_artwork_evidence_id(evidence: &[EvidenceItem]) -> Option<String> {
    evidence
        .iter()
        .find(|item| {
            item.role == EvidenceRole::FinalArtwork
                && item.verified
                && item.sha256.is_some()
                && item.verification_error.is_none()
        })
        .map(|item| item.id.clone())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn provider_response_staging_path(track_root: &Path, extension: &str) -> Result<PathBuf> {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if extension.is_empty()
        || !EvidenceRole::ExternalTimestamp
            .allowed_extensions()
            .contains(&extension.as_str())
    {
        return Err(AppError::Validation(
            "Timestamp provider returned an unsupported evidence format.".into(),
        ));
    }
    let parent = ensure_contained_directory(
        track_root,
        Path::new(".archive/timestamp-provider-response"),
    )?;
    Ok(parent.join(format!("response-{}.{}", Uuid::new_v4(), extension)))
}

fn referenced_finalization_snapshot_id(track: &TrackRecord, certificate_id: &str) -> String {
    track
        .certificate
        .finalization_snapshot_id
        .clone()
        // Pre-schema tracks have no separately persisted snapshot UUID. The
        // Certificate ID is already immutable and archived, so a namespaced
        // fallback keeps the association explicit without modifying history.
        .unwrap_or_else(|| format!("legacy-finalization-snapshot:{certificate_id}"))
}

fn workflow_version_mismatch(track: &TrackRecord) -> Result<Option<String>> {
    let current = workflow::config()?;
    if track.workflow_id == current.id && track.workflow_version == current.version {
        return Ok(None);
    }
    Ok(Some(format!(
        "Workflow {} {} is stored for this track, but {} {} is current. Re-evaluate the track explicitly before generating documents, hashes, or a certificate.",
        track.workflow_id, track.workflow_version, current.id, current.version
    )))
}

fn ensure_current_workflow(track: &TrackRecord) -> Result<()> {
    if let Some(message) = workflow_version_mismatch(track)? {
        return Err(AppError::Validation(message));
    }
    Ok(())
}

fn invalidate_state(track: &mut TrackRecord, reason: &str) {
    track.certificate.valid = false;
    track.certificate.invalidated_at = Some(now());
    track.certificate.invalidation_reason = Some(reason.into());
}

fn mark_content_changed(track: &mut TrackRecord) {
    track.documents.current = false;
    track.integrity = IntegrityState::default();
}

/// Treats a valid date from the current verified Suno export as authoritative
/// for the Suno and production facts. The last-editing date follows the WAV
/// only after the user confirms that no desktop editing took place.
fn reconcile_evidence_derived_fields(track: &mut TrackRecord, evidence: &[EvidenceItem]) -> bool {
    let suno = evidence.iter().find(|item| {
        item.role == EvidenceRole::SunoFinalExport
            && item.verified
            && item.verification_error.is_none()
            && item.sha256.is_some()
            && item.metadata.suno_studio_detected
            && !item.metadata.suno_created_timestamp.trim().is_empty()
            && !item.metadata.suno_created_date.trim().is_empty()
    });
    let derived = suno.map(|item| EvidenceDerivedField {
        value: item.metadata.suno_created_date.clone(),
        original_value: item.metadata.suno_created_timestamp.clone(),
        evidence_id: item.id.clone(),
        evidence_sha256: item.sha256.clone().unwrap_or_default(),
    });
    let derived_id = suno.and_then(|item| {
        let id = item.metadata.suno_id.trim();
        (!id.is_empty()).then(|| EvidenceDerivedField {
            value: id.to_owned(),
            // The structured UUID is already normalized by the strict WAV
            // parser. Keeping it as the original value lets us distinguish
            // this ID-specific origin from the timestamp-derived date facts.
            original_value: id.to_owned(),
            evidence_id: item.id.clone(),
            evidence_sha256: item.sha256.clone().unwrap_or_default(),
        })
    });

    let mut changed = reconcile_evidence_derived_id(
        &mut track.fields.suno_final_generation_id,
        &mut track.field_origins.suno_final_generation_id,
        derived_id.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.suno_final_generation_date,
        &mut track.field_origins.suno_final_generation_date,
        derived.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.production_end_date,
        &mut track.field_origins.production_end_date,
        derived.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.suno_download_export_date,
        &mut track.field_origins.suno_download_export_date,
        derived.as_ref(),
    );
    let final_export_derived = (track.fields.post_export_editing_performed == Some(false))
        .then_some(derived.as_ref())
        .flatten();
    changed |= reconcile_derived_value(
        &mut track.fields.final_export_date,
        &mut track.field_origins.final_export_date,
        final_export_derived,
    );
    changed
}

fn reconcile_derived_value(
    field: &mut String,
    origin: &mut Option<EvidenceDerivedField>,
    derived: Option<&EvidenceDerivedField>,
) -> bool {
    let previous_field = field.clone();
    let previous_origin = origin.clone();

    // A value that no longer equals the last automatic assignment has been
    // edited by the user and immediately ceases to be system-owned.
    if origin
        .as_ref()
        .is_some_and(|recorded| field != &recorded.value)
    {
        *origin = None;
    }

    if let Some(value) = derived {
        // Valid metadata wins over a submitted fallback value. The UI also
        // renders these fields read-only, while this assignment enforces the
        // same invariant for every native caller.
        *field = value.value.clone();
        *origin = Some(value.clone());
    } else {
        if origin
            .as_ref()
            .is_some_and(|recorded| field == &recorded.value)
        {
            field.clear();
        }
        *origin = None;
    }

    *field != previous_field || *origin != previous_origin
}

/// Unlike the date facts, a final-generation ID entered by the user is never
/// replaced by WAV metadata. An automatic ID is nevertheless kept tied to its
/// exact evidence so replacement and removal update or clear only that
/// system-owned value.
fn reconcile_evidence_derived_id(
    field: &mut String,
    origin: &mut Option<EvidenceDerivedField>,
    derived: Option<&EvidenceDerivedField>,
) -> bool {
    let previous_field = field.clone();
    let previous_origin = origin.clone();

    // A changed automatic value is a user-confirmed override. The frontend
    // sends a full draft, so equality (rather than patch presence) is the only
    // reliable way to preserve automatic ownership across unrelated saves.
    if origin
        .as_ref()
        .is_some_and(|recorded| field != &recorded.value)
    {
        *origin = None;
    }

    match (origin.as_ref(), derived) {
        // Only values previously owned by this automation follow a replacement
        // or are cleared when the Suno evidence disappears.
        (Some(_), Some(value)) => {
            *field = value.value.clone();
            *origin = Some(value.clone());
        }
        (Some(recorded), None) => {
            if field == &recorded.value {
                field.clear();
            }
            *origin = None;
        }
        // A non-empty field without an automatic origin is a manual value and
        // must never be overwritten. A blank field is intentionally filled.
        (None, Some(value)) if field.trim().is_empty() => {
            *field = value.value.clone();
            *origin = Some(value.clone());
        }
        _ => {}
    }

    *field != previous_field || *origin != previous_origin
}

fn directory_is_empty_or_missing(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AppError::Symlink(path.display().to_string()));
    }
    Ok(fs::read_dir(path)
        .map_err(|error| AppError::io(path, error))?
        .next()
        .is_none())
}

fn matching_revision_certificate(
    track_root: &Path,
    revision_relative: &Path,
    track: &TrackRecord,
) -> Result<Option<PathBuf>> {
    let metadata_path =
        contained_path(track_root, &revision_relative.join("revision.json"), false)?;
    let candidate = contained_path(track_root, &revision_relative.join("certificate"), false)?;
    if !candidate.is_dir() || !metadata_path.is_file() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(&metadata_path).map_err(|error| AppError::io(&metadata_path, error))?,
    )?;
    if value.get("track_id").and_then(|value| value.as_str()) != Some(track.id.as_str()) {
        return Ok(None);
    }
    let archived_certificate_id = value
        .get("previous_certificate")
        .and_then(|value| value.get("certificateId"))
        .and_then(|value| value.as_str());
    if archived_certificate_id != track.certificate.certificate_id.as_deref() {
        return Ok(None);
    }
    Ok(Some(candidate))
}

fn finalized_artifacts_need_revision_restore(
    track_root: &Path,
    live_certificate: &Path,
    live_pdf: &Path,
) -> Result<bool> {
    if directory_is_empty_or_missing(live_certificate)? {
        return Ok(true);
    }
    // A malformed live set is ordinary certificate invalidation, not proof of
    // an interrupted revision. Only a readable current-format set establishes
    // that the root PDF belongs to this finalized snapshot and needs recovery.
    Ok(!live_pdf.is_file() && matches!(certificate::expects_pdf(track_root), Ok(true)))
}

fn restore_revision_artifacts(
    live_certificate: &Path,
    live_pdf: &Path,
    archived_certificate: &Path,
) -> Result<bool> {
    let revision_directory = archived_certificate
        .parent()
        .ok_or_else(|| AppError::Data("Revision certificate has no archive directory.".into()))?;
    let archived_pdf = revision_directory.join(certificate::PDF_FILE);
    let restore_certificate = directory_is_empty_or_missing(live_certificate)?;
    let archived_pdf_exists =
        regular_file_if_present(&archived_pdf, "The archived technical documentation PDF")?;
    let live_pdf_exists =
        regular_file_if_present(live_pdf, "The live technical documentation PDF")?;
    let restore_pdf = archived_pdf_exists && !live_pdf_exists;
    if archived_pdf_exists
        && live_pdf_exists
        && sha256_file(&archived_pdf)? != sha256_file(live_pdf)?
    {
        return Err(AppError::Validation(
            "The live and archived technical documentation PDFs do not match.".into(),
        ));
    }
    if !restore_certificate && !restore_pdf {
        return Ok(false);
    }

    if restore_certificate && live_certificate.exists() {
        fs::remove_dir(live_certificate).map_err(|error| AppError::io(live_certificate, error))?;
    }
    let mut certificate_moved = false;
    let mut pdf_moved = false;
    let restore_result = (|| -> Result<()> {
        if restore_certificate {
            fs::rename(archived_certificate, live_certificate)
                .map_err(|error| AppError::io(live_certificate, error))?;
            certificate_moved = true;
        }
        if restore_pdf {
            copy_new(&archived_pdf, live_pdf)?;
            pdf_moved = true;
        }
        Ok(())
    })();
    if let Err(cause) = restore_result {
        let mut rollback_errors = Vec::new();
        if pdf_moved {
            if let Err(error) = fs::remove_file(live_pdf) {
                rollback_errors.push(format!("PDF rollback failed: {error}"));
            }
        }
        if certificate_moved {
            if let Err(error) = fs::rename(live_certificate, archived_certificate) {
                rollback_errors.push(format!("certificate rollback failed: {error}"));
            }
        }
        if restore_certificate && !live_certificate.exists() {
            if let Err(error) = fs::create_dir(live_certificate) {
                rollback_errors.push(format!("live directory recovery failed: {error}"));
            }
        }
        if rollback_errors.is_empty() {
            return Err(cause);
        }
        return Err(AppError::Data(format!(
            "Revision recovery failed ({cause}); {}",
            rollback_errors.join("; ")
        )));
    }
    Ok(true)
}

fn regular_file_if_present(path: &Path, label: &str) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(AppError::Symlink(path.display().to_string()))
        }
        Ok(metadata) if !metadata.is_file() => Err(AppError::Validation(format!(
            "{label} is not a regular file."
        ))),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn rollback_removed_file(archived: &Path, original: &Path, cause: AppError) -> AppError {
    match fs::rename(archived, original) {
        Ok(()) => cause,
        Err(rollback_error) => AppError::Data(format!(
            "Removal failed ({cause}); file rollback failed: {rollback_error}"
        )),
    }
}

struct CertificateRollback {
    error: AppError,
    complete: bool,
}

fn rollback_certificate_set(track_root: &Path, cause: AppError) -> CertificateRollback {
    let certificate_dir = match contained_path(
        track_root,
        Path::new(certificate::CERTIFICATE_DIR),
        false,
    ) {
        Ok(path) => path,
        Err(rollback_error) => {
            return CertificateRollback {
                    error: AppError::Data(format!(
                        "Finalization failed ({cause}); certificate rollback path failed: {rollback_error}"
                    )),
                    complete: false,
            };
        }
    };
    let pdf = match contained_path(track_root, Path::new(certificate::PDF_FILE), false) {
        Ok(path) => path,
        Err(rollback_error) => {
            return CertificateRollback {
                error: AppError::Data(format!(
                    "Finalization failed ({cause}); PDF rollback path failed: {rollback_error}"
                )),
                complete: false,
            };
        }
    };
    if pdf.exists() {
        if let Err(rollback_error) = fs::remove_file(&pdf) {
            return CertificateRollback {
                error: AppError::Data(format!(
                    "Finalization failed ({cause}); PDF cleanup failed: {rollback_error}"
                )),
                complete: false,
            };
        }
    }
    if certificate_dir.exists() {
        if let Err(rollback_error) = fs::remove_dir_all(&certificate_dir) {
            return CertificateRollback {
                error: AppError::Data(format!(
                    "Finalization failed ({cause}); certificate cleanup failed: {rollback_error}"
                )),
                complete: false,
            };
        }
    }
    if let Err(rollback_error) = fs::create_dir(&certificate_dir) {
        return CertificateRollback {
            error: AppError::Data(format!(
                "Finalization failed ({cause}); empty certificate directory recovery failed: {rollback_error}"
            )),
            complete: false,
        };
    }
    CertificateRollback {
        error: cause,
        complete: true,
    }
}

fn rollback_revision_state(
    live_certificate: &Path,
    archived_certificate: &Path,
    live_pdf: &Path,
    archived_pdf: &Path,
    revision_directory: &Path,
    certificate_existed: bool,
    pdf_moved: bool,
    cause: AppError,
) -> AppError {
    let mut rollback_errors = Vec::new();
    let mut pdf_restored = !pdf_moved;
    if pdf_moved {
        if live_pdf.exists() {
            rollback_errors.push(format!(
                "PDF rollback would overwrite {}",
                live_pdf.display()
            ));
        } else {
            match copy_new(archived_pdf, live_pdf) {
                Ok(()) => pdf_restored = true,
                Err(error) => rollback_errors.push(format!("PDF rollback failed: {error}")),
            }
        }
    }

    let mut certificate_restored = !certificate_existed;
    if certificate_existed && pdf_restored {
        let live_can_be_replaced = match directory_is_empty_or_missing(live_certificate) {
            Ok(true) => {
                if live_certificate.exists() {
                    match fs::remove_dir(live_certificate) {
                        Ok(()) => true,
                        Err(error) => {
                            rollback_errors
                                .push(format!("live certificate cleanup failed: {error}"));
                            false
                        }
                    }
                } else {
                    true
                }
            }
            Ok(false) => {
                rollback_errors.push(format!(
                    "certificate rollback would overwrite {}",
                    live_certificate.display()
                ));
                false
            }
            Err(error) => {
                rollback_errors.push(format!("live certificate validation failed: {error}"));
                false
            }
        };
        if live_can_be_replaced {
            match fs::rename(archived_certificate, live_certificate) {
                Ok(()) => certificate_restored = true,
                Err(error) => rollback_errors.push(format!("certificate rollback failed: {error}")),
            }
        }
    } else if certificate_existed {
        certificate_restored = false;
    }

    if pdf_restored && certificate_restored {
        if pdf_moved && archived_pdf.exists() {
            if let Err(error) = fs::remove_file(archived_pdf) {
                rollback_errors.push(format!("archived PDF cleanup failed: {error}"));
            }
        }
        if !certificate_existed && archived_certificate.exists() {
            if let Err(error) = fs::remove_dir_all(archived_certificate) {
                rollback_errors.push(format!("staged certificate cleanup failed: {error}"));
            }
        }
        if rollback_errors.is_empty() && revision_directory.exists() {
            if let Err(error) = fs::remove_dir_all(revision_directory) {
                rollback_errors.push(format!("staging cleanup failed: {error}"));
            }
        }
    }

    if rollback_errors.is_empty() {
        cause
    } else {
        AppError::Data(format!(
            "Revision failed ({cause}); {}",
            rollback_errors.join("; ")
        ))
    }
}

/// `AUDIO_SCREENING` is system-owned and may be moved only as a whole. The
/// caller has already resolved both paths under the managed track root; this
/// helper still refuses links and collisions before restoring an interrupted
/// revision transaction.
fn restore_audio_screening_directory(staged: &Path, live: &Path) -> Result<()> {
    if !staged.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(staged).map_err(|error| AppError::io(staged, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AppError::Symlink(staged.display().to_string()));
    }
    if live.exists() {
        return Err(AppError::Collision(live.display().to_string()));
    }
    let parent = live.parent().ok_or_else(|| {
        AppError::Validation("Audio-screening directory has no live parent.".into())
    })?;
    fs::create_dir_all(parent).map_err(|error| AppError::io(parent, error))?;
    fs::rename(staged, live).map_err(|error| AppError::io(live, error))
}

fn regular_directory_if_present(path: &Path, label: &str) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(AppError::Symlink(path.display().to_string()))
        }
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(AppError::Validation(format!("{label} is not a directory."))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::io(path, error)),
    }
}

fn cleanup_revision_staging(stage: &Path, cause: AppError) -> AppError {
    if !stage.exists() {
        return cause;
    }
    match fs::remove_dir_all(stage) {
        Ok(()) => cause,
        Err(error) => AppError::Data(format!(
            "Revision staging failed ({cause}); cleanup failed: {error}"
        )),
    }
}

fn validate_short_text(name: &str, value: &str, max: usize, required: bool) -> Result<()> {
    let trimmed = value.trim();
    if required && trimmed.is_empty() {
        return Err(AppError::Validation(format!("{name} is required.")));
    }
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!(
            "{name} is invalid or too long."
        )));
    }
    Ok(())
}

fn validate_multiline_text(name: &str, value: &str, max: usize) -> Result<()> {
    if value.chars().count() > max
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(AppError::Validation(format!(
            "{name} is invalid or too long."
        )));
    }
    Ok(())
}

fn validate_text_list(
    name: &str,
    values: &[String],
    max_items: usize,
    max_item: usize,
) -> Result<()> {
    if values.len() > max_items {
        return Err(AppError::Validation(format!(
            "{name} contains too many entries."
        )));
    }
    for value in values {
        validate_multiline_text(name, value, max_item)?;
    }
    Ok(())
}

fn parse_date(name: &str, value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("{name} must use YYYY-MM-DD.")))
}

fn subscription_coverage_end(
    coverage_start: &str,
    billing_cycle: SubscriptionBillingCycle,
) -> Result<String> {
    let start = parse_date("Subscription coverage start", coverage_start)?;
    let months = match billing_cycle {
        SubscriptionBillingCycle::Monthly => 1,
        SubscriptionBillingCycle::Annual => 12,
    };
    let next_period_start = start
        .checked_add_months(Months::new(months))
        .ok_or_else(|| {
            AppError::Validation(
                "Subscription coverage date is outside the supported range.".into(),
            )
        })?;
    let end = next_period_start.pred_opt().ok_or_else(|| {
        AppError::Validation("Subscription coverage date is outside the supported range.".into())
    })?;
    Ok(end.format("%Y-%m-%d").to_string())
}

fn validate_optional_date(name: &str, value: &str) -> Result<()> {
    if !value.trim().is_empty() {
        parse_date(name, value)?;
    }
    Ok(())
}

fn validate_date_range(name: &str, start: &str, end: &str) -> Result<()> {
    let start_date = parse_date(&format!("{name} start"), start)?;
    let end_date = parse_date(&format!("{name} end"), end)?;
    if end_date < start_date {
        return Err(AppError::Validation(format!(
            "{name} end cannot be before its start."
        )));
    }
    Ok(())
}

fn validate_profile(profile: &Profile, complete: bool) -> Result<()> {
    for (name, value) in [
        ("Artist name", profile.artist_name.as_str()),
        ("Suno profile name", profile.suno_profile_name.as_str()),
        ("Suno handle", profile.suno_handle.as_str()),
        ("Suno plan", profile.suno_plan.as_str()),
        (
            "Default AI image service",
            profile.default_ai_image_service.as_str(),
        ),
        ("Disclosure text", profile.disclosure_text.as_str()),
    ] {
        validate_short_text(name, value, 500, complete)?;
    }
    validate_short_text(
        "Subscription start date",
        &profile.subscription_start_date,
        10,
        complete,
    )?;
    validate_optional_date("Subscription start date", &profile.subscription_start_date)?;
    if !matches!(
        profile.artwork_transparency_policy.as_str(),
        "always" | "per_artwork" | "none"
    ) {
        return Err(AppError::Validation(
            "Artwork transparency policy is invalid.".into(),
        ));
    }
    Ok(())
}

fn validate_track_fields(fields: &crate::model::TrackFields) -> Result<()> {
    let normalized_fields = fields.normalized_conditionals();
    let fields = &normalized_fields;
    validate_track_title(&fields.title)?;
    for (name, value, max) in [
        ("Suno model", fields.suno_model.as_str(), 200),
        (
            "Suno plan at generation",
            fields.suno_plan_at_generation.as_str(),
            200,
        ),
        (
            "Legacy Suno plan at creation",
            fields.legacy_suno_plan_at_creation.as_str(),
            200,
        ),
        ("Audio AI system", fields.audio_ai_system.as_str(), 500),
        (
            "Other Suno lyrics/structure content type",
            fields.suno_lyrics_other_content_type.as_str(),
            1000,
        ),
        ("AI image service", fields.ai_image_service.as_str(), 500),
        ("Disclosure text", fields.disclosure_text.as_str(), 80),
    ] {
        validate_short_text(name, value, max, false)?;
    }
    for (name, value, max) in [
        ("Lyrics", fields.lyrics_text.as_str(), 1_000_000),
        (
            "Suno lyrics/structure field content",
            fields.suno_lyrics_field_text.as_str(),
            1_000_000,
        ),
        (
            "Suno style prompt",
            fields.suno_style_prompt.as_str(),
            100_000,
        ),
        (
            "External audio source",
            fields.external_audio_source.as_str(),
            4000,
        ),
        (
            "External audio ownership",
            fields.external_audio_ownership.as_str(),
            4000,
        ),
        ("Own audio source", fields.own_audio_source.as_str(), 4000),
        (
            "Own audio ownership",
            fields.own_audio_ownership.as_str(),
            4000,
        ),
        (
            "Sample source",
            fields.third_party_sample_source.as_str(),
            4000,
        ),
        (
            "Sample ownership",
            fields.third_party_sample_ownership.as_str(),
            4000,
        ),
        (
            "Human editing details",
            fields.human_editing_details.as_str(),
            20_000,
        ),
        (
            "Post-export editing details",
            fields.post_export_editing_details.as_str(),
            20_000,
        ),
        (
            "Code-audio post-processing note",
            fields.code_audio_post_processing_note.as_str(),
            20_000,
        ),
        (
            "Human artwork process notes",
            fields.human_artwork_process_notes.as_str(),
            20_000,
        ),
        (
            "Custom artwork change",
            fields.custom_artwork_change.as_str(),
            20_000,
        ),
        (
            "Real-person note",
            fields.real_person_notes.as_str(),
            20_000,
        ),
        ("Real-event note", fields.real_event_notes.as_str(), 20_000),
        ("Trademark note", fields.trademark_notes.as_str(), 20_000),
        (
            "Audio disclosure text",
            fields.audio_disclosure_text.as_str(),
            20_000,
        ),
        (
            "Audio disclosure reason",
            fields.audio_disclosure_reason.as_str(),
            20_000,
        ),
        ("Release notes", fields.release_notes.as_str(), 20_000),
    ] {
        validate_multiline_text(name, value, max)?;
    }
    for (name, values) in [
        (
            "Code-audio post-processing operations",
            fields.code_audio_post_processing_operations.as_slice(),
        ),
        (
            "Human artwork process operations",
            fields.human_artwork_process_operations.as_slice(),
        ),
        (
            "Human artwork modifications",
            fields.human_artwork_modifications.as_slice(),
        ),
        (
            "Audio disclosure locations",
            fields.audio_disclosure_locations.as_slice(),
        ),
    ] {
        validate_text_list(name, values, 100, 4_000)?;
    }
    for (name, value) in [
        ("Production start", fields.production_start_date.as_str()),
        ("Production end", fields.production_end_date.as_str()),
        ("Last editing date", fields.final_export_date.as_str()),
        (
            "Final generation date",
            fields.suno_final_generation_date.as_str(),
        ),
        (
            "Suno download/export date",
            fields.suno_download_export_date.as_str(),
        ),
    ] {
        validate_optional_date(name, value)?;
    }
    if !fields.production_start_date.is_empty() && !fields.production_end_date.is_empty() {
        validate_date_range(
            "Production period",
            &fields.production_start_date,
            &fields.production_end_date,
        )?;
    }
    if !fields.final_export_date.is_empty() && !fields.production_start_date.is_empty() {
        let start = parse_date("Production start", &fields.production_start_date)?;
        let export = parse_date("Last editing date", &fields.final_export_date)?;
        if export < start {
            return Err(AppError::Validation(
                "Last editing date cannot be before production start.".into(),
            ));
        }
    }
    if !fields.lyrics_source.is_empty()
        && !matches!(
            fields.lyrics_source.as_str(),
            "instrumental" | "human" | "suno" | "mixed"
        )
    {
        return Err(AppError::Validation("Lyrics source is invalid.".into()));
    }
    if !fields.artwork_origin.is_empty()
        && !matches!(
            fields.artwork_origin.as_str(),
            "none" | "human" | "ai_generated" | "ai_assisted"
        )
    {
        return Err(AppError::Validation("Artwork origin is invalid.".into()));
    }
    if !fields.suno_project_url.trim().is_empty() {
        let url = Url::parse(fields.suno_project_url.trim())
            .map_err(|_| AppError::Validation("Suno project URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Suno project URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

fn validate_evidence_metadata(role: &EvidenceRole, metadata: &EvidenceMetadata) -> Result<()> {
    for (name, value, max) in [
        (
            "Evidence document title",
            metadata.document_title.as_str(),
            1000,
        ),
        ("Evidence provider", metadata.provider.as_str(), 1000),
        ("Evidence source URL", metadata.source_url.as_str(), 4000),
        (
            "Evidence factual note",
            metadata.factual_note.as_str(),
            20_000,
        ),
        (
            "Applicable production period",
            metadata.applicable_production_period.as_str(),
            1000,
        ),
        ("Timestamp type", metadata.timestamp_type.as_str(), 200),
        ("Timestamp value", metadata.external_timestamp.as_str(), 200),
        ("Referenced hash", metadata.referenced_hash.as_str(), 512),
        (
            "Referenced artifact",
            metadata.referenced_artifact.as_str(),
            4000,
        ),
        (
            "External reference ID",
            metadata.external_reference_id.as_str(),
            1000,
        ),
        (
            "Provider verification URL",
            metadata.provider_verification_url.as_str(),
            4000,
        ),
        (
            "Evidence file extension",
            metadata.file_extension.as_str(),
            32,
        ),
        ("Evidence MIME type", metadata.mime_type.as_str(), 200),
        ("Evidence audio format", metadata.audio_format.as_str(), 200),
        (
            "Suno created timestamp",
            metadata.suno_created_timestamp.as_str(),
            200,
        ),
        ("Suno created date", metadata.suno_created_date.as_str(), 10),
        ("Suno technical ID", metadata.suno_id.as_str(), 200),
        (
            "Raw embedded Suno metadata",
            metadata.suno_raw_metadata.as_str(),
            65_536,
        ),
    ] {
        validate_multiline_text(name, value, max)?;
    }
    if metadata.embedded_metadata.len() > 256 {
        return Err(AppError::Validation(
            "Evidence contains too many embedded metadata entries.".into(),
        ));
    }
    for entry in &metadata.embedded_metadata {
        validate_multiline_text("Embedded metadata key", &entry.key, 128)?;
        validate_multiline_text("Embedded metadata value", &entry.value, 65_536)?;
    }
    for (name, value) in [
        ("Evidence retrieval date", metadata.retrieval_date.as_str()),
        ("Evidence effective date", metadata.effective_date.as_str()),
    ] {
        validate_optional_date(name, value)?;
    }
    if !metadata.source_url.trim().is_empty() {
        let url = Url::parse(metadata.source_url.trim())
            .map_err(|_| AppError::Validation("Evidence source URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Evidence source URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    if !metadata.provider_verification_url.trim().is_empty() {
        let url = Url::parse(metadata.provider_verification_url.trim())
            .map_err(|_| AppError::Validation("Provider verification URL is invalid.".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(AppError::Validation(
                "Provider verification URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    match role {
        EvidenceRole::SunoTermsRights => {
            if metadata.document_title.trim().is_empty()
                || metadata.provider.trim().is_empty()
                || metadata.retrieval_date.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "Terms evidence exists, but descriptive metadata is incomplete: document title, provider/source, and retrieval date are required."
                        .into(),
                ));
            }
        }
        EvidenceRole::ExternalTimestamp => {
            if metadata.provider.trim().is_empty()
                || metadata.external_timestamp.trim().is_empty()
                || metadata.referenced_hash.trim().is_empty()
                || metadata.referenced_artifact.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "External timestamp evidence requires provider/issuer, timestamp, referenced hash, and referenced artifact.".into(),
                ));
            }
            if metadata.referenced_hash.len() != 64
                || !metadata
                    .referenced_hash
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(AppError::Validation(
                    "External timestamp evidence referenced hash must be a SHA-256 value.".into(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn apply_descriptive_evidence_metadata(target: &mut EvidenceMetadata, supplied: &EvidenceMetadata) {
    target.document_title = supplied.document_title.clone();
    target.provider = supplied.provider.clone();
    target.source_url = supplied.source_url.clone();
    target.retrieval_date = supplied.retrieval_date.clone();
    target.effective_date = supplied.effective_date.clone();
    target.applicable_production_period = supplied.applicable_production_period.clone();
    target.factual_note = supplied.factual_note.clone();
}

fn normalize_track_library(input: TrackLibraryPlacement) -> Result<TrackLibraryPlacement> {
    match input.section {
        TrackLibrarySection::Single => Ok(TrackLibraryPlacement::default()),
        TrackLibrarySection::Album => {
            let raw_title = input.album_title.as_deref().unwrap_or_default();
            if raw_title.chars().any(char::is_control) {
                return Err(AppError::Validation(
                    "Album title is invalid or too long.".into(),
                ));
            }
            let title = raw_title.trim();
            validate_short_text("Album title", title, 200, true)?;
            safe_album_directory(title)?;
            Ok(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some(title.to_owned()),
            })
        }
    }
}

fn safe_album_directory(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty()
        || title.starts_with('.')
        || title.eq_ignore_ascii_case(SINGLES_DIRECTORY)
        || title.contains(['/', '\\'])
        || title.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "Album title does not form a safe folder name.".into(),
        ));
    }
    Ok(title.to_owned())
}

fn physical_library_parent(library: &TrackLibraryPlacement) -> Result<PathBuf> {
    match library.section {
        TrackLibrarySection::Single => Ok(PathBuf::from(SINGLES_DIRECTORY)),
        TrackLibrarySection::Album => Ok(PathBuf::from(safe_album_directory(
            library.album_title.as_deref().unwrap_or_default(),
        )?)),
    }
}

fn physical_track_relative(library: &TrackLibraryPlacement, title: &str) -> Result<String> {
    Ok(portable_relative(
        &physical_library_parent(library)?.join(safe_track_directory(title)?),
    ))
}

fn safe_track_directory(title: &str) -> Result<String> {
    let title = title.trim();
    slugify(title)?;
    if title.starts_with('.') || title.contains(['/', '\\']) || title.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "Track title does not form a safe folder name.".into(),
        ));
    }
    Ok(title.to_owned())
}

fn physical_library_from_relative(relative: &str) -> Result<Option<TrackLibraryPlacement>> {
    let relative = Path::new(relative);
    crate::security::validate_relative(relative)?;
    let components = relative.components().collect::<Vec<_>>();
    if components.len() != 2 {
        return Ok(None);
    }
    let parent = components[0]
        .as_os_str()
        .to_str()
        .ok_or_else(|| AppError::Validation("Track library folder name must use UTF-8.".into()))?;
    if parent == SINGLES_DIRECTORY {
        return Ok(Some(TrackLibraryPlacement::default()));
    }
    Ok(Some(normalize_track_library(TrackLibraryPlacement {
        section: TrackLibrarySection::Album,
        album_title: Some(parent.to_owned()),
    })?))
}

fn is_hidden_workspace_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
}

fn discover_track_identities(root: &Path) -> Result<HashMap<String, String>> {
    let mut identities = HashMap::new();
    for entry in WalkDir::new(root)
        .min_depth(3)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry.depth() > 2
                || !entry.file_name().to_string_lossy().starts_with('.')
        })
    {
        let entry = entry.map_err(|error| {
            AppError::io(
                error.path().unwrap_or(root),
                std::io::Error::other(error.to_string()),
            )
        })?;
        if entry.file_type().is_symlink() {
            continue;
        }
        if !entry.file_type().is_file()
            || entry.file_name() != std::ffi::OsStr::new("track.json")
            || entry.path().parent().and_then(Path::file_name)
                != Some(std::ffi::OsStr::new(".summary"))
        {
            continue;
        }
        let Some(track_root) = entry.path().parent().and_then(Path::parent) else {
            continue;
        };
        if track_root.starts_with(root.join(".suno-doc")) {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(id) = value.get("trackId").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let relative = portable_relative(
            track_root
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?,
        );
        match identities.entry(id.to_owned()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(relative);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(String::new());
            }
        }
    }
    identities.retain(|_, relative| !relative.is_empty());
    Ok(identities)
}

fn find_unclaimed_track_in_library(
    root: &Path,
    library: &TrackLibraryPlacement,
    claimed: &HashSet<String>,
) -> Result<Option<String>> {
    let parent_relative = physical_library_parent(library)?;
    let parent = contained_path(root, &parent_relative, false)?;
    if !parent.is_dir() {
        return Ok(None);
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&parent).map_err(|error| AppError::io(&parent, error))? {
        let entry = entry.map_err(|error| AppError::io(&parent, error))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let relative = portable_relative(
            &entry
                .path()
                .strip_prefix(root)
                .map_err(|_| AppError::PathEscape)?,
        );
        if !claimed.contains(&relative) {
            candidates.push(relative);
        }
    }
    Ok((candidates.len() == 1).then(|| candidates.remove(0)))
}

fn discover_workspace_tracks(
    root: &Path,
    warnings: &mut Vec<String>,
) -> Result<Vec<(String, PathBuf, TrackLibraryPlacement)>> {
    let mut result = Vec::new();
    let entries = fs::read_dir(root).map_err(|error| AppError::io(root, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| AppError::io(root, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() {
            warnings.push(format!("Skipped symbolic-link candidate: {name}"));
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }

        if name == SINGLES_DIRECTORY {
            collect_library_children(
                root,
                &entry.path(),
                TrackLibraryPlacement::default(),
                warnings,
                &mut result,
            )?;
        } else if looks_like_track_root(&entry.path()) {
            result.push((
                name.clone(),
                PathBuf::from(name),
                TrackLibraryPlacement::default(),
            ));
        } else {
            let library = match normalize_track_library(TrackLibraryPlacement {
                section: TrackLibrarySection::Album,
                album_title: Some(name.clone()),
            }) {
                Ok(library) => library,
                Err(error) => {
                    warnings.push(format!("Skipped invalid album folder {name}: {error}"));
                    continue;
                }
            };
            collect_library_children(root, &entry.path(), library, warnings, &mut result)?;
        }
    }
    result.sort_by(|left, right| {
        portable_relative(&left.1)
            .to_lowercase()
            .cmp(&portable_relative(&right.1).to_lowercase())
    });
    Ok(result)
}

fn collect_library_children(
    root: &Path,
    parent: &Path,
    library: TrackLibraryPlacement,
    warnings: &mut Vec<String>,
    result: &mut Vec<(String, PathBuf, TrackLibraryPlacement)>,
) -> Result<()> {
    for entry in fs::read_dir(parent).map_err(|error| AppError::io(parent, error))? {
        let entry = entry.map_err(|error| AppError::io(parent, error))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| AppError::io(entry.path(), error))?;
        if metadata.file_type().is_symlink() {
            warnings.push(format!(
                "Skipped symbolic-link track candidate: {}",
                entry.path().display()
            ));
            continue;
        }
        if !metadata.is_dir() || name.starts_with('.') {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| AppError::PathEscape)?
            .to_owned();
        crate::security::validate_relative(&relative)?;
        result.push((name, relative, library.clone()));
    }
    Ok(())
}

fn looks_like_track_root(path: &Path) -> bool {
    path.join(TRACK_IDENTITY_FILE).is_file()
        || [
            "01_RELEASE",
            "02_SUNO",
            "03_DOCUMENTATION",
            "04_LICENSES",
            "05_ARTWORK",
            "06_CERTIFICATE",
        ]
        .iter()
        .any(|folder| path.join(folder).is_dir())
}

fn validate_track_title(title: &str) -> Result<()> {
    validate_short_text("Track title", title, 200, true)?;
    if title.contains(['/', '\\']) || title.split_whitespace().any(|part| part == "..") {
        return Err(AppError::Validation(
            "Track title must not contain path separators or traversal components.".into(),
        ));
    }
    slugify(title)?;
    Ok(())
}

fn validate_required_production_range(track: &TrackRecord) -> Result<()> {
    if track.fields.production_start_date.is_empty() || track.fields.production_end_date.is_empty()
    {
        return Err(AppError::Validation(
            "Production start and end dates are required for this operation.".into(),
        ));
    }
    validate_date_range(
        "Production period",
        &track.fields.production_start_date,
        &track.fields.production_end_date,
    )
}

#[cfg(test)]
fn apply_patch(fields: &mut crate::model::TrackFields, patch: TrackPatch) {
    apply_patch_with_explicit_nulls(fields, patch, &[]);
}

fn apply_patch_with_explicit_nulls(
    fields: &mut crate::model::TrackFields,
    patch: TrackPatch,
    explicit_null_fields: &[String],
) {
    if let Some(value) = patch.title {
        fields.title = value;
    }
    if let Some(value) = patch.production_start_date {
        fields.production_start_date = value;
    }
    if let Some(value) = patch.production_end_date {
        fields.production_end_date = value;
    }
    if let Some(value) = patch.suno_model {
        fields.suno_model = value;
    }
    if let Some(value) = patch.suno_project_url {
        fields.suno_project_url = value;
    }
    if let Some(value) = patch.suno_project_version_id {
        fields.suno_project_version_id = value;
    }
    if let Some(value) = patch.suno_final_generation_id {
        fields.suno_final_generation_id = value;
    }
    if let Some(value) = patch.suno_final_generation_date {
        fields.suno_final_generation_date = value;
    }
    if let Some(value) = patch.suno_final_generation_time {
        fields.suno_final_generation_time = value;
    }
    if let Some(value) = patch.suno_download_export_date {
        fields.suno_download_export_date = value;
    }
    if let Some(value) = patch.suno_plan_at_generation {
        fields.suno_plan_at_generation = value;
    }
    if let Some(value) = patch.legacy_suno_plan_at_creation {
        fields.legacy_suno_plan_at_creation = value;
    }
    if let Some(value) = patch.final_export_date {
        fields.final_export_date = value;
    }
    if let Some(value) = patch.instrumental_track {
        fields.instrumental_track = Some(value);
    }
    if let Some(value) = patch.vocal_lyrics_present {
        fields.vocal_lyrics_present = Some(value);
    }
    if let Some(value) = patch.suno_lyrics_field_content {
        fields.suno_lyrics_field_content = Some(value);
    }
    if let Some(value) = patch.suno_lyrics_content_types {
        fields.suno_lyrics_content_types = value;
    }
    if let Some(value) = patch.suno_lyrics_content_source {
        fields.suno_lyrics_content_source = Some(value);
    }
    if let Some(value) = patch.suno_lyrics_field_text {
        fields.suno_lyrics_field_text = value;
    }
    if let Some(value) = patch.suno_lyrics_other_content_type {
        fields.suno_lyrics_other_content_type = value;
    }
    if let Some(value) = patch.lyrics_source {
        fields.lyrics_source = value;
    }
    if let Some(value) = patch.lyrics_text {
        fields.lyrics_text = value;
    }
    if let Some(value) = patch.suno_style_prompt {
        fields.suno_style_prompt = value;
    }
    if let Some(value) = patch.external_audio_uploaded {
        fields.external_audio_uploaded = Some(value);
    }
    if let Some(value) = patch.external_audio_source {
        fields.external_audio_source = value;
    }
    if let Some(value) = patch.external_audio_ownership {
        fields.external_audio_ownership = value;
    }
    if let Some(value) = patch.own_audio_uploaded {
        fields.own_audio_uploaded = Some(value);
    }
    if let Some(value) = patch.own_audio_source {
        fields.own_audio_source = value;
    }
    if let Some(value) = patch.own_audio_ownership {
        fields.own_audio_ownership = value;
    }
    if let Some(value) = patch.code_based_generation {
        fields.code_based_generation = Some(value);
    }
    if let Some(value) = patch.code_audio_post_processed {
        fields.code_audio_post_processed = Some(value);
    }
    if let Some(value) = patch.code_audio_post_processing_operations {
        fields.code_audio_post_processing_operations = value;
    }
    if let Some(value) = patch.code_audio_post_processing_note {
        fields.code_audio_post_processing_note = value;
    }
    if let Some(value) = patch.third_party_samples_uploaded {
        fields.third_party_samples_uploaded = Some(value);
    }
    if let Some(value) = patch.third_party_sample_source {
        fields.third_party_sample_source = value;
    }
    if let Some(value) = patch.third_party_sample_ownership {
        fields.third_party_sample_ownership = value;
    }
    if let Some(value) = patch.human_editing_performed {
        fields.human_editing_performed = Some(value);
    }
    if let Some(value) = patch.human_editing_details {
        fields.human_editing_details = value;
    }
    if let Some(value) = patch.post_export_editing_performed {
        fields.post_export_editing_performed = Some(value);
    }
    if let Some(value) = patch.post_export_editing_details {
        fields.post_export_editing_details = value;
    }
    if let Some(value) = patch.commercial_use_intended {
        fields.commercial_use_intended = value;
    }
    if let Some(value) = patch.release_filename_difference_confirmed {
        fields.release_filename_difference_confirmed = Some(value);
    }
    if let Some(value) = patch.suno_export_filename_difference_confirmed {
        fields.suno_export_filename_difference_confirmed = Some(value);
    }
    if let Some(value) = patch.suno_terms_evidence_not_available {
        fields.suno_terms_evidence_not_available = Some(value);
    }
    if let Some(value) = patch.generative_ai_used {
        fields.generative_ai_used = Some(value);
    }
    if let Some(value) = patch.audio_ai_system {
        fields.audio_ai_system = value;
    }
    if let Some(value) = patch.ai_assisted_audio_elements {
        fields.ai_assisted_audio_elements = Some(value);
    }
    if let Some(value) = patch.ai_generated_audio_elements {
        fields.ai_generated_audio_elements = Some(value);
    }
    if let Some(value) = patch.real_person_voice_intentionally_imitated {
        fields.real_person_voice_intentionally_imitated = Some(value);
    }
    if let Some(value) = patch.real_person_identity_intentionally_represented {
        fields.real_person_identity_intentionally_represented = Some(value);
    }
    if let Some(value) = patch.real_event_represented_as_authentic_recording {
        fields.real_event_represented_as_authentic_recording = Some(value);
    }
    if let Some(value) = patch.real_location_institution_event_presented_as_authentic_ai_recording {
        fields.real_location_institution_event_presented_as_authentic_ai_recording = Some(value);
    }
    if let Some(value) = patch.audio_disclosure_applied {
        fields.audio_disclosure_applied = Some(value);
    }
    if let Some(value) = patch.audio_disclosure_locations {
        fields.audio_disclosure_locations = value;
    }
    if let Some(value) = patch.audio_disclosure_text {
        fields.audio_disclosure_text = value;
    }
    if let Some(value) = patch.audio_disclosure_reason {
        fields.audio_disclosure_reason = value;
    }
    if let Some(value) = patch.artwork_origin {
        fields.artwork_origin = value;
    }
    if let Some(value) = patch.ai_image_service {
        fields.ai_image_service = value;
    }
    if let Some(value) = patch.human_artwork_process_operations {
        fields.human_artwork_process_operations = value;
    }
    if let Some(value) = patch.human_artwork_process_notes {
        fields.human_artwork_process_notes = value;
    }
    if let Some(value) = patch.human_artwork_modifications {
        fields.human_artwork_modifications = value;
    }
    if let Some(value) = patch.custom_artwork_change {
        fields.custom_artwork_change = value;
    }
    if let Some(value) = patch.depicts_real_person {
        fields.depicts_real_person = Some(value);
    }
    if let Some(value) = patch.real_person_notes {
        fields.real_person_notes = value;
    }
    if let Some(value) = patch.depicts_real_event {
        fields.depicts_real_event = Some(value);
    }
    if let Some(value) = patch.real_event_notes {
        fields.real_event_notes = value;
    }
    if let Some(value) = patch.contains_trademark {
        fields.contains_trademark = Some(value);
    }
    if let Some(value) = patch.trademark_notes {
        fields.trademark_notes = value;
    }
    if let Some(value) = patch.disclosure_applied {
        fields.disclosure_applied = Some(value);
    }
    if let Some(value) = patch.disclosure_text {
        fields.disclosure_text = value;
    }
    if let Some(value) = patch.release_notes {
        fields.release_notes = value;
    }
    for field in explicit_null_fields {
        match field.as_str() {
            "instrumentalTrack" => fields.instrumental_track = None,
            "vocalLyricsPresent" => fields.vocal_lyrics_present = None,
            "sunoLyricsFieldContent" => fields.suno_lyrics_field_content = None,
            "sunoLyricsContentSource" => fields.suno_lyrics_content_source = None,
            "externalAudioUploaded" => fields.external_audio_uploaded = None,
            "ownAudioUploaded" => fields.own_audio_uploaded = None,
            "codeBasedGeneration" => fields.code_based_generation = None,
            "codeAudioPostProcessed" => fields.code_audio_post_processed = None,
            "thirdPartySamplesUploaded" => fields.third_party_samples_uploaded = None,
            "humanEditingPerformed" => fields.human_editing_performed = None,
            "postExportEditingPerformed" => fields.post_export_editing_performed = None,
            "releaseFilenameDifferenceConfirmed" => {
                fields.release_filename_difference_confirmed = None;
            }
            "sunoExportFilenameDifferenceConfirmed" => {
                fields.suno_export_filename_difference_confirmed = None;
            }
            "sunoTermsEvidenceNotAvailable" => fields.suno_terms_evidence_not_available = None,
            "depictsRealPerson" => fields.depicts_real_person = None,
            "depictsRealEvent" => fields.depicts_real_event = None,
            "containsTrademark" => fields.contains_trademark = None,
            "disclosureApplied" => fields.disclosure_applied = None,
            "generativeAiUsed" => fields.generative_ai_used = None,
            "aiAssistedAudioElements" => fields.ai_assisted_audio_elements = None,
            "aiGeneratedAudioElements" => fields.ai_generated_audio_elements = None,
            "realPersonVoiceIntentionallyImitated" => {
                fields.real_person_voice_intentionally_imitated = None;
            }
            "realPersonIdentityIntentionallyRepresented" => {
                fields.real_person_identity_intentionally_represented = None;
            }
            "realEventRepresentedAsAuthenticRecording" => {
                fields.real_event_represented_as_authentic_recording = None;
            }
            "realLocationInstitutionEventPresentedAsAuthenticAiRecording" => {
                fields.real_location_institution_event_presented_as_authentic_ai_recording = None;
            }
            "audioDisclosureApplied" => fields.audio_disclosure_applied = None,
            _ => {}
        }
    }
    fields.normalize_conditionals();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CertificateLanguage, CustomRfc3161Settings, DocumentationAnswer, EmbeddedMetadata,
        FactOrigin, FinalizeOptions, SunoLyricsContentSource, SunoLyricsContentType,
        TimestampProviderKind, TimestampReferencedArtifact, TimestampSettings, TimestampType,
    };
    use crate::workflow::CoverageStatus;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[derive(Debug)]
    struct ParsedCertificate {
        fields: BTreeMap<String, String>,
        completed_steps: BTreeMap<String, String>,
        na_reasons: BTreeMap<String, String>,
    }

    fn complete_profile() -> Profile {
        Profile {
            artist_name: "Acceptance Artist".into(),
            suno_profile_name: "acceptance-profile".into(),
            suno_handle: "@acceptance".into(),
            suno_plan: "Pro".into(),
            subscription_start_date: "2026-01-01".into(),
            default_commercial_use: false,
            default_ai_image_service: "Local Tool".into(),
            artwork_transparency_policy: "always".into(),
            disclosure_text: "AI-assisted".into(),
            certificate_language: CertificateLanguage::En,
        }
    }

    fn source_file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        walkdir::WalkDir::new(root)
            .min_depth(1)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .expect("relative source file")
                    .to_string_lossy()
                    .replace('\\', "/");
                (
                    relative,
                    fs::read(entry.path()).expect("source fixture bytes"),
                )
            })
            .collect()
    }

    #[test]
    fn folder_import_single_uses_selected_library_and_keeps_incomplete_facts_open() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let source = directory.path().join("Awakening");
        fs::create_dir(&source).expect("source directory");
        fs::write(source.join("Awakening.mp3"), b"ID3audio").expect("MP3 fixture");
        fs::write(source.join("Awakening.mp4"), b"\0\0\0\x0cftypisom").expect("MP4 fixture");
        fs::write(source.join("Awakening.wav"), b"RIFF\x04\0\0\0WAVE").expect("WAV fixture");
        fs::write(source.join("Awakening.jpeg"), b"\xff\xd8\xffimage").expect("JPEG fixture");
        fs::write(
            source.join("Awakening_AI_ORIGINAL.png"),
            b"\x89PNG\r\n\x1a\noriginal",
        )
        .expect("AI original fixture");
        fs::write(
            source.join("Awakening_AI_EDITED.png"),
            b"\x89PNG\r\n\x1a\nedited",
        )
        .expect("AI edited fixture");
        fs::write(
            source.join("Bildschirmfoto_20260817_141059.png"),
            b"\x89PNG\r\n\x1a\nscreenshot",
        )
        .expect("screenshot fixture");
        fs::write(source.join("SpaceWideToWide1.rb"), b"play 60\n").expect("source-code fixture");
        fs::write(source.join("Lyrics.txt"), b"First line\nSecond line\n").expect("lyrics fixture");
        fs::write(source.join("Style.txt"), b"dreamy synthwave\n").expect("style fixture");
        let before = source_file_snapshot(&source);

        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let imported = app
            .import_folder(FolderImportExecutionInput {
                source_path: source.display().to_string(),
                expected_kind: folder_import::FolderImportKind::Single,
                single_track_title: Some("Awakening".into()),
                single_track_library: Some(TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("Chosen Album".into()),
                }),
                production_start_date: String::new(),
                commercial_use_intended: Some(false),
            })
            .expect("folder import");

        assert_eq!(imported.len(), 1);
        let track = &imported[0];
        assert_eq!(track.relative_path, "Chosen Album/Awakening");
        assert_eq!(track.library.section, TrackLibrarySection::Album);
        assert_eq!(track.fields.production_start_date, "");
        assert_ne!(track.status, TrackStatus::Finalized);
        assert_eq!(track.fields.code_based_generation, Some(true));
        assert_eq!(track.fields.external_audio_uploaded, None);
        assert_eq!(track.fields.own_audio_uploaded, None);
        assert_eq!(track.fields.third_party_samples_uploaded, None);
        assert_eq!(track.fields.human_editing_performed, None);
        assert_eq!(track.fields.lyrics_text, "First line\nSecond line\n");
        assert!(track.fields.lyrics_source.is_empty());
        assert_eq!(track.fields.suno_style_prompt, "dreamy synthwave\n");
        for role in [
            EvidenceRole::ReleaseMp3,
            EvidenceRole::ReleaseMp4,
            EvidenceRole::ReleaseWav,
            EvidenceRole::ArtworkSunoOriginal,
            EvidenceRole::AiArtworkOriginal,
            EvidenceRole::AiArtworkEdited,
            EvidenceRole::SunoScreenshot,
            EvidenceRole::SourceCodeFile,
            EvidenceRole::Lyrics,
            EvidenceRole::Style,
        ] {
            assert!(
                track.evidence.iter().any(|item| item.role == role),
                "missing imported role {role:?}"
            );
        }
        let track_root = workspace.join(&track.relative_path);
        for folder in TRACK_FOLDERS {
            assert!(track_root.join(folder).is_dir(), "missing {folder}");
        }
        assert!(track_root.join(".summary/track.json").is_file());
        assert_eq!(source_file_snapshot(&source), before);
        assert_eq!(track.steps.len(), 10);
        assert!(track.missing_count > 0);
    }

    #[test]
    fn folder_import_album_creates_normal_tracks_and_leaves_root_files_unassigned() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let source = directory.path().join("The Final Protocol");
        fs::create_dir(&source).expect("album source");
        for title in ["Awakening", "Boot Sequence", "LastWarnung"] {
            let track = source.join(title);
            fs::create_dir(&track).expect("track source");
            fs::write(track.join(format!("{title}.mp3")), b"ID3audio").expect("track media");
        }
        fs::write(source.join("signed_contract.pdf"), b"%PDF-contract").expect("root fixture");
        let before = source_file_snapshot(&source);

        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let imported = app
            .import_folder(FolderImportExecutionInput {
                source_path: source.display().to_string(),
                expected_kind: folder_import::FolderImportKind::Album,
                single_track_title: Some("ignored".into()),
                single_track_library: Some(TrackLibraryPlacement::default()),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: Some(false),
            })
            .expect("album folder import");

        assert_eq!(imported.len(), 3);
        for track in &imported {
            assert!(track.relative_path.starts_with("The Final Protocol/"));
            assert_eq!(track.library.section, TrackLibrarySection::Album);
            assert_eq!(
                track.library.album_title.as_deref(),
                Some("The Final Protocol")
            );
            assert!(track.fields.production_start_date.is_empty());
            assert_ne!(track.status, TrackStatus::Finalized);
            assert!(track
                .evidence
                .iter()
                .any(|item| item.role == EvidenceRole::ReleaseMp3));
            assert!(!track
                .evidence
                .iter()
                .any(|item| item.metadata.original_file_name == "signed_contract.pdf"));
            for folder in TRACK_FOLDERS {
                assert!(workspace.join(&track.relative_path).join(folder).is_dir());
            }
            assert!(workspace
                .join(&track.relative_path)
                .join(".summary/track.json")
                .is_file());
        }
        assert_eq!(source_file_snapshot(&source), before);
    }

    #[test]
    fn folder_import_suno_wav_derives_timestamp_id_and_byte_identity_without_questions() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let source = directory.path().join("Metadata Track");
        fs::create_dir(&source).expect("source directory");
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");
        fs::write(source.join("Metadata Track.wav"), p0_pcm_wav(Some(&raw)))
            .expect("Suno WAV fixture");

        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let imported = app
            .import_folder(FolderImportExecutionInput {
                source_path: source.display().to_string(),
                expected_kind: folder_import::FolderImportKind::Single,
                single_track_title: Some("Metadata Track".into()),
                single_track_library: Some(TrackLibraryPlacement::default()),
                production_start_date: String::new(),
                commercial_use_intended: Some(false),
            })
            .expect("folder import");
        let track = &imported[0];

        assert_eq!(track.fields.suno_download_export_date, "2026-08-17");
        assert_eq!(track.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(track.fields.production_end_date, "2026-08-17");
        assert_eq!(
            track.automation.download_export_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(
            track.automation.suno_created_timestamp.as_deref(),
            Some("2026-08-17T06:38:06Z")
        );
        assert_eq!(track.automation.suno_id.as_deref(), Some(P0_SUNO_ID));
        assert!(track.automation.release_identical_to_suno_export);
        let suno = track
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SunoFinalExport)
            .expect("Suno evidence");
        assert_eq!(suno.metadata.suno_id, P0_SUNO_ID);
        assert_eq!(suno.metadata.suno_created_timestamp, "2026-08-17T06:38:06Z");
        assert!(suno.sha256.as_deref().is_some_and(|hash| hash.len() == 64));
    }

    #[test]
    fn folder_import_rejects_a_target_inside_the_source_before_creating_a_track() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        fs::write(workspace.join("Source.mp3"), b"ID3audio").expect("source media");
        let before = source_file_snapshot(&workspace);

        let result = app.import_folder(FolderImportExecutionInput {
            source_path: workspace.display().to_string(),
            expected_kind: folder_import::FolderImportKind::Single,
            single_track_title: Some("Unsafe Nested Target".into()),
            single_track_library: Some(TrackLibraryPlacement::default()),
            production_start_date: String::new(),
            commercial_use_intended: Some(false),
        });

        assert!(matches!(result, Err(AppError::Validation(_))));
        assert!(app.list_tracks().expect("track list").is_empty());
        assert_eq!(source_file_snapshot(&workspace), before);
    }

    fn prepare_ready_track(app: &WorkspaceApp, fixture_root: &Path, title: &str) -> TrackDetail {
        let created = app
            .create_track(CreateTrackInput {
                title: title.into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track creation");
        let updated = app
            .update_track(
                &created.id,
                TrackPatch {
                    suno_model: Some("v4.5".into()),
                    suno_project_url: Some("https://suno.com/song/acceptance-track".into()),
                    suno_project_version_id: Some("acceptance-project-version".into()),
                    suno_final_generation_id: Some("acceptance-generation".into()),
                    suno_final_generation_date: Some("2026-08-02".into()),
                    suno_download_export_date: Some("2026-08-03".into()),
                    suno_plan_at_generation: Some("Pro".into()),
                    production_end_date: Some("2026-08-03".into()),
                    final_export_date: Some("2026-08-03".into()),
                    instrumental_track: Some(true),
                    vocal_lyrics_present: Some(false),
                    suno_lyrics_field_content: Some(false),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(false),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(false),
                    commercial_use_intended: Some(false),
                    generative_ai_used: Some(true),
                    audio_ai_system: Some("Suno".into()),
                    ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
                    ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
                    real_person_voice_intentionally_imitated: Some(DocumentationAnswer::No),
                    real_person_identity_intentionally_represented: Some(DocumentationAnswer::No),
                    real_event_represented_as_authentic_recording: Some(DocumentationAnswer::No),
                    real_location_institution_event_presented_as_authentic_ai_recording: Some(
                        DocumentationAnswer::No,
                    ),
                    audio_disclosure_applied: Some(DocumentationAnswer::No),
                    audio_disclosure_reason: Some(
                        "User deliberately recorded that no disclosure was applied.".into(),
                    ),
                    artwork_origin: Some("none".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("complete track facts");
        fs::create_dir_all(fixture_root).expect("fixture directory");
        let suno_export = fixture_root.join("suno-export.wav");
        let release_master = fixture_root.join("release-master.wav");
        // The current workflow requires a genuine local Chromaprint result.
        // Use non-trivial PCM fixtures so the bundled engine can produce one;
        // the two deterministic signals intentionally differ.
        let suno_bytes = p0_screening_wav(None, 17);
        let release_bytes = p0_screening_wav(None, 23);
        fs::write(&suno_export, &suno_bytes).expect("Suno fixture");
        fs::write(&release_master, &release_bytes).expect("one-byte-different release fixture");
        app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
            .expect("Suno evidence import");
        app.import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
            .expect("release evidence import");
        app.update_track(
            &updated.id,
            TrackPatch {
                suno_export_filename_difference_confirmed: Some(true),
                release_filename_difference_confirmed: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("confirm intentional fixture filename deviations");
        app.generate_documents(&updated.id, false)
            .expect("document generation");
        let ready = app
            .calculate_hashes(&updated.id)
            .expect("SHA-256 generation")
            .track
            .expect("ready track detail");
        let validation = app.validate_track(&updated.id).expect("native gate");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
        ready
    }

    fn finalize_acceptance_track(
        app: &WorkspaceApp,
        fixture_root: &Path,
        title: &str,
    ) -> TrackDetail {
        let ready = prepare_ready_track(app, fixture_root, title);
        app.finalize_track(&ready.id)
            .expect("finalization")
            .track
            .expect("finalized track detail")
    }

    #[test]
    fn removing_release_audio_archives_live_screening_artifacts_and_marks_state_stale() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Remove Release Screening",
        );
        let root = app.root().join(&ready.relative_path);
        let live = root.join(audio_screening::AUDIO_SCREENING_DIR);
        assert!(live.join("LOCAL_FINGERPRINT.json").is_file());
        let release = ready
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");

        let removed = app
            .remove_evidence(&ready.id, &release.id)
            .expect("remove release evidence");

        assert_eq!(
            removed.audio_screening.local.status,
            AudioScreeningStatus::Stale
        );
        assert!(!live.exists(), "old screening must not remain live");
        let archive_entries = fs::read_dir(root.join(".archive/audio-screening"))
            .expect("screening archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("screening archive entries");
        assert!(archive_entries.iter().any(|entry| {
            entry
                .path()
                .join("AUDIO_SCREENING/LOCAL_FINGERPRINT.json")
                .is_file()
        }));
    }

    #[test]
    fn release_byte_mismatch_stales_and_archives_screening_before_new_docs_or_hashes() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Mutated Release Screening",
        );
        let root = app.root().join(&ready.relative_path);
        let release = ready
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");
        fs::write(
            root.join(&release.relative_path),
            p0_screening_wav(None, 91),
        )
        .expect("out-of-band source mutation");

        let verified = app
            .verify_evidence(&ready.id, Some(&release.id))
            .expect("record controlled mismatch");

        assert_eq!(
            verified.audio_screening.local.status,
            AudioScreeningStatus::Stale
        );
        assert!(!root.join(audio_screening::AUDIO_SCREENING_DIR).exists());
        assert!(matches!(
            app.generate_documents(&ready.id, false),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            app.calculate_hashes(&ready.id),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn altered_local_fingerprint_artifact_blocks_documents_hashes_and_finalization() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Altered Fingerprint Artifact",
        );
        let root = app.root().join(&ready.relative_path);
        let fingerprint = root.join(audio_screening::LOCAL_FINGERPRINT_FILE);
        let mut bytes = fs::read(&fingerprint).expect("local fingerprint artifact");
        bytes.extend_from_slice(b"\nmutated outside SunoDM\n");
        fs::write(&fingerprint, bytes).expect("mutate local fingerprint artifact");

        assert!(matches!(
            app.generate_documents(&ready.id, false),
            Err(AppError::Validation(message)) if message.contains("local audio-screening record")
        ));
        assert!(matches!(
            app.calculate_hashes(&ready.id),
            Err(AppError::Validation(message)) if message.contains("local audio-screening record")
        ));
        let finalization = app.finalize_track(&ready.id);
        assert!(
            matches!(finalization, Err(AppError::Validation(message)) if message.contains("audio screening"))
        );
        assert!(!root.join(certificate::CERTIFICATE_FILE).exists());
    }

    #[test]
    fn editable_workflow_upgrade_automatically_generates_current_local_screening() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Editable Screening Upgrade",
        );
        let root = app.root().join(&ready.relative_path);
        let mut legacy = app.persistence.track(&ready.id).expect("stored track");
        legacy.workflow_version = "1.7".into();
        legacy.audio_screening = Default::default();
        app.persistence
            .save_track(&legacy)
            .expect("simulate old workflow");
        fs::remove_dir_all(root.join(audio_screening::AUDIO_SCREENING_DIR))
            .expect("remove simulated pre-feature directory");

        let upgraded = app
            .re_evaluate_track(&ready.id)
            .expect("upgrade editable track")
            .track
            .expect("upgraded detail");

        assert_eq!(upgraded.workflow_version, "1.8");
        assert_eq!(
            upgraded.audio_screening.local.status,
            AudioScreeningStatus::FingerprintGenerated
        );
        assert!(root.join(audio_screening::LOCAL_FINGERPRINT_FILE).is_file());
    }

    #[test]
    fn release_replacement_resets_optional_external_state_without_blocking_finalization() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Replacement Optional Provider",
        );
        assert_eq!(
            ready.audio_screening.external.status,
            AudioScreeningStatus::SkippedNotConfigured
        );
        let release = ready
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");
        let replacement_source = directory.path().join("replacement-release.wav");
        fs::write(&replacement_source, p0_screening_wav(None, 101))
            .expect("replacement release source");

        let replaced = app
            .replace_evidence_from(
                &ready.id,
                &release.id,
                EvidenceRole::ReleaseWav,
                &replacement_source,
            )
            .expect("replace release and rerun local screening");

        assert_eq!(
            replaced.audio_screening.local.status,
            AudioScreeningStatus::FingerprintGenerated
        );
        assert_eq!(
            replaced.audio_screening.external.status,
            AudioScreeningStatus::SkippedNotConfigured
        );
        app.generate_documents(&ready.id, false)
            .expect("documents after replacement");
        app.calculate_hashes(&ready.id)
            .expect("hashes after replacement");
        let validation = app.validate_track(&ready.id).expect("finalization gate");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
        assert_eq!(
            app.finalize_track(&ready.id)
                .expect("finalization remains allowed")
                .track
                .expect("finalized detail")
                .status,
            TrackStatus::Finalized
        );
    }

    fn custom_timestamp_settings(
        endpoint: String,
        auto_after_finalization: bool,
    ) -> TimestampSettings {
        TimestampSettings {
            enabled: true,
            provider: TimestampProviderKind::CustomRfc3161,
            auto_after_finalization,
            custom: CustomRfc3161Settings {
                provider_name: "Deterministic test TSA".into(),
                endpoint,
                timeout_seconds: 2,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn one_shot_timestamp_server(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("timestamp test listener");
        let endpoint = format!(
            "http://{}/rfc3161",
            listener.local_addr().expect("listener address")
        );
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("timestamp request");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/timestamp-reply\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream
                .write_all(header.as_bytes())
                .expect("response header");
            stream.write_all(&body).expect("response body");
        });
        (endpoint, handle)
    }

    fn observing_timestamp_server() -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("timestamp test listener");
        listener
            .set_nonblocking(true)
            .expect("non-blocking listener");
        let endpoint = format!(
            "http://{}/rfc3161",
            listener.local_addr().expect("listener address")
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = calls.clone();
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(400);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((_stream, _)) => {
                        observed_calls.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("timestamp test listener failed: {error}"),
                }
            }
        });
        (endpoint, calls, handle)
    }

    #[test]
    fn legacy_finalized_snapshot_uses_stable_fallback_for_automatic_timestamp_attachment() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Legacy Timestamp Snapshot",
        );
        let certificate_id = finalized
            .certificate
            .certificate_id
            .clone()
            .expect("certificate ID");
        let mut legacy = app.persistence.track(&finalized.id).expect("stored track");
        legacy.certificate.finalization_snapshot_id = None;
        app.persistence.save_track(&legacy).expect("legacy state");
        let (endpoint, server) = one_shot_timestamp_server(b"malformed TSA response".to_vec());
        app.update_timestamp_settings(custom_timestamp_settings(endpoint, false))
            .expect("custom timestamp settings");

        let attached = app
            .attach_configured_external_timestamp(&finalized.id)
            .expect("provider response is archived even when verification fails");
        server.join().expect("timestamp server");

        assert_eq!(attached.status, TrackStatus::Finalized);
        assert_eq!(
            attached.external_timestamp_summary.status,
            ExternalTimestampStatus::VerificationFailed
        );
        let record = attached
            .external_timestamps
            .first()
            .expect("automatic timestamp record");
        assert_eq!(
            record
                .provider_metadata
                .as_ref()
                .expect("provider metadata")
                .referenced_revision_id,
            format!("legacy-finalization-snapshot:{certificate_id}")
        );
    }

    #[test]
    fn automatic_provider_failure_keeps_phase_one_finalized() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserved timestamp port");
        let endpoint = format!(
            "http://{}/rfc3161",
            listener.local_addr().expect("listener address")
        );
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("automatic timestamp request");
            drop(stream); // deterministic connection failure after phase one
        });
        app.update_timestamp_settings(custom_timestamp_settings(endpoint, true))
            .expect("auto timestamp settings");

        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Auto Timestamp Failure",
        );
        server.join().expect("timestamp server");

        assert_eq!(finalized.status, TrackStatus::Finalized);
        assert!(finalized.certificate.valid);
        assert_eq!(
            finalized.external_timestamp_summary.status,
            ExternalTimestampStatus::ProviderUnavailable
        );
        assert!(finalized.external_timestamps.is_empty());
    }

    #[test]
    fn manifest_tamper_records_anchor_mismatch_without_contacting_provider() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Timestamp Anchor Tamper",
        );
        let (endpoint, calls, server) = observing_timestamp_server();
        app.update_timestamp_settings(custom_timestamp_settings(endpoint, false))
            .expect("timestamp settings");
        let root = app.root().join(&finalized.relative_path);
        fs::write(
            root.join(certificate::MANIFEST_FILE),
            b"{\"tampered\":true}\n",
        )
        .expect("tamper manifest");

        let detail = app
            .attach_configured_external_timestamp(&finalized.id)
            .expect("anchor mismatch returns track state");
        server.join().expect("observer server");

        assert_eq!(
            detail.external_timestamp_summary.status,
            ExternalTimestampStatus::AnchorMismatch
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(detail.external_timestamps.is_empty());
    }

    #[test]
    fn one_changed_audio_byte_is_reported_as_not_identical_in_every_certificate_format() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "One Byte Identity Check",
        );
        assert!(!finalized.automation.release_identical_to_suno_export);
        let suno_hash = p0_evidence(&finalized, EvidenceRole::SunoFinalExport)
            .sha256
            .as_deref()
            .expect("Suno SHA-256");
        let release_hash = p0_evidence(&finalized, EvidenceRole::ReleaseWav)
            .sha256
            .as_deref()
            .expect("release SHA-256");
        assert_ne!(suno_hash, release_hash);

        let track_root = app.root().join(&finalized.relative_path);
        let managed = fs::read_to_string(track_root.join("02_SUNO/suno_project.txt"))
            .expect("managed Suno document");
        assert!(
            managed.contains("Release identical to Suno final export [System verification]: NO")
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("manifest bytes"),
        )
        .expect("manifest JSON");
        assert_eq!(
            manifest["system_verification"]["release_identical_to_suno_export"].as_bool(),
            Some(false)
        );
        let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
            .expect("Markdown certificate");
        assert!(markdown.contains("Release identical to Suno final export: **NO**"));
        let pdf = fs::read(track_root.join(certificate::PDF_FILE)).expect("certificate PDF");
        let mut warnings = Vec::new();
        let pdf = printpdf::PdfDocument::parse(
            &pdf,
            &printpdf::PdfParseOptions::default(),
            &mut warnings,
        )
        .expect("parse certificate PDF");
        let compact_pdf_text = pdf
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<String>()
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            compact_pdf_text.contains("ReleaseidenticaltoSunofinalexport[Systemverification]NO")
        );
    }

    #[test]
    fn external_timestamps_are_hash_checked_addenda_bound_to_one_certificate_revision() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Timestamp Revision Binding",
        );
        let certificate_id = finalized
            .certificate
            .certificate_id
            .clone()
            .expect("certificate ID");
        let manifest_anchor = finalized
            .finalization_anchors
            .iter()
            .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
            .expect("manifest anchor")
            .clone();
        let track_root = app.root().join(&finalized.relative_path);
        let excluded_anchor = track_root.join(".summary/track.json");
        let excluded_source = directory.path().join("timestamp-excluded.json");
        fs::write(&excluded_source, b"{\"providerRecord\":\"excluded\"}\n")
            .expect("excluded timestamp fixture");
        let excluded_error = app
            .attach_external_timestamp_from(
                &finalized.id,
                &excluded_source,
                ExternalTimestampInput {
                    provider: "Example Timestamp Provider".into(),
                    timestamp_type: TimestampType::ExternalIntegrityTimestamp,
                    timestamp_value: "2026-08-17T11:55:00Z".into(),
                    referenced_artifact: TimestampReferencedArtifact::Other,
                    other_referenced_artifact: ".summary/track.json".into(),
                    referenced_sha256: sha256_file(&excluded_anchor)
                        .expect("excluded anchor digest"),
                    external_reference_id: String::new(),
                    provider_verification_url: String::new(),
                    note: String::new(),
                },
            )
            .expect_err("excluded post-finalization file cannot become an Other anchor");
        assert!(excluded_error.to_string().contains("phase-one SHA256SUMS"));
        assert!(app
            .persistence
            .external_timestamps(&finalized.id)
            .expect("no rejected timestamp record")
            .is_empty());
        let stable_files = [
            certificate::MANIFEST_FILE,
            certificate::CERTIFICATE_FILE,
            certificate::CERTIFICATE_HASH_FILE,
            certificate::PDF_FILE,
            integrity::HASH_FILE,
        ]
        .into_iter()
        .map(|relative| {
            (
                relative.to_owned(),
                fs::read(track_root.join(relative)).expect("stable phase-one artifact"),
            )
        })
        .collect::<BTreeMap<_, _>>();

        let matching_source = directory.path().join("timestamp-match.json");
        fs::write(
            &matching_source,
            b"{\"providerRecord\":\"timestamp-match\"}\n",
        )
        .expect("matching timestamp fixture");
        let attached = app
            .attach_external_timestamp_from(
                &finalized.id,
                &matching_source,
                ExternalTimestampInput {
                    provider: "Example Timestamp Provider".into(),
                    timestamp_type: TimestampType::QualifiedElectronicTimestampUserDeclared,
                    timestamp_value: "2026-08-17T12:00:00Z".into(),
                    referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                    other_referenced_artifact: String::new(),
                    referenced_sha256: manifest_anchor.sha256.clone(),
                    external_reference_id: "EXT-REF-001".into(),
                    provider_verification_url: "https://timestamp.example/verify/EXT-REF-001"
                        .into(),
                    note: "Provider qualification is user-declared, not system-verified.".into(),
                },
            )
            .expect("matching timestamp attachment");
        assert_eq!(attached.external_timestamps.len(), 1);
        let matching = &attached.external_timestamps[0];
        assert_eq!(matching.certificate_id, certificate_id);
        assert_eq!(matching.referenced_hash_match, Some(true));
        assert_eq!(matching.actual_sha256, manifest_anchor.sha256);
        for relative in [
            &matching.record_relative_path,
            &matching.markdown_relative_path,
            &matching.pdf_relative_path,
            &matching.hash_list_relative_path,
        ] {
            assert!(
                track_root.join(relative).is_file(),
                "missing addendum {relative}"
            );
        }
        let addendum = fs::read_to_string(track_root.join(&matching.markdown_relative_path))
            .expect("timestamp addendum markdown");
        assert!(addendum.contains("Referenced hash match [System verification]: **YES**"));
        assert!(addendum.contains("user declared"));
        assert!(addendum.contains("does not determine any legal qualification"));

        let mismatch_source = directory.path().join("timestamp-mismatch.tsr");
        fs::write(&mismatch_source, b"opaque non-empty RFC3161-like fixture")
            .expect("mismatch timestamp fixture");
        let mismatched = app
            .attach_external_timestamp_from(
                &finalized.id,
                &mismatch_source,
                ExternalTimestampInput {
                    provider: "Example Timestamp Provider".into(),
                    timestamp_type: TimestampType::ExternalIntegrityTimestamp,
                    timestamp_value: "2026-08-17T12:05:00Z".into(),
                    referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                    other_referenced_artifact: String::new(),
                    referenced_sha256: "0".repeat(64),
                    external_reference_id: String::new(),
                    provider_verification_url: String::new(),
                    note: String::new(),
                },
            )
            .expect("mismatching timestamp remains documented");
        assert_eq!(mismatched.external_timestamps.len(), 2);
        let mismatch = mismatched
            .external_timestamps
            .iter()
            .find(|record| record.referenced_hash_match == Some(false))
            .expect("negative hash comparison");
        let mismatch_addendum =
            fs::read_to_string(track_root.join(&mismatch.markdown_relative_path))
                .expect("mismatch addendum markdown");
        assert!(mismatch_addendum.contains("Referenced hash match [System verification]: **NO**"));

        for (relative, expected) in &stable_files {
            assert_eq!(
                &fs::read(track_root.join(relative)).expect("phase-one artifact after addendum"),
                expected,
                "timestamp attachment changed {relative}"
            );
        }
        certificate::verify(&track_root).expect("primary certificate remains valid");
        assert!(
            integrity::verify(&track_root)
                .expect("primary integrity remains readable")
                .verified
        );

        let matching_directory = track_root
            .join(&matching.record_relative_path)
            .parent()
            .expect("timestamp record directory")
            .to_path_buf();
        let matching_evidence = matching_directory.join("TIMESTAMP_EVIDENCE.json");
        fs::write(&matching_evidence, b"tampered timestamp evidence")
            .expect("tamper timestamp sidecar only");
        let after_sidecar_tamper = app
            .load_track(&finalized.id)
            .expect("phase-one snapshot remains loadable");
        assert!(after_sidecar_tamper.certificate.valid);
        let damaged = after_sidecar_tamper
            .external_timestamps
            .iter()
            .find(|record| record.id == matching.id)
            .expect("damaged timestamp record remains visible");
        assert!(!damaged.integrity_verified);
        assert!(damaged
            .integrity_issues
            .iter()
            .any(|issue| issue.contains("evidence SHA-256")));
        assert!(after_sidecar_tamper
            .external_timestamps
            .iter()
            .find(|record| record.id == mismatch.id)
            .is_some_and(|record| record.integrity_verified));
        assert!(
            integrity::verify(&track_root)
                .expect("phase-one integrity after sidecar tamper")
                .verified
        );

        let revision = app
            .create_revision(&finalized.id)
            .expect("new revision after timestamp")
            .track
            .expect("revision detail");
        assert_eq!(revision.external_timestamps.len(), 2);
        assert!(revision
            .external_timestamps
            .iter()
            .find(|record| record.id == matching.id)
            .is_some_and(|record| !record.integrity_verified));
        assert!(revision
            .external_timestamps
            .iter()
            .find(|record| record.id == mismatch.id)
            .is_some_and(|record| record.integrity_verified));
        assert!(revision.finalization_anchors.is_empty());
        let persisted = app
            .persistence
            .external_timestamps(&finalized.id)
            .expect("historical timestamp records");
        assert_eq!(persisted.len(), 2);
        assert!(persisted
            .iter()
            .all(|record| record.certificate_id == certificate_id));
        let archived_records = WalkDir::new(track_root.join(".archive/revisions"))
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_name() == "TIMESTAMP_RECORD.json")
            .count();
        assert_eq!(archived_records, 2);

        let archived_mismatch_evidence = WalkDir::new(track_root.join(".archive/revisions"))
            .into_iter()
            .filter_map(std::result::Result::ok)
            .find(|entry| {
                entry.file_name() == "TIMESTAMP_EVIDENCE.tsr"
                    && entry.path().parent().and_then(Path::file_name)
                        == Some(std::ffi::OsStr::new(&mismatch.id))
            })
            .expect("archived mismatch evidence")
            .into_path();
        let revision_root = archived_mismatch_evidence
            .ancestors()
            .find(|path| {
                path.parent().and_then(Path::file_name) == Some(std::ffi::OsStr::new("revisions"))
            })
            .expect("timestamp revision root");
        let revision_metadata = revision_root.join("revision.json");
        let original_revision_metadata =
            fs::read(&revision_metadata).expect("original revision metadata");
        let mut wrong_revision: serde_json::Value =
            serde_json::from_slice(&original_revision_metadata).expect("revision metadata JSON");
        wrong_revision["previous_certificate"]["certificateId"] =
            serde_json::Value::String("SDM-different-certificate".into());
        fs::write(
            &revision_metadata,
            serde_json::to_vec_pretty(&wrong_revision).expect("wrong revision metadata bytes"),
        )
        .expect("tamper revision certificate binding");
        let after_revision_binding_tamper = app
            .load_track(&finalized.id)
            .expect("track remains loadable after revision binding tamper");
        let wrongly_bound = after_revision_binding_tamper
            .external_timestamps
            .iter()
            .find(|record| record.id == mismatch.id)
            .expect("wrongly bound archived timestamp remains visible");
        assert!(!wrongly_bound.integrity_verified);
        assert!(wrongly_bound
            .integrity_issues
            .iter()
            .any(|issue| issue.contains("certificate ID")));
        fs::write(&revision_metadata, original_revision_metadata)
            .expect("restore revision certificate binding");
        let restored_revision_binding = app
            .load_track(&finalized.id)
            .expect("track reload after revision binding restore");
        assert!(restored_revision_binding
            .external_timestamps
            .iter()
            .find(|record| record.id == mismatch.id)
            .is_some_and(|record| record.integrity_verified));

        fs::write(
            &archived_mismatch_evidence,
            b"tampered archived timestamp evidence",
        )
        .expect("tamper archived timestamp evidence");
        let after_archive_tamper = app
            .load_track(&finalized.id)
            .expect("track remains loadable after archived sidecar tamper");
        let damaged_archive = after_archive_tamper
            .external_timestamps
            .iter()
            .find(|record| record.id == mismatch.id)
            .expect("archived timestamp remains visible");
        assert!(!damaged_archive.integrity_verified);
        assert!(damaged_archive
            .integrity_issues
            .iter()
            .any(|issue| issue.contains("evidence SHA-256")));
    }

    #[test]
    fn timestamp_publication_recovery_reconciles_pending_stages_without_adopting_orphans() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Timestamp Publication Recovery",
        );
        let certificate_id = finalized
            .certificate
            .certificate_id
            .as_deref()
            .expect("certificate ID")
            .to_owned();
        let anchor = finalized
            .finalization_anchors
            .iter()
            .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
            .expect("manifest anchor")
            .clone();
        let track_root = app.root().join(&finalized.relative_path);
        let input = |note: &str| ExternalTimestampInput {
            provider: "Recovery Timestamp Provider".into(),
            timestamp_type: TimestampType::ExternalIntegrityTimestamp,
            timestamp_value: "2026-08-17T15:00:00Z".into(),
            referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
            other_referenced_artifact: String::new(),
            referenced_sha256: anchor.sha256.clone(),
            external_reference_id: String::new(),
            provider_verification_url: String::new(),
            note: note.into(),
        };

        let abandoned_source = directory.path().join("abandoned-timestamp.json");
        fs::write(&abandoned_source, b"{\"state\":\"staged-only\"}\n")
            .expect("abandoned timestamp source");
        let abandoned = external_timestamp::stage(
            &track_root,
            &certificate_id,
            &abandoned_source,
            input("crash before database registration"),
        )
        .expect("unregistered stage");
        let abandoned_stage = track_root
            .join(".archive/timestamp-staging")
            .join(&abandoned.record.id);
        assert!(abandoned_stage.is_dir());
        assert!(!track_root
            .join(&abandoned.record.record_relative_path)
            .exists());
        drop(app);

        let app = WorkspaceApp::open(&workspace, false).expect("clean abandoned stage recovery");
        assert!(!abandoned_stage.exists());
        assert!(app
            .persistence
            .external_timestamps(&finalized.id)
            .expect("timestamp rows after abandoned stage cleanup")
            .is_empty());

        let pending_source = directory.path().join("pending-timestamp.json");
        fs::write(&pending_source, b"{\"state\":\"database-registered\"}\n")
            .expect("pending timestamp source");
        let pending = external_timestamp::stage(
            &track_root,
            &certificate_id,
            &pending_source,
            input("crash after database registration"),
        )
        .expect("registered stage");
        app.persistence
            .save_external_timestamp(&finalized.id, &pending.record)
            .expect("register pending timestamp before simulated crash");
        let pending_stage = track_root
            .join(".archive/timestamp-staging")
            .join(&pending.record.id);
        let pending_live = track_root
            .join(&pending.record.record_relative_path)
            .parent()
            .expect("pending live directory")
            .to_path_buf();
        assert!(pending_stage.is_dir());
        assert!(!pending_live.exists());
        drop(app);

        let app = WorkspaceApp::open(&workspace, false).expect("publish registered pending stage");
        assert!(!pending_stage.exists());
        assert!(pending_live.is_dir());
        let recovered = app
            .load_track(&finalized.id)
            .expect("load recovered timestamp record");
        assert!(recovered
            .external_timestamps
            .iter()
            .find(|record| record.id == pending.record.id)
            .is_some_and(|record| record.integrity_verified));

        // Simulate the old unsafe crash window: a live sidecar without a DB
        // registration is detected explicitly and is never auto-adopted.
        let orphan_source = directory.path().join("orphan-timestamp.json");
        fs::write(&orphan_source, b"{\"state\":\"unregistered-live\"}\n")
            .expect("orphan timestamp source");
        let orphan = external_timestamp::stage(
            &track_root,
            &certificate_id,
            &orphan_source,
            input("legacy unregistered live orphan"),
        )
        .expect("orphan stage");
        external_timestamp::publish(&track_root, &orphan).expect("publish orphan fixture");
        drop(app);
        let error = WorkspaceApp::open(&workspace, false)
            .expect_err("unregistered live sidecar must block silent recovery");
        assert!(error
            .to_string()
            .contains("Unregistered external timestamp sidecar detected"));
    }

    #[test]
    fn finalization_reports_certificate_and_snapshot_verification_progress() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Progress Certificate",
        );
        let mut events = Vec::new();

        let finalized = app
            .finalize_track_with_progress(&ready.id, &mut |progress| events.push(progress))
            .expect("finalization with progress")
            .track
            .expect("finalized detail");

        assert!(finalized.certificate.valid);
        for stage in [
            "validating_finalization_gate",
            "collecting_final_snapshot",
            "writing_finalization_marker",
            "generating_certificate",
            "verifying_certificate",
            "verifying_final_snapshot",
            "verifying",
            "comparing_hashes",
            "saving_final_snapshot",
            "complete",
        ] {
            assert!(
                events.iter().any(|progress| progress.stage == stage),
                "missing finalization progress stage {stage}"
            );
        }
        assert!(events.iter().any(|progress| {
            progress.stage == "verifying"
                && progress.total_bytes > 0
                && progress.current_file.is_some()
        }));
        assert!(events.last().is_some_and(|progress| {
            progress.stage == "complete"
                && progress.processed_files == finalized.integrity.verified_count
                && progress.total_files == finalized.integrity.file_count
        }));
    }

    #[test]
    fn profile_updates_refresh_open_tracks_but_preserve_finalized_snapshots() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let original_profile = complete_profile();
        app.update_profile(original_profile.clone())
            .expect("original profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("finalized-fixtures"),
            "Frozen Profile",
        );
        let active = app
            .create_track(CreateTrackInput {
                title: "Current Profile".into(),
                production_start_date: "2026-08-10".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("active track");
        app.generate_documents(&active.id, false)
            .expect("initial active documents");

        let mut changed_profile = original_profile.clone();
        changed_profile.artist_name = "Updated Artist".into();
        changed_profile.suno_profile_name = "updated-profile".into();
        changed_profile.suno_handle = "@updated".into();
        app.update_profile(changed_profile.clone())
            .expect("updated profile");

        let refreshed = app.load_track(&active.id).expect("refreshed active track");
        assert_eq!(refreshed.profile_snapshot, changed_profile);
        assert!(!refreshed.documents.current);
        let frozen = app
            .load_track(&finalized.id)
            .expect("unchanged finalized track");
        assert_eq!(frozen.profile_snapshot, original_profile);

        app.generate_documents(&active.id, false)
            .expect("regenerated active documents");
        let readme = fs::read_to_string(
            app.root()
                .join(&refreshed.relative_path)
                .join("03_DOCUMENTATION/README.md"),
        )
        .expect("generated README");
        assert!(readme.contains("- Artist: Updated Artist"));
        assert!(readme.contains("- Suno profile: updated-profile"));
        assert!(readme.contains("- Suno handle: @updated"));
        assert!(!readme.contains("Artist: Not documented"));
    }

    #[test]
    fn certificate_language_change_preserves_open_outputs_and_freezes_finalization_options() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let original_profile = complete_profile();
        app.update_profile(original_profile.clone())
            .expect("original profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("ready-fixtures"),
            "Language-only Profile Update",
        );
        let before = app.load_track(&ready.id).expect("ready track");
        let documents_before = serde_json::to_value(&before.documents).expect("document state");
        let integrity_before = serde_json::to_value(&before.integrity).expect("integrity state");
        assert!(before.documents.current);
        assert!(before.integrity.verified);

        let mut german_profile = original_profile.clone();
        german_profile.certificate_language = CertificateLanguage::De;
        app.update_profile(german_profile.clone())
            .expect("language-only profile update");

        let unchanged = app.load_track(&ready.id).expect("unchanged ready track");
        assert_eq!(app.profile().expect("saved profile"), german_profile);
        assert_eq!(unchanged.profile_snapshot, original_profile);
        assert_eq!(
            serde_json::to_value(&unchanged.documents).expect("document state after update"),
            documents_before
        );
        assert_eq!(
            serde_json::to_value(&unchanged.integrity).expect("integrity state after update"),
            integrity_before
        );

        let finalized = app
            .finalize_track_with_options(&ready.id, FinalizeOptions { bilingual: true })
            .expect("finalization with bilingual option")
            .track
            .expect("finalized detail");
        assert_eq!(
            finalized.certificate.certificate_language,
            CertificateLanguage::De
        );
        assert!(finalized.certificate.bilingual);
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(
                app.root()
                    .join(&finalized.relative_path)
                    .join(certificate::MANIFEST_FILE),
            )
            .expect("finalized evidence manifest"),
        )
        .expect("manifest JSON");
        assert_eq!(manifest["certificate"]["rendering"]["language"], "de");
        assert_eq!(manifest["certificate"]["rendering"]["bilingual"], true);

        let mut english_profile = german_profile;
        english_profile.certificate_language = CertificateLanguage::En;
        app.update_profile(english_profile)
            .expect("subsequent language-only profile update");
        let frozen = app.load_track(&ready.id).expect("frozen finalized track");
        assert_eq!(frozen.status, TrackStatus::Finalized);
        assert_eq!(
            frozen.certificate.certificate_language,
            CertificateLanguage::De
        );
        assert!(frozen.certificate.bilingual);
    }

    #[test]
    fn reopening_assigns_saved_global_profile_to_existing_legacy_track() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let profile = complete_profile();
        app.update_profile(profile.clone())
            .expect("saved global profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("legacy-profile-fixtures"),
            "Legacy Global Profile",
        );
        let mut stale = app
            .persistence
            .track(&ready.id)
            .expect("stored ready track");
        stale.profile_snapshot = Profile::default();
        stale.legacy = true;
        app.persistence
            .save_track(&stale)
            .expect("stale track fixture");
        for step_id in ["track", "suno", "integrity", "finalize"] {
            app.persistence
                .save_step(
                    &stale.id,
                    &StepState {
                        id: step_id.into(),
                        status: StepStatus::NotVerified,
                        na_reason: None,
                        updated_at: Some("2026-08-14T00:00:00Z".into()),
                    },
                )
                .expect("stored legacy status");
        }
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
        let recovered = reopened.load_track(&ready.id).expect("recovered track");

        assert_eq!(recovered.profile_snapshot, profile);
        for step_id in ["track", "suno", "integrity", "finalize"] {
            assert_eq!(
                recovered
                    .steps
                    .iter()
                    .find(|step| step.id == step_id)
                    .map(|step| &step.status),
                Some(&StepStatus::Pass),
                "{step_id} did not recover from NOT_VERIFIED"
            );
        }
        let validation = reopened
            .validate_track(&ready.id)
            .expect("validate recovered track");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
    }

    #[test]
    fn monthly_subscription_coverage_uses_one_calendar_month() {
        assert_eq!(
            subscription_coverage_end("2026-08-15", SubscriptionBillingCycle::Monthly)
                .expect("monthly coverage"),
            "2026-09-14"
        );
        assert_eq!(
            serde_json::to_string(&SubscriptionBillingCycle::Monthly)
                .expect("monthly billing-cycle serialization"),
            "\"monthly\""
        );
    }

    #[test]
    fn annual_subscription_coverage_uses_twelve_calendar_months() {
        assert_eq!(
            subscription_coverage_end("2026-08-15", SubscriptionBillingCycle::Annual)
                .expect("annual coverage"),
            "2027-08-14"
        );
        assert_eq!(
            serde_json::to_string(&SubscriptionBillingCycle::Annual)
                .expect("annual billing-cycle serialization"),
            "\"annual\""
        );
    }

    #[test]
    fn monthly_subscription_coverage_clamps_month_end_before_subtracting_a_day() {
        assert_eq!(
            subscription_coverage_end("2026-01-31", SubscriptionBillingCycle::Monthly)
                .expect("month-end coverage"),
            "2026-02-27"
        );
    }

    #[test]
    fn subscription_coverage_handles_leap_years() {
        assert_eq!(
            subscription_coverage_end("2024-02-01", SubscriptionBillingCycle::Monthly)
                .expect("leap-month coverage"),
            "2024-02-29"
        );
        assert_eq!(
            subscription_coverage_end("2023-03-01", SubscriptionBillingCycle::Annual)
                .expect("annual coverage ending in a leap year"),
            "2024-02-29"
        );
        assert_eq!(
            subscription_coverage_end("2024-02-29", SubscriptionBillingCycle::Annual)
                .expect("annual coverage beginning on leap day"),
            "2025-02-27"
        );
    }

    #[test]
    fn subscription_coverage_rejects_invalid_start_dates() {
        let error = subscription_coverage_end("2026-02-30", SubscriptionBillingCycle::Monthly)
            .expect_err("invalid coverage start must fail");
        assert!(matches!(error, AppError::Validation(_)));
        assert!(error
            .to_string()
            .contains("Subscription coverage start must use YYYY-MM-DD"));
    }

    #[test]
    fn billing_cycle_registration_derives_and_persists_exact_coverage_dates() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let monthly_source = directory.path().join("subscription-monthly.pdf");
        let annual_source = directory.path().join("subscription-annual.pdf");
        fs::write(&monthly_source, b"%PDF-1.7\nmonthly receipt\n%%EOF\n")
            .expect("monthly subscription fixture");
        fs::write(&annual_source, b"%PDF-1.7\nannual receipt\n%%EOF\n")
            .expect("annual subscription fixture");

        let monthly = app
            .register_global_evidence_for_billing_cycle(
                EvidenceRole::SubscriptionPayment,
                &monthly_source,
                "2026-08-01",
                SubscriptionBillingCycle::Monthly,
            )
            .expect("monthly billing-cycle registration");
        let annual = app
            .register_global_evidence_for_billing_cycle(
                EvidenceRole::SubscriptionPayment,
                &annual_source,
                "2026-08-01",
                SubscriptionBillingCycle::Annual,
            )
            .expect("annual billing-cycle registration");

        assert_eq!(
            monthly.evidence.coverage_start.as_deref(),
            Some("2026-08-01")
        );
        assert_eq!(monthly.evidence.coverage_end.as_deref(), Some("2026-08-31"));
        assert_eq!(
            annual.evidence.coverage_start.as_deref(),
            Some("2026-08-01")
        );
        assert_eq!(annual.evidence.coverage_end.as_deref(), Some("2027-07-31"));
        assert_eq!(
            fs::read(&monthly_source).expect("preserved monthly source"),
            b"%PDF-1.7\nmonthly receipt\n%%EOF\n"
        );
        assert_eq!(
            fs::read(&annual_source).expect("preserved annual source"),
            b"%PDF-1.7\nannual receipt\n%%EOF\n"
        );

        drop(app);
        let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
        let persisted = reopened
            .global_evidence()
            .expect("persisted global evidence");
        assert_eq!(persisted.len(), 2);
        for registered in [monthly, annual] {
            let stored = persisted
                .iter()
                .find(|item| item.evidence.id == registered.evidence.id)
                .expect("registered billing-cycle evidence persisted");
            assert_eq!(
                stored.evidence.coverage_start,
                registered.evidence.coverage_start
            );
            assert_eq!(
                stored.evidence.coverage_end,
                registered.evidence.coverage_end
            );
        }
    }

    #[test]
    fn global_terms_pdf_import_requires_core_metadata_and_propagates_to_mutable_tracks() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Local Evidence Metadata".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: true,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        app.update_track(
            &track.id,
            TrackPatch {
                suno_terms_evidence_not_available: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("explicit unavailable status");
        let immutable_track = app
            .create_track(CreateTrackInput {
                title: "Immutable Before Global Terms".into(),
                production_start_date: "2026-07-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track that will represent a finalized snapshot");
        let mut immutable_record = app
            .persistence
            .track(&immutable_track.id)
            .expect("immutable track record");
        immutable_record.status = TrackStatus::Finalized;
        app.persistence
            .save_track(&immutable_record)
            .expect("mark immutable fixture finalized");

        let invalid_terms_source = directory.path().join("suno-terms.txt");
        fs::write(&invalid_terms_source, b"not an accepted terms PDF\n")
            .expect("invalid terms fixture");
        let terms_metadata = EvidenceMetadata {
            document_title: "Suno Terms of Service".into(),
            provider: "Suno, Inc.".into(),
            source_url: "https://suno.com/terms".into(),
            retrieval_date: "2026-08-01".into(),
            effective_date: "2026-07-01".into(),
            applicable_production_period: "2026-07 through 2026-08".into(),
            factual_note: "Locally archived terms edition.".into(),
            ..EvidenceMetadata::default()
        };
        assert!(matches!(
            app.register_global_terms_evidence(&invalid_terms_source, terms_metadata.clone()),
            Err(AppError::FileType { .. })
        ));
        let disguised_pdf = directory.path().join("disguised-terms.pdf");
        fs::write(&disguised_pdf, b"plain text with a PDF extension\n")
            .expect("disguised terms fixture");
        assert!(matches!(
            app.register_global_terms_evidence(&disguised_pdf, terms_metadata.clone()),
            Err(AppError::Validation(_))
        ));

        let terms_source = directory.path().join("suno-terms-2026-08-01.pdf");
        fs::write(
            &terms_source,
            b"%PDF-1.7\n1 0 obj\n<</Type /Terms>>\nendobj\n%%EOF\n",
        )
        .expect("terms PDF fixture");
        assert!(app
            .register_global_terms_evidence(&terms_source, EvidenceMetadata::default())
            .expect_err("incomplete terms metadata")
            .to_string()
            .contains("descriptive metadata is incomplete"));
        let global_terms = app
            .register_global_terms_evidence(&terms_source, terms_metadata.clone())
            .expect("global terms import");
        assert_eq!(global_terms.evidence.role, EvidenceRole::SunoTermsRights);
        assert_eq!(
            global_terms.evidence.metadata.original_file_name,
            "suno-terms-2026-08-01.pdf"
        );
        assert_eq!(
            global_terms.evidence.metadata.document_title,
            "Suno Terms of Service"
        );
        assert_eq!(global_terms.evidence.metadata.provider, "Suno, Inc.");
        assert_eq!(global_terms.evidence.metadata.retrieval_date, "2026-08-01");
        let terms = app.load_track(&track.id).expect("track with global terms");
        assert_eq!(terms.fields.suno_terms_evidence_not_available, Some(false));
        let error = app
            .update_track(
                &track.id,
                TrackPatch {
                    suno_terms_evidence_not_available: Some(true),
                    ..TrackPatch::default()
                },
            )
            .expect_err("verified Terms evidence must reject an unavailable claim");
        assert!(error.to_string().contains(
            "Terms evidence cannot be marked unavailable while a verified local Terms evidence file is attached."
        ));
        let terms = app.load_track(&track.id).expect("unchanged Terms status");
        assert_eq!(terms.fields.suno_terms_evidence_not_available, Some(false));
        let terms_item = terms
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SunoTermsRights)
            .expect("terms evidence");
        assert_eq!(
            terms_item.metadata.original_file_name,
            "suno-terms-2026-08-01.pdf"
        );
        assert_eq!(terms_item.metadata.provider, "Suno, Inc.");
        assert_eq!(
            terms_item.source_global_evidence_id.as_deref(),
            Some(global_terms.evidence.id.as_str())
        );
        assert_eq!(terms_item.provenance, EvidenceProvenance::GlobalCopy);
        assert!(!app
            .load_track(&immutable_track.id)
            .expect("unchanged finalized track")
            .evidence
            .iter()
            .any(|item| item.role == EvidenceRole::SunoTermsRights));

        let edited_metadata = EvidenceMetadata {
            document_title: "Suno Terms of Service — archived edition".into(),
            provider: "Suno, Inc.".into(),
            source_url: "https://suno.com/terms".into(),
            retrieval_date: "2026-08-01".into(),
            effective_date: "2026-07-01".into(),
            applicable_production_period: "Production during August 2026".into(),
            factual_note: "Context corrected before track finalization.".into(),
            ..EvidenceMetadata::default()
        };
        let edited = app
            .update_global_terms_evidence_metadata(
                &global_terms.evidence.id,
                edited_metadata.clone(),
            )
            .expect("edit terms metadata");
        assert_eq!(
            edited.evidence.metadata.document_title,
            "Suno Terms of Service — archived edition"
        );
        let updated_copy = app
            .load_track(&track.id)
            .expect("track after terms metadata edit")
            .evidence
            .into_iter()
            .find(|item| item.role == EvidenceRole::SunoTermsRights)
            .expect("updated portable terms copy");
        assert_eq!(updated_copy.metadata, edited.evidence.metadata);
        assert!(!app
            .load_track(&immutable_track.id)
            .expect("finalized track remains unchanged after metadata edit")
            .evidence
            .iter()
            .any(|item| item.role == EvidenceRole::SunoTermsRights));

        let later_track = app
            .create_track(CreateTrackInput {
                title: "Created After Global Terms".into(),
                production_start_date: "2026-08-02".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("later track");
        assert!(later_track.evidence.iter().any(|item| {
            item.role == EvidenceRole::SunoTermsRights
                && item.source_global_evidence_id.as_deref()
                    == Some(global_terms.evidence.id.as_str())
        }));

        let timestamp_source = directory.path().join("external-timestamp.json");
        fs::write(&timestamp_source, b"{\"timestamp\":\"fixture\"}\n").expect("timestamp fixture");
        let invalid = app.import_evidence_with_metadata_from(
            &track.id,
            EvidenceRole::ExternalTimestamp,
            &timestamp_source,
            EvidenceMetadata::default(),
        );
        assert!(invalid
            .expect_err("pre-finalization timestamp route is disabled")
            .to_string()
            .contains("after technical finalization"));

        drop(app);
        let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
        let stored_global = reopened.global_evidence().expect("global evidence");
        assert!(stored_global.iter().any(|item| {
            item.evidence.id == global_terms.evidence.id
                && item.evidence.metadata.original_file_name == "suno-terms-2026-08-01.pdf"
        }));
        let evidence = reopened
            .load_track(&track.id)
            .expect("reopened track")
            .evidence;
        assert!(evidence.iter().any(|item| {
            item.role == EvidenceRole::SunoTermsRights
                && item.metadata.original_file_name == "suno-terms-2026-08-01.pdf"
                && item.metadata.document_title == "Suno Terms of Service — archived edition"
        }));
    }

    fn track_tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut result = BTreeMap::new();
        for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
            let entry = entry.expect("read track tree");
            let relative = entry.path().strip_prefix(root).expect("relative path");
            let key = portable_relative(relative);
            let value = if entry.file_type().is_dir() {
                b"<directory>".to_vec()
            } else {
                fs::read(entry.path()).expect("read track fixture")
            };
            result.insert(key, value);
        }
        result
    }

    #[test]
    fn track_patches_clear_values_from_inactive_conditional_branches() {
        let mut fields = crate::model::TrackFields {
            lyrics_source: "human".into(),
            lyrics_text: "stale human lyrics".into(),
            external_audio_uploaded: Some(true),
            external_audio_source: "stale external source".into(),
            external_audio_ownership: "stale external rights".into(),
            own_audio_uploaded: Some(true),
            own_audio_source: "stale own source".into(),
            own_audio_ownership: "stale own rights".into(),
            third_party_samples_uploaded: Some(true),
            third_party_sample_source: "stale sample source".into(),
            third_party_sample_ownership: "stale sample rights".into(),
            human_editing_performed: Some(true),
            human_editing_details: "stale human edit".into(),
            post_export_editing_performed: Some(true),
            post_export_editing_details: "stale post edit".into(),
            artwork_origin: "ai_assisted".into(),
            ai_image_service: "stale AI service".into(),
            human_artwork_modifications: vec!["stale artwork modifications".into()],
            depicts_real_person: Some(true),
            real_person_notes: "stale person note".into(),
            depicts_real_event: Some(true),
            real_event_notes: "stale event note".into(),
            contains_trademark: Some(true),
            trademark_notes: "stale trademark note".into(),
            disclosure_applied: Some(true),
            disclosure_text: "stale disclosure".into(),
            ..crate::model::TrackFields::default()
        };

        apply_patch(
            &mut fields,
            TrackPatch {
                lyrics_source: Some("instrumental".into()),
                external_audio_uploaded: Some(false),
                own_audio_uploaded: Some(false),
                code_based_generation: Some(false),
                third_party_samples_uploaded: Some(false),
                human_editing_performed: Some(false),
                post_export_editing_performed: Some(false),
                artwork_origin: Some("human".into()),
                depicts_real_person: Some(false),
                depicts_real_event: Some(false),
                contains_trademark: Some(false),
                ..TrackPatch::default()
            },
        );

        assert_eq!(fields.lyrics_text, "stale human lyrics");
        assert_eq!(fields.lyrics_source, "instrumental");
        for value in [
            &fields.external_audio_source,
            &fields.external_audio_ownership,
            &fields.own_audio_source,
            &fields.own_audio_ownership,
            &fields.third_party_sample_source,
            &fields.third_party_sample_ownership,
            &fields.human_editing_details,
            &fields.post_export_editing_details,
            &fields.ai_image_service,
            &fields.real_person_notes,
            &fields.real_event_notes,
            &fields.trademark_notes,
            &fields.disclosure_text,
        ] {
            assert!(value.is_empty(), "inactive value survived: {value}");
        }
        assert!(fields.human_artwork_modifications.is_empty());
        assert_eq!(fields.disclosure_applied, None);
        assert_eq!(fields.depicts_real_person, Some(false));

        apply_patch(
            &mut fields,
            TrackPatch {
                artwork_origin: Some("none".into()),
                ..TrackPatch::default()
            },
        );
        assert_eq!(fields.depicts_real_person, None);
        assert_eq!(fields.depicts_real_event, None);
        assert_eq!(fields.contains_trademark, None);

        let mut expected = fields.clone();
        expected.lyrics_text = "updated legacy value".into();
        apply_patch(
            &mut fields,
            TrackPatch {
                lyrics_text: Some("updated legacy value".into()),
                real_person_notes: Some("ignored hidden note".into()),
                ..TrackPatch::default()
            },
        );
        assert_eq!(
            fields, expected,
            "legacy lyrics remain editable, while inactive artwork details stay ignored"
        );
    }

    #[test]
    fn desktop_patch_null_clears_documented_nullable_facts_but_omission_preserves_them() {
        let mut fields = crate::model::TrackFields {
            instrumental_track: Some(true),
            vocal_lyrics_present: Some(false),
            generative_ai_used: Some(true),
            audio_ai_system: "Suno".into(),
            ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
            ..crate::model::TrackFields::default()
        };
        let request: TrackPatchRequest = serde_json::from_value(serde_json::json!({
            "instrumentalTrack": null,
            "generativeAiUsed": null,
            "aiGeneratedAudioElements": null
        }))
        .expect("desktop track patch");

        apply_patch_with_explicit_nulls(&mut fields, request.patch, &request.explicit_null_fields);

        assert_eq!(fields.instrumental_track, None);
        assert_eq!(fields.generative_ai_used, None);
        assert_eq!(fields.ai_generated_audio_elements, None);
        assert_eq!(fields.vocal_lyrics_present, Some(false));
        assert_eq!(fields.audio_ai_system, "Suno");
    }

    fn certificate_file_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        [
            certificate::CERTIFICATE_FILE,
            certificate::MANIFEST_FILE,
            certificate::CERTIFICATE_HASH_FILE,
            certificate::PDF_FILE,
        ]
        .into_iter()
        .map(|relative| {
            (
                relative.to_owned(),
                fs::read(root.join(relative)).expect("read certificate file"),
            )
        })
        .collect()
    }

    fn parse_certificate_document(content: &str) -> ParsedCertificate {
        enum Section {
            Fields,
            CompletedSteps,
            NaReasons,
            Other,
        }

        let mut fields = BTreeMap::new();
        let mut completed_steps = BTreeMap::new();
        let mut na_reasons = BTreeMap::new();
        let mut section = Section::Fields;
        for line in content.lines() {
            section = match line {
                "## J. Evidence register" => Section::Other,
                "## K. Integrity anchors and workflow" => Section::Fields,
                "### Mandatory steps completed" => Section::CompletedSteps,
                "### N/A steps with reasons" => Section::NaReasons,
                "## L. Technical certificate statement" => Section::Other,
                _ => section,
            };
            if !line.starts_with("- ") {
                continue;
            }
            let entry = &line[2..];
            match section {
                Section::Fields => {
                    let Some((key, value)) = entry.split_once(": ") else {
                        continue;
                    };
                    assert!(
                        fields
                            .insert(key.into(), unquote_certificate_value(value))
                            .is_none(),
                        "duplicate certificate field: {key}"
                    );
                }
                Section::CompletedSteps => {
                    let (step_id, status) = entry
                        .split_once(": ")
                        .unwrap_or_else(|| panic!("malformed completed step: {line}"));
                    assert!(
                        completed_steps
                            .insert(step_id.into(), status.into())
                            .is_none(),
                        "duplicate completed step: {step_id}"
                    );
                }
                Section::NaReasons if !entry.eq_ignore_ascii_case("none") => {
                    let (step_id, reason) = entry
                        .split_once(" — ")
                        .unwrap_or_else(|| panic!("malformed N/A reason: {line}"));
                    assert!(
                        na_reasons.insert(step_id.into(), reason.into()).is_none(),
                        "duplicate N/A step: {step_id}"
                    );
                }
                Section::Other if entry.starts_with("Evidence file count: ") => {
                    let (key, value) = entry
                        .split_once(": ")
                        .expect("evidence count certificate field");
                    fields.insert(key.into(), unquote_certificate_value(value));
                }
                Section::NaReasons | Section::Other => {}
            }
        }
        ParsedCertificate {
            fields,
            completed_steps,
            na_reasons,
        }
    }

    fn unquote_certificate_value(value: &str) -> String {
        value
            .strip_prefix('`')
            .and_then(|value| value.strip_suffix('`'))
            .or_else(|| {
                value
                    .strip_prefix("**")
                    .and_then(|value| value.strip_suffix("**"))
            })
            .unwrap_or(value)
            .replace('`', "")
    }

    fn parse_sha256sums(content: &str) -> BTreeMap<String, String> {
        content
            .lines()
            .map(|line| {
                let (digest, relative) = line
                    .split_once("  ")
                    .unwrap_or_else(|| panic!("malformed SHA256SUMS entry: {line}"));
                assert_eq!(digest.len(), 64, "SHA-256 digest length for {relative}");
                assert!(
                    digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "non-hex SHA-256 digest for {relative}"
                );
                (relative.into(), digest.to_ascii_lowercase())
            })
            .collect()
    }

    fn track_record_without_library_path(track: &TrackRecord) -> serde_json::Value {
        let mut value = serde_json::to_value(track).expect("serializable track record");
        let object = value.as_object_mut().expect("track record object");
        object.remove("library");
        object.remove("relativePath");
        value
    }

    fn manifest_string<'a>(manifest: &'a serde_json::Value, pointer: &str) -> &'a str {
        manifest
            .pointer(pointer)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("manifest field {pointer} must be a string"))
    }

    #[test]
    fn workspace_creation_initializes_local_database() {
        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("workspace");
        let app = WorkspaceApp::open(&root, true).expect("create workspace");
        assert!(root.join(".suno-doc/workspace.sqlite").is_file());
        assert!(root.join(SINGLES_DIRECTORY).is_dir());
        assert_eq!(app.summary().expect("summary").track_count, 0);
    }

    #[test]
    fn album_creation_persists_an_empty_folder_and_supports_rename() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");

        assert_eq!(
            app.create_album("  Gravity Drift  ")
                .expect("create empty album"),
            vec!["Gravity Drift"]
        );
        assert!(workspace.join("Gravity Drift").is_dir());
        assert!(workspace.join(SINGLES_DIRECTORY).is_dir());
        assert_eq!(
            app.scan_workspace().expect("scan empty album").discovered,
            0,
            "an empty album must not be indexed as a track"
        );

        app.rename_album("Gravity Drift", "Gravity Drive")
            .expect("rename empty album");
        assert!(!workspace.join("Gravity Drift").exists());
        assert!(workspace.join("Gravity Drive").is_dir());
        assert_eq!(
            app.list_albums().expect("album list"),
            vec!["Gravity Drive"]
        );
        assert!(matches!(
            app.create_album("gravity drive")
                .expect_err("case-insensitive duplicate must fail"),
            AppError::Collision(_)
        ));
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
        assert_eq!(
            reopened.list_albums().expect("reopened album list"),
            vec!["Gravity Drive"]
        );
        assert!(workspace.join(SINGLES_DIRECTORY).is_dir());
    }

    #[test]
    fn hidden_workspace_folders_are_pruned_from_album_and_track_discovery() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");

        fs::create_dir_all(workspace.join(".archive/Archived Track/01_RELEASE"))
            .expect("hidden archive fixture");
        fs::create_dir_all(workspace.join(".draft/01_RELEASE"))
            .expect("hidden direct-track fixture");
        fs::create_dir_all(workspace.join("Visible Album/Visible Track/01_RELEASE"))
            .expect("visible album fixture");

        assert_eq!(
            app.list_albums().expect("album list"),
            vec!["Visible Album"]
        );
        let scan = app.scan_workspace().expect("workspace scan");
        assert_eq!(scan.discovered, 1);
        assert_eq!(scan.indexed, 1);
        assert!(scan.warnings.is_empty());
        assert_eq!(
            scan.candidates[0].relative_path,
            "Visible Album/Visible Track"
        );
        assert!(app
            .persistence
            .track_by_relative_path(".archive/Archived Track")
            .expect("hidden archive lookup")
            .is_none());
        assert!(app
            .persistence
            .track_by_relative_path(".draft")
            .expect("hidden direct-track lookup")
            .is_none());
    }

    #[test]
    fn previously_indexed_hidden_paths_remain_unloaded_after_reopen() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Archived Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let source = workspace.join(&created.relative_path);
        let hidden_parent = workspace.join(".archive");
        let target = hidden_parent.join("Archived Track");
        fs::create_dir(&hidden_parent).expect("hidden parent");
        fs::rename(&source, &target).expect("move fixture into hidden parent");

        let mut record = app.persistence.track(&created.id).expect("stored track");
        record.relative_path = ".archive/Archived Track".into();
        record.library = TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some(".archive".into()),
        };
        app.persistence
            .save_track(&record)
            .expect("persist pre-fix hidden path fixture");

        assert!(app.list_tracks().expect("visible tracks").is_empty());
        assert_eq!(app.summary().expect("visible summary").track_count, 0);
        assert!(matches!(
            app.load_track(&created.id),
            Err(AppError::TrackNotFound(id)) if id == created.id
        ));
        assert!(target.join(TRACK_IDENTITY_FILE).is_file());
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
        assert!(reopened.list_tracks().expect("reopened tracks").is_empty());
        assert!(reopened.list_albums().expect("reopened albums").is_empty());
        assert_eq!(
            reopened.scan_workspace().expect("reopened scan").discovered,
            0
        );
        assert!(target.join(TRACK_IDENTITY_FILE).is_file());
    }

    #[test]
    fn track_creation_builds_exact_folders() {
        let directory = tempdir().expect("tempdir");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Acceptance Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let root = app.root().join(&track.relative_path);
        for folder in TRACK_FOLDERS {
            assert!(root.join(folder).is_dir(), "{folder}");
        }
    }

    #[test]
    fn track_creation_persists_album_library_placement_after_reopen() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");

        let created = app
            .create_track(CreateTrackInput {
                title: "Album Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("  Night Drive  ".into()),
                },
            })
            .expect("album track");
        assert_eq!(created.library.section, TrackLibrarySection::Album);
        assert_eq!(created.library.album_title.as_deref(), Some("Night Drive"));
        assert_eq!(created.relative_path, "Night Drive/Album Track");
        assert!(workspace.join("Night Drive/Album Track").is_dir());
        assert_eq!(crate::persistence::SCHEMA_VERSION, 7);
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
        let detail = reopened.load_track(&created.id).expect("reopened track");
        assert_eq!(detail.library, created.library);
        let summary = reopened
            .list_tracks()
            .expect("track summaries")
            .into_iter()
            .find(|track| track.id == created.id)
            .expect("album summary");
        assert_eq!(summary.library, created.library);
    }

    #[test]
    fn new_guided_and_free_text_track_values_survive_workspace_reopen_exactly() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");

        let assisted = app
            .create_track(CreateTrackInput {
                title: "Future Model Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("AI-assisted track");
        app.update_track(
            &assisted.id,
            TrackPatch {
                suno_model: Some("v6 private preview".into()),
                suno_plan_at_generation: Some("Historical Founder Plan".into()),
                code_based_generation: Some(true),
                code_audio_post_processed: Some(true),
                code_audio_post_processing_operations: Some(vec![
                    "Mixing".into(),
                    "EQ".into(),
                    "Other post-processing".into(),
                ]),
                code_audio_post_processing_note: Some("Manual spectral repair".into()),
                artwork_origin: Some("ai_assisted".into()),
                ai_image_service: Some("Future Image Tool".into()),
                human_artwork_modifications: Some(vec![
                    "Cropping".into(),
                    "Typography added".into(),
                    "Other human editing".into(),
                ]),
                custom_artwork_change: Some("Hand-painted edge cleanup".into()),
                depicts_real_person: Some(true),
                real_person_notes: Some("A named collaborator in the foreground".into()),
                depicts_real_event: Some(false),
                contains_trademark: Some(true),
                trademark_notes: Some("A supplied sponsor logo in the corner".into()),
                ..TrackPatch::default()
            },
        )
        .expect("persist AI-assisted values");

        let human = app
            .create_track(CreateTrackInput {
                title: "Human Artwork Track".into(),
                production_start_date: "2026-08-02".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("human-artwork track");
        app.update_track(
            &human.id,
            TrackPatch {
                artwork_origin: Some("human".into()),
                human_artwork_process_operations: Some(vec![
                    "Photographed".into(),
                    "Color correction".into(),
                    "Typography added".into(),
                ]),
                human_artwork_process_notes: Some("35 mm scan, then manual layout".into()),
                depicts_real_person: Some(false),
                depicts_real_event: Some(false),
                contains_trademark: Some(false),
                ..TrackPatch::default()
            },
        )
        .expect("persist human-artwork values");
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopen workspace");
        let assisted = reopened
            .load_track(&assisted.id)
            .expect("reload AI-assisted track");
        assert_eq!(assisted.fields.suno_model, "v6 private preview");
        assert_eq!(
            assisted.fields.suno_plan_at_generation,
            "Historical Founder Plan"
        );
        assert_eq!(assisted.fields.code_audio_post_processed, Some(true));
        assert_eq!(
            assisted.fields.code_audio_post_processing_operations,
            vec!["Mixing", "EQ", "Other post-processing"]
        );
        assert_eq!(
            assisted.fields.code_audio_post_processing_note,
            "Manual spectral repair"
        );
        assert_eq!(
            assisted.fields.human_artwork_modifications,
            vec!["Cropping", "Typography added", "Other human editing"]
        );
        assert_eq!(
            assisted.fields.custom_artwork_change,
            "Hand-painted edge cleanup"
        );
        assert_eq!(assisted.fields.depicts_real_person, Some(true));
        assert_eq!(assisted.fields.depicts_real_event, Some(false));
        assert_eq!(assisted.fields.contains_trademark, Some(true));
        assert_eq!(
            assisted.fields.real_person_notes,
            "A named collaborator in the foreground"
        );
        assert_eq!(
            assisted.fields.trademark_notes,
            "A supplied sponsor logo in the corner"
        );

        let human = reopened
            .load_track(&human.id)
            .expect("reload human-artwork track");
        assert_eq!(
            human.fields.human_artwork_process_operations,
            vec!["Photographed", "Color correction", "Typography added"]
        );
        assert_eq!(
            human.fields.human_artwork_process_notes,
            "35 mm scan, then manual layout"
        );
        assert_eq!(human.fields.depicts_real_person, Some(false));
        assert_eq!(human.fields.depicts_real_event, Some(false));
        assert_eq!(human.fields.contains_trademark, Some(false));
    }

    #[test]
    fn older_track_json_defaults_to_single_library_section_without_rewriting_it() {
        let legacy_input: CreateTrackInput = serde_json::from_value(serde_json::json!({
            "title": "Legacy API Input",
            "productionStartDate": "2026-08-01",
            "commercialUseIntended": false
        }))
        .expect("legacy create input");
        assert_eq!(legacy_input.library, TrackLibraryPlacement::default());

        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Pre Library Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");

        let record = app.persistence.track(&created.id).expect("stored track");
        let mut legacy_json = serde_json::to_value(record).expect("track JSON");
        legacy_json
            .as_object_mut()
            .expect("track object")
            .remove("library");
        legacy_json
            .pointer_mut("/fields")
            .and_then(serde_json::Value::as_object_mut)
            .expect("track fields")
            .remove("codeBasedGeneration");
        app.persistence
            .open()
            .expect("database")
            .execute(
                "UPDATE tracks SET data_json=?1 WHERE id=?2",
                rusqlite::params![
                    serde_json::to_string(&legacy_json).expect("legacy JSON"),
                    created.id
                ],
            )
            .expect("remove library field from stored fixture");

        let defaulted = app.persistence.track(&created.id).expect("defaulted track");
        assert_eq!(defaulted.library, TrackLibraryPlacement::default());
        assert_eq!(defaulted.fields.code_based_generation, None);
        let loaded = app.load_track(&created.id).expect("load defaulted track");
        assert_eq!(loaded.library, TrackLibraryPlacement::default());
        assert_eq!(loaded.fields.code_based_generation, None);
        let materialized_json: String = app
            .persistence
            .open()
            .expect("database")
            .query_row(
                "SELECT data_json FROM tracks WHERE id=?1",
                [&created.id],
                |row| row.get(0),
            )
            .expect("materialized track JSON");
        assert!(
            serde_json::from_str::<serde_json::Value>(&materialized_json)
                .expect("materialized JSON")
                .pointer("/library")
                .is_none(),
            "loading a legacy record must not rewrite its JSON merely to add defaults"
        );
    }

    #[test]
    fn legacy_scan_defaults_library_placement_without_modifying_track_files() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Historical Track");
        fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy directory");
        fs::write(
            legacy_root.join("01_RELEASE/history.wav"),
            b"RIFF\x08\0\0\0WAVEhistorical track",
        )
        .expect("legacy file");
        let before = track_tree_snapshot(&legacy_root);

        app.scan_workspace().expect("legacy scan");
        let indexed = app
            .persistence
            .track_by_relative_path("Historical Track")
            .expect("legacy lookup")
            .expect("indexed legacy track");
        assert!(indexed.legacy);
        assert_eq!(indexed.library, TrackLibraryPlacement::default());
        assert_eq!(track_tree_snapshot(&legacy_root), before);
    }

    #[test]
    fn track_library_validation_rejects_invalid_albums_and_normalizes_singles() {
        for album_title in [
            None,
            Some(String::new()),
            Some("   ".into()),
            Some(".archive".into()),
            Some("  .private  ".into()),
            Some("invalid\ncontrol".into()),
            Some("x".repeat(201)),
        ] {
            assert!(matches!(
                normalize_track_library(TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title,
                }),
                Err(AppError::Validation(_))
            ));
        }
        assert!(
            serde_json::from_value::<TrackLibraryPlacement>(serde_json::json!({
                "section": "ep"
            }))
            .is_err()
        );
        assert_eq!(
            normalize_track_library(TrackLibraryPlacement {
                section: TrackLibrarySection::Single,
                album_title: Some("Ignored album".into()),
            })
            .expect("single normalization"),
            TrackLibraryPlacement::default()
        );
        assert_eq!(
            serde_json::to_value(TrackLibraryPlacement::default()).expect("single serialization"),
            serde_json::json!({ "section": "single" })
        );

        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let error = app
            .create_track(CreateTrackInput {
                title: "Invalid Album".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: None,
                },
            })
            .expect_err("album without a title must fail");
        assert!(matches!(error, AppError::Validation(_)));
        assert!(!app.root().join("Invalid-Album").exists());
        assert!(app.list_tracks().expect("empty track list").is_empty());
    }

    #[test]
    fn library_reclassification_preserves_active_track_state_and_files() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Immutable Library Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let track_root = app.root().join(&created.relative_path);
        fs::write(
            track_root.join("03_DOCUMENTATION/library-sentinel.bin"),
            b"library placement must not touch track files",
        )
        .expect("track sentinel");

        let mut record = app.persistence.track(&created.id).expect("stored track");
        record.status = TrackStatus::Active;
        record.documents = DocumentState {
            generated: true,
            current: true,
            generated_at: Some("2026-08-01T12:00:00Z".into()),
            template_version: "preserved-template".into(),
            files: vec!["03_DOCUMENTATION/library-sentinel.bin".into()],
            input_fingerprint: "preserved-fingerprint".into(),
        };
        record.integrity = IntegrityState {
            generated: true,
            verified: true,
            file_count: 7,
            verified_count: 7,
            generated_at: Some("2026-08-01T12:01:00Z".into()),
            verified_at: Some("2026-08-01T12:02:00Z".into()),
            mismatch_files: Vec::new(),
        };
        record.certificate = CertificateState {
            valid: true,
            certificate_id: Some("SDM-library-preservation".into()),
            finalization_snapshot_id: None,
            finalized_at: Some("2026-08-01T12:03:00Z".into()),
            workflow_version: Some(record.workflow_version.clone()),
            certificate_language: CertificateLanguage::En,
            bilingual: false,
            invalidated_at: None,
            invalidation_reason: None,
        };
        record.updated_at = "2026-08-01T12:04:00Z".into();
        app.persistence
            .save_track(&record)
            .expect("finalized fixture");
        let protected_before = track_record_without_library_path(&record);
        let tree_before = track_tree_snapshot(&track_root);

        let album = app
            .update_track_library(
                &created.id,
                TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("  Preserved Album  ".into()),
                },
            )
            .expect("active album reassignment");
        assert_eq!(album.status, TrackStatus::Active);
        assert_eq!(album.updated_at, "2026-08-01T12:04:00Z");
        assert_eq!(album.library.section, TrackLibrarySection::Album);
        assert_eq!(
            album.library.album_title.as_deref(),
            Some("Preserved Album")
        );
        let album_record = app.persistence.track(&created.id).expect("stored album");
        assert_eq!(
            track_record_without_library_path(&album_record),
            protected_before
        );
        let album_root = app.root().join(&album.relative_path);
        assert_eq!(track_tree_snapshot(&album_root), tree_before);
        assert!(!track_root.exists());

        let single = app
            .update_track_library(
                &created.id,
                TrackLibraryPlacement {
                    section: TrackLibrarySection::Single,
                    album_title: Some("must be cleared".into()),
                },
            )
            .expect("active single reassignment");
        assert_eq!(single.library, TrackLibraryPlacement::default());
        let single_record = app.persistence.track(&created.id).expect("stored single");
        assert_eq!(
            track_record_without_library_path(&single_record),
            protected_before
        );
        let single_root = app.root().join(&single.relative_path);
        assert_eq!(track_tree_snapshot(&single_root), tree_before);
        assert!(!album_root.exists(), "the track left its former album path");
        assert!(
            app.root().join("Preserved Album").is_dir(),
            "empty album folders remain reusable"
        );
    }

    #[test]
    fn album_rename_moves_the_folder_and_updates_every_member_path() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let library = TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some("Gravity Drift".into()),
        };
        let first = app
            .create_track(CreateTrackInput {
                title: "Gravaty".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library,
            })
            .expect("first album track");
        let second = app
            .create_track(CreateTrackInput {
                title: "Orbit".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("gravity drift".into()),
                },
            })
            .expect("second album track");
        assert_eq!(second.relative_path, "Gravity Drift/Orbit");
        fs::write(
            app.root()
                .join(&first.relative_path)
                .join("01_RELEASE/master.wav"),
            b"album rename sentinel",
        )
        .expect("sentinel");

        let renamed = app
            .rename_album("Gravity Drift", "Gravity Drive")
            .expect("rename album");

        assert!(!app.root().join("Gravity Drift").exists());
        assert!(app.root().join("Gravity Drive/Gravaty").is_dir());
        assert!(app.root().join("Gravity Drive/Orbit").is_dir());
        assert_eq!(
            fs::read(
                app.root()
                    .join("Gravity Drive/Gravaty/01_RELEASE/master.wav")
            )
            .expect("preserved sentinel"),
            b"album rename sentinel"
        );
        for id in [&first.id, &second.id] {
            let summary = renamed
                .iter()
                .find(|track| &track.id == id)
                .expect("renamed member");
            assert!(summary.relative_path.starts_with("Gravity Drive/"));
            assert_eq!(
                summary.library.album_title.as_deref(),
                Some("Gravity Drive")
            );
        }
    }

    #[test]
    fn changing_a_track_title_renames_its_managed_folder() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Old Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let old_root = app.root().join(&created.relative_path);
        fs::write(old_root.join("02_SUNO/source.wav"), b"rename sentinel").expect("sentinel");
        let release_source = directory.path().join("original-master.wav");
        fs::write(&release_source, b"RIFF\x08\0\0\0WAVErename release").expect("release fixture");
        let imported = app
            .import_evidence_from(&created.id, EvidenceRole::ReleaseWav, &release_source)
            .expect("release import before title rename");
        let old_release = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("old release evidence");
        assert_eq!(old_release.relative_path, "01_RELEASE/Old Track.wav");

        let renamed = app
            .update_track(
                &created.id,
                TrackPatch {
                    title: Some("New Track".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("rename track");

        assert_eq!(renamed.relative_path, "Singles/New Track");
        assert!(!old_root.exists());
        assert_eq!(
            fs::read(app.root().join("Singles/New Track/02_SUNO/source.wav"))
                .expect("preserved sentinel"),
            b"rename sentinel"
        );
        let renamed_release = renamed
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("renamed release evidence");
        assert_eq!(renamed_release.file_name, "New Track.wav");
        assert_eq!(renamed_release.relative_path, "01_RELEASE/New Track.wav");
        assert!(app
            .root()
            .join("Singles/New Track/01_RELEASE/New Track.wav")
            .is_file());
        assert!(!app
            .root()
            .join("Singles/New Track/01_RELEASE/Old Track.wav")
            .exists());
    }

    #[test]
    fn track_title_release_collision_rolls_back_folder_file_and_metadata() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Old Conflict".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let source = directory.path().join("release.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVEmanaged release").expect("release source");
        app.import_evidence_from(&created.id, EvidenceRole::ReleaseWav, &source)
            .expect("release import");
        let old_root = app.root().join(&created.relative_path);
        let occupied = old_root.join("01_RELEASE/New Conflict.wav");
        fs::write(&occupied, b"unmanaged collision sentinel").expect("collision sentinel");

        let error = app
            .update_track(
                &created.id,
                TrackPatch {
                    title: Some("New Conflict".into()),
                    ..TrackPatch::default()
                },
            )
            .expect_err("occupied release target must reject title change");
        assert!(matches!(error, AppError::Collision(_)));
        assert!(old_root.is_dir());
        assert!(!app.root().join("Singles/New Conflict").exists());
        assert_eq!(
            fs::read(old_root.join("01_RELEASE/Old Conflict.wav"))
                .expect("managed release after rollback"),
            b"RIFF\x08\0\0\0WAVEmanaged release"
        );
        assert_eq!(
            fs::read(&occupied).expect("collision sentinel after rollback"),
            b"unmanaged collision sentinel"
        );
        let unchanged = app.load_track(&created.id).expect("unchanged track");
        assert_eq!(unchanged.title, "Old Conflict");
        let release = unchanged
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");
        assert_eq!(release.file_name, "Old Conflict.wav");
        assert_eq!(release.relative_path, "01_RELEASE/Old Conflict.wav");
    }

    #[test]
    fn library_move_rolls_back_when_the_database_rejects_the_new_path() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let moving = app
            .create_track(CreateTrackInput {
                title: "First".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("moving track");
        let collision = app
            .create_track(CreateTrackInput {
                title: "Second".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("collision track");
        let collision_root = app.root().join(&collision.relative_path);
        fs::remove_file(collision_root.join(TRACK_IDENTITY_FILE)).expect("remove test identity");
        let mut collision_record = app
            .persistence
            .track(&collision.id)
            .expect("collision record");
        collision_record.relative_path = "Rollback Album/First".into();
        collision_record.library = TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some("Rollback Album".into()),
        };
        app.persistence
            .save_track(&collision_record)
            .expect("stale collision record");

        let error = app
            .update_track_library(
                &moving.id,
                TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("Rollback Album".into()),
                },
            )
            .expect_err("database uniqueness must reject target path");

        assert!(matches!(error, AppError::Database(_)));
        assert!(app.root().join("Singles/First").is_dir());
        assert!(!app.root().join("Rollback Album/First").exists());
        assert!(!app.root().join("Rollback Album").exists());
        let unchanged = app.persistence.track(&moving.id).expect("unchanged record");
        assert_eq!(unchanged.relative_path, moving.relative_path);
        assert_eq!(unchanged.library, TrackLibraryPlacement::default());
    }

    #[test]
    fn reopen_recovers_an_externally_renamed_album_folder_from_track_identity() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Orbit".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement {
                    section: TrackLibrarySection::Album,
                    album_title: Some("Old Album".into()),
                },
            })
            .expect("track");
        drop(app);
        fs::rename(workspace.join("Old Album"), workspace.join("Renamed Album"))
            .expect("external album rename");

        let reopened = WorkspaceApp::open(&workspace, false).expect("reopen renamed workspace");
        let recovered = reopened.load_track(&created.id).expect("recovered track");
        assert_eq!(recovered.relative_path, "Renamed Album/Orbit");
        assert_eq!(
            recovered.library.album_title.as_deref(),
            Some("Renamed Album")
        );
    }

    #[test]
    fn reopen_repairs_the_reported_legacy_missing_path_from_its_album_folder() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let old_root = workspace.join("Neuer Ordner");
        fs::create_dir_all(old_root.join("01_RELEASE")).expect("legacy track");
        app.scan_workspace().expect("index legacy track");
        let mut record = app
            .persistence
            .track_by_relative_path("Neuer Ordner")
            .expect("lookup")
            .expect("legacy record");
        record.library = TrackLibraryPlacement {
            section: TrackLibrarySection::Album,
            album_title: Some("Gravity Drift".into()),
        };
        app.persistence
            .save_track(&record)
            .expect("album assignment");
        drop(app);
        fs::create_dir(workspace.join("Gravity Drift")).expect("album folder");
        fs::rename(&old_root, workspace.join("Gravity Drift/Gravaty"))
            .expect("external track move");

        let reopened = WorkspaceApp::open(&workspace, false).expect("repaired workspace");
        let recovered = reopened
            .load_track(&record.id)
            .expect("recovered legacy track");
        assert_eq!(recovered.relative_path, "Gravity Drift/Gravaty");
        assert_eq!(recovered.title, "Gravaty");
    }

    #[test]
    fn native_validation_rejects_invalid_enums_dates_and_urls() {
        let mut fields = crate::model::TrackFields {
            artwork_origin: "adversarial".into(),
            ..Default::default()
        };
        assert!(validate_track_fields(&fields).is_err());
        fields.artwork_origin = "none".into();
        fields.production_start_date = "2026-08-31".into();
        fields.production_end_date = "2026-08-01".into();
        assert!(validate_track_fields(&fields).is_err());
        fields.production_end_date = "2026-09-01".into();
        fields.suno_project_url = "javascript:alert(1)".into();
        assert!(validate_track_fields(&fields).is_err());
    }

    #[test]
    fn authoritative_release_suno_and_artwork_roles_are_singular() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Singular Assets".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let fixtures = directory.path().join("fixtures");
        fs::create_dir(&fixtures).expect("fixtures");
        let first_wav = fixtures.join("first.wav");
        let second_wav = fixtures.join("second.wav");
        fs::write(&first_wav, b"RIFF\x08\0\0\0WAVEfirst release").expect("first wav");
        fs::write(&second_wav, b"RIFF\x08\0\0\0WAVEsecond release").expect("second wav");
        app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &first_wav)
            .expect("first release import");
        assert!(matches!(
            app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &second_wav),
            Err(AppError::Validation(_))
        ));
        let current_release = app
            .load_track(&track.id)
            .expect("current release")
            .evidence
            .into_iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");
        let replaced_release = app
            .replace_evidence_from(
                &track.id,
                &current_release.id,
                EvidenceRole::ReleaseWav,
                &second_wav,
            )
            .expect("explicit release replacement");
        let active_release = replaced_release
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("active replacement");
        assert_eq!(active_release.id, current_release.id);
        assert_eq!(active_release.file_name, "Singular Assets.wav");
        assert!(app
            .root()
            .join(&replaced_release.relative_path)
            .join(".archive/evidence-replacements")
            .is_dir());

        app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_wav)
            .expect("first Suno export import");
        assert!(matches!(
            app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &second_wav),
            Err(AppError::Validation(_))
        ));

        let first_art = fixtures.join("first.png");
        let second_art = fixtures.join("second.jpeg");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([20, 30, 40, 255]))
            .save(&first_art)
            .expect("first art");
        image::RgbImage::from_pixel(64, 64, image::Rgb([40, 30, 20]))
            .save(&second_art)
            .expect("second art");
        app.import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &first_art)
            .expect("first artwork import");
        assert!(matches!(
            app.import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &second_art),
            Err(AppError::Validation(_))
        ));
        let current = app.load_track(&track.id).expect("current track");
        assert_eq!(
            current
                .evidence
                .iter()
                .filter(|item| item.role == EvidenceRole::ReleaseWav)
                .count(),
            1
        );
        assert_eq!(
            current
                .evidence
                .iter()
                .filter(|item| item.role == EvidenceRole::SunoFinalExport)
                .count(),
            1
        );
        assert_eq!(
            current
                .evidence
                .iter()
                .filter(|item| item.role == EvidenceRole::FinalArtwork)
                .count(),
            1
        );
    }

    #[test]
    fn release_import_never_overwrites_an_existing_track_title_target() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Collision Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let target = app
            .root()
            .join(&track.relative_path)
            .join("01_RELEASE/Collision Track.wav");
        fs::write(&target, b"existing bytes").expect("collision sentinel");
        let source = directory.path().join("incoming.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVEincoming").expect("release source");

        assert!(matches!(
            app.import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source),
            Err(AppError::Collision(_))
        ));
        assert_eq!(
            fs::read(&target).expect("preserved target"),
            b"existing bytes"
        );
        assert!(app
            .persistence
            .evidence(&track.id)
            .expect("evidence list")
            .is_empty());
    }

    #[test]
    fn unfinalized_legacy_managed_release_name_migrates_but_finalized_snapshot_does_not() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Legacy Managed".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let source = directory.path().join("source.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVElegacy managed").expect("release source");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source)
            .expect("release import");
        let mut item = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence")
            .clone();
        let root = app.root().join(&track.relative_path);
        fs::rename(
            root.join(&item.relative_path),
            root.join("01_RELEASE/suno_final_export.wav"),
        )
        .expect("simulate historical managed name");
        item.file_name = "suno_final_export.wav".into();
        item.relative_path = "01_RELEASE/suno_final_export.wav".into();
        app.persistence
            .save_evidence(&track.id, &item)
            .expect("historical evidence metadata");

        let migrated = app.load_track(&track.id).expect("load and migrate");
        let migrated_item = migrated
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("migrated evidence");
        assert_eq!(migrated_item.relative_path, "01_RELEASE/Legacy Managed.wav");
        assert!(root.join("01_RELEASE/Legacy Managed.wav").is_file());

        fs::rename(
            root.join("01_RELEASE/Legacy Managed.wav"),
            root.join("01_RELEASE/suno_final_export.wav"),
        )
        .expect("restore historical name");
        let mut finalized_item = migrated_item.clone();
        finalized_item.file_name = "suno_final_export.wav".into();
        finalized_item.relative_path = "01_RELEASE/suno_final_export.wav".into();
        app.persistence
            .save_evidence(&track.id, &finalized_item)
            .expect("finalized historical metadata");
        let mut record = app.persistence.track(&track.id).expect("track record");
        record.status = TrackStatus::Finalized;
        app.persistence
            .save_track(&record)
            .expect("finalized record");

        assert!(!app
            .migrate_legacy_release_evidence(&record)
            .expect("finalized migration check"));
        assert!(root.join("01_RELEASE/suno_final_export.wav").is_file());
        assert!(!root.join("01_RELEASE/Legacy Managed.wav").exists());
        assert_eq!(
            app.persistence
                .evidence(&track.id)
                .expect("stored finalized evidence")[0]
                .relative_path,
            "01_RELEASE/suno_final_export.wav"
        );
    }

    #[test]
    fn legacy_release_migration_leaves_an_occupied_title_target_unchanged() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Migration Collision".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let source = directory.path().join("source.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVElegacy managed").expect("release source");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &source)
            .expect("release import");
        let mut item = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence")
            .clone();
        let root = app.root().join(&track.relative_path);
        fs::rename(
            root.join(&item.relative_path),
            root.join("01_RELEASE/suno_final_export.wav"),
        )
        .expect("simulate historical managed name");
        item.file_name = "suno_final_export.wav".into();
        item.relative_path = "01_RELEASE/suno_final_export.wav".into();
        app.persistence
            .save_evidence(&track.id, &item)
            .expect("historical evidence metadata");
        let occupied = root.join("01_RELEASE/Migration Collision.wav");
        fs::write(&occupied, b"occupied title target").expect("occupied target");

        let loaded = app.load_track(&track.id).expect("load without migration");
        let unchanged = loaded
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("unchanged evidence");
        assert_eq!(unchanged.file_name, "suno_final_export.wav");
        assert_eq!(unchanged.relative_path, "01_RELEASE/suno_final_export.wav");
        assert!(root.join("01_RELEASE/suno_final_export.wav").is_file());
        assert_eq!(
            fs::read(occupied).expect("preserved occupied target"),
            b"occupied title target"
        );
    }

    #[test]
    fn evidence_preview_embeds_images_and_source_text_but_does_not_load_zip_archives() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Evidence Preview".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let fixtures = directory.path().join("fixtures");
        fs::create_dir(&fixtures).expect("fixtures");
        let screenshot = fixtures.join("screenshot.png");
        image::RgbaImage::from_pixel(32, 32, image::Rgba([12, 24, 48, 255]))
            .save(&screenshot)
            .expect("screenshot fixture");
        let project = fixtures.join("project.zip");
        fs::write(&project, b"PK\x03\x04project fixture").expect("ZIP fixture");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoScreenshot, &screenshot)
            .expect("screenshot import");
        let screenshot_item = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SunoScreenshot)
            .expect("screenshot evidence");
        let image_preview = app
            .preview_evidence(&track.id, &screenshot_item.id)
            .expect("image preview");
        assert!(image_preview
            .data_url
            .as_deref()
            .is_some_and(|value| value.starts_with("data:image/png;base64,")));

        let source_code = fixtures.join("generator.py");
        fs::write(&source_code, b"def generate():\n    return 'sound'\n")
            .expect("source-code fixture");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SourceCodeFile, &source_code)
            .expect("source-code import");
        let source_code_item = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SourceCodeFile)
            .expect("source-code evidence");
        let source_code_preview = app
            .preview_evidence(&track.id, &source_code_item.id)
            .expect("source-code preview");
        assert_eq!(
            source_code_preview.text_content.as_deref(),
            Some("def generate():\n    return 'sound'\n")
        );

        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoProjectZip, &project)
            .expect("ZIP import");
        let project_item = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SunoProjectZip)
            .expect("ZIP evidence");
        let zip_preview = app
            .preview_evidence(&track.id, &project_item.id)
            .expect("ZIP preview metadata");
        assert!(zip_preview.data_url.is_none());
        assert!(zip_preview.text_content.is_none());
        assert!(zip_preview
            .message
            .as_deref()
            .is_some_and(|message| message.contains("nicht entpackt")));

        let replacement_root = directory.path().join("replacement");
        fs::create_dir(&replacement_root).expect("replacement root");
        let replacement_project = replacement_root.join("project.zip");
        fs::write(&replacement_project, b"PK\x03\x04replacement project")
            .expect("replacement ZIP fixture");
        let replaced = app
            .replace_evidence_from(
                &track.id,
                &project_item.id,
                EvidenceRole::SunoProjectZip,
                &replacement_project,
            )
            .expect("same-path database replacement");
        let projects = replaced
            .evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::SunoProjectZip)
            .collect::<Vec<_>>();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, project_item.id);
        assert_eq!(projects[0].relative_path, project_item.relative_path);
    }

    #[test]
    fn track_cover_uses_a_bounded_centered_final_artwork_thumbnail() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Centered Cover".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        assert!(app.track_cover(&track.id).expect("empty cover").is_none());

        let fixture = directory.path().join("wide-final.png");
        image::RgbaImage::from_fn(600, 200, |x, _| {
            if x < 200 {
                image::Rgba([180, 30, 40, 255])
            } else if x < 400 {
                image::Rgba([30, 180, 70, 255])
            } else {
                image::Rgba([30, 60, 180, 255])
            }
        })
        .save(&fixture)
        .expect("wide final-artwork fixture");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::FinalArtwork, &fixture)
            .expect("final artwork import");
        let evidence_id = imported.cover_evidence_id.expect("cover evidence ID");
        assert_eq!(
            app.list_tracks()
                .expect("track summaries")
                .into_iter()
                .find(|item| item.id == track.id)
                .and_then(|item| item.cover_evidence_id),
            Some(evidence_id.clone())
        );

        let preview = app
            .track_cover(&track.id)
            .expect("track cover")
            .expect("present track cover");
        assert_eq!(preview.evidence_id, evidence_id);
        let encoded = preview
            .data_url
            .strip_prefix("data:image/png;base64,")
            .expect("PNG data URL");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("base64 thumbnail");
        let thumbnail = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .expect("decode thumbnail")
            .to_rgba8();
        assert_eq!(
            thumbnail.dimensions(),
            (
                crate::artwork::COVER_PREVIEW_SIZE,
                crate::artwork::COVER_PREVIEW_SIZE
            )
        );
        assert_eq!(
            *thumbnail.get_pixel(96, 96),
            image::Rgba([30, 180, 70, 255])
        );
    }

    #[test]
    fn track_creation_rejects_path_like_titles_without_writing_folders() {
        let directory = tempdir().expect("tempdir");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        for title in ["../escape", "/absolute", r"..\\escape", ".draft"] {
            assert!(app
                .create_track(CreateTrackInput {
                    title: title.into(),
                    production_start_date: "2026-08-01".into(),
                    commercial_use_intended: false,
                    library: TrackLibraryPlacement::default(),
                })
                .is_err());
        }
        assert!(!directory.path().join("escape").exists());
        assert!(!workspace.join("absolute").exists());
    }

    #[test]
    fn track_update_rejects_path_like_titles_without_changing_stored_identity() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Stable Track Title".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track creation");
        for title in ["../x", r"a\b", "...", ".draft", "🚀"] {
            assert!(matches!(
                app.update_track(
                    &track.id,
                    TrackPatch {
                        title: Some(title.into()),
                        ..TrackPatch::default()
                    },
                ),
                Err(AppError::Validation(_))
            ));
            let unchanged = app.load_track(&track.id).expect("unchanged track");
            assert_eq!(unchanged.title, "Stable Track Title");
            assert_eq!(unchanged.relative_path, track.relative_path);
        }
    }

    #[test]
    fn finalized_track_rejects_mutation_until_revision_archives_snapshot() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let detail = app
            .create_track(CreateTrackInput {
                title: "Finalized Track".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track");
        let track_root = app.root().join(&detail.relative_path);
        fs::write(
            track_root.join("01_RELEASE/revision-fixture.wav"),
            b"revision fixture",
        )
        .expect("hashable revision fixture");
        integrity::calculate(&track_root).expect("valid main hash fixture");
        fs::write(
            track_root.join(certificate::CERTIFICATE_FILE),
            b"certificate",
        )
        .expect("certificate fixture");
        fs::write(track_root.join(certificate::MANIFEST_FILE), b"{}\n").expect("manifest fixture");
        let mut pdf_document = printpdf::PdfDocument::new("Revision fixture");
        let pdf_bytes = pdf_document
            .with_pages(vec![printpdf::PdfPage::new(
                printpdf::Mm(210.0),
                printpdf::Mm(297.0),
                Vec::new(),
            )])
            .save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
        fs::write(track_root.join(certificate::PDF_FILE), &pdf_bytes).expect("PDF fixture");
        let certificate_hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
            sha256_file(&track_root.join(integrity::HASH_FILE)).expect("main hash digest"),
            integrity::HASH_FILE,
            sha256_file(&track_root.join(certificate::MANIFEST_FILE)).expect("manifest digest"),
            certificate::MANIFEST_FILE,
            sha256_file(&track_root.join(certificate::CERTIFICATE_FILE))
                .expect("certificate digest"),
            certificate::CERTIFICATE_FILE,
            sha256_file(&track_root.join(certificate::PDF_FILE)).expect("PDF digest"),
            certificate::PDF_FILE,
        );
        fs::write(
            track_root.join(certificate::CERTIFICATE_HASH_FILE),
            certificate_hashes,
        )
        .expect("certificate hashes fixture");
        let mut record = app.persistence.track(&detail.id).expect("stored track");
        record.status = TrackStatus::Finalized;
        record.certificate = CertificateState {
            valid: true,
            certificate_id: Some("SDM-test".into()),
            finalization_snapshot_id: None,
            finalized_at: Some("2026-08-13T12:00:00Z".into()),
            workflow_version: Some(record.workflow_version.clone()),
            certificate_language: CertificateLanguage::En,
            bilingual: false,
            invalidated_at: None,
            invalidation_reason: None,
        };
        app.persistence
            .save_track(&record)
            .expect("finalized state");

        let locked = app
            .update_track(
                &detail.id,
                TrackPatch {
                    release_notes: Some("must not be written".into()),
                    ..TrackPatch::default()
                },
            )
            .expect_err("finalized mutation must be refused");
        assert!(matches!(locked, AppError::Finalized));

        fs::remove_dir(track_root.join(".archive/revisions"))
            .expect("simulate an older track without the managed revision parent");
        let revision = app.create_revision(&detail.id).expect("new revision");
        let revised = revision.track.expect("revised track detail");
        assert_eq!(revised.status, TrackStatus::Active);
        assert!(!revised.certificate.valid);
        for relative in [
            certificate::CERTIFICATE_FILE,
            certificate::MANIFEST_FILE,
            certificate::CERTIFICATE_HASH_FILE,
            certificate::PDF_FILE,
        ] {
            assert!(!track_root.join(relative).exists(), "live {relative}");
        }
        let revision_directories = fs::read_dir(track_root.join(".archive/revisions"))
            .expect("revision archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("revision entries");
        assert_eq!(revision_directories.len(), 1);
        let archive = revision_directories[0].path();
        assert!(archive.join("revision.json").is_file(), "revision metadata");
        for expected in [
            "DOCUMENTATION_CERTIFICATE.md",
            "EVIDENCE_MANIFEST.json",
            "CERTIFICATE_SHA256.txt",
        ] {
            assert!(
                archive.join("certificate").join(expected).is_file(),
                "archived {expected}"
            );
        }
        assert!(archive.join(certificate::PDF_FILE).is_file());
        assert!(archive.join(integrity::HASH_FILE).is_file());

        let mutable = app
            .update_track(
                &detail.id,
                TrackPatch {
                    release_notes: Some("revision change".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("mutation after revision");
        assert_eq!(mutable.fields.release_notes, "revision change");
    }

    #[test]
    fn end_to_end_documentation_workflow_creates_portable_certificate() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "End To End".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: true,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track creation");
        let updated = app
            .update_track(
                &created.id,
                TrackPatch {
                    production_end_date: Some("2026-08-02".into()),
                    suno_model: Some("v4.5".into()),
                    suno_project_url: Some("https://suno.com/song/test-track".into()),
                    suno_project_version_id: Some("project-end-to-end-v1".into()),
                    suno_final_generation_id: Some("generation-end-to-end".into()),
                    suno_final_generation_time: Some("14:35".into()),
                    suno_plan_at_generation: Some("Pro".into()),
                    final_export_date: Some("2026-08-03".into()),
                    instrumental_track: Some(true),
                    vocal_lyrics_present: Some(false),
                    suno_lyrics_field_content: Some(true),
                    suno_lyrics_content_types: Some(vec![
                        SunoLyricsContentType::StructureInstructions,
                        SunoLyricsContentType::SoundInstructions,
                        SunoLyricsContentType::ArrangementInstructions,
                    ]),
                    suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
                    suno_lyrics_field_text: Some(
                        "[Intro]\n[sidechained synth pad]\n[Drop]\n[Outro]".into(),
                    ),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(true),
                    code_audio_post_processed: Some(true),
                    code_audio_post_processing_operations: Some(vec![
                        "EQ".into(),
                        "Other post-processing".into(),
                    ]),
                    code_audio_post_processing_note: Some(
                        "Rendered Sonic Pi layer was level-adjusted before use.".into(),
                    ),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(true),
                    post_export_editing_details: Some(
                        "Final level adjustment and metadata preparation.".into(),
                    ),
                    commercial_use_intended: Some(true),
                    generative_ai_used: Some(true),
                    audio_ai_system: Some("Suno".into()),
                    ai_assisted_audio_elements: Some(DocumentationAnswer::Yes),
                    ai_generated_audio_elements: Some(DocumentationAnswer::Yes),
                    real_person_voice_intentionally_imitated: Some(DocumentationAnswer::No),
                    real_person_identity_intentionally_represented: Some(DocumentationAnswer::No),
                    real_event_represented_as_authentic_recording: Some(DocumentationAnswer::No),
                    real_location_institution_event_presented_as_authentic_ai_recording: Some(
                        DocumentationAnswer::NotDocumented,
                    ),
                    audio_disclosure_applied: Some(DocumentationAnswer::Yes),
                    audio_disclosure_locations: Some(vec![
                        "release metadata".into(),
                        "description".into(),
                    ]),
                    audio_disclosure_text: Some(
                        "AI-generated and AI-assisted audio elements documented with SunoDM."
                            .into(),
                    ),
                    artwork_origin: Some("ai_assisted".into()),
                    ai_image_service: Some("Local Tool".into()),
                    human_artwork_modifications: Some(vec![
                        "Visible disclosure added locally".into()
                    ]),
                    depicts_real_person: Some(true),
                    real_person_notes: Some("Documented real-person depiction".into()),
                    depicts_real_event: Some(false),
                    contains_trademark: Some(false),
                    ..TrackPatch::default()
                },
            )
            .expect("track facts");
        let fixture_root = directory.path().join("fixtures");
        fs::create_dir(&fixture_root).expect("fixture directory");
        let suno_export = fixture_root.join("suno-export.wav");
        let release_master = fixture_root.join("release-master.wav");
        let ai_original = fixture_root.join("ai-original.png");
        let release_mp3 = fixture_root.join("release-master.mp3");
        let source_code = fixture_root.join("sonic-pi-generator.rb");
        let code_audio = fixture_root.join("sonic-pi-render.wav");
        let subscription_one = fixture_root.join("subscription-one.pdf");
        let subscription_two = fixture_root.join("subscription-two.pdf");
        let terms_source = fixture_root.join("suno-terms.pdf");
        let final_audio = p0_screening_wav(Some(&p0_suno_comment("2026-08-02T06:38:06Z")), 31);
        fs::write(&suno_export, &final_audio).expect("Suno fixture");
        fs::write(&release_master, &final_audio).expect("byte-identical release fixture");
        fs::write(&release_mp3, b"ID3\x04\0\0release mp3 fixture").expect("release MP3 fixture");
        fs::write(
            &source_code,
            b"use_bpm 110\nlive_loop :documented_layer do\n  play 48\n  sleep 1\nend\n",
        )
        .expect("Sonic Pi source fixture");
        fs::write(&code_audio, b"RIFF\x08\0\0\0WAVEsonic pi rendered layer")
            .expect("code-generated audio fixture");
        image::RgbaImage::from_pixel(640, 640, image::Rgba([24, 48, 96, 255]))
            .save(&ai_original)
            .expect("AI artwork fixture");
        fs::write(
            &subscription_one,
            b"%PDF-1.7\n1 0 obj\n<</Type /Receipt>>\nendobj\n%%EOF\n",
        )
        .expect("first subscription fixture");
        fs::write(
            &subscription_two,
            b"%PDF-1.7\n1 0 obj\n<</Type /Receipt /Period 2>>\nendobj\n%%EOF\n",
        )
        .expect("second subscription fixture");
        fs::write(
            &terms_source,
            b"%PDF-1.7\n1 0 obj\n<</Type /Terms>>\nendobj\n%%EOF\n",
        )
        .expect("terms fixture");
        app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
            .expect("Suno evidence import");
        let automated = app
            .import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
            .expect("release evidence import");
        assert_eq!(automated.fields.suno_final_generation_date, "2026-08-02");
        assert_eq!(
            automated.fields.suno_final_generation_id,
            "generation-end-to-end"
        );
        assert_eq!(automated.fields.production_end_date, "2026-08-02");
        assert_eq!(automated.fields.suno_download_export_date, "2026-08-02");
        assert_eq!(
            automated.automation.final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(
            automated.automation.final_generation_id_origin,
            FactOrigin::UserConfirmedFact
        );
        assert!(automated.automation.release_identical_to_suno_export);
        app.update_track(
            &updated.id,
            TrackPatch {
                release_filename_difference_confirmed: Some(true),
                suno_export_filename_difference_confirmed: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("explicit filename difference confirmations");
        app.import_evidence_from(&updated.id, EvidenceRole::AiArtworkOriginal, &ai_original)
            .expect("AI original import");
        app.import_evidence_from(&updated.id, EvidenceRole::ReleaseMp3, &release_mp3)
            .expect("release MP3 import");
        app.import_evidence_from(&updated.id, EvidenceRole::SourceCodeFile, &source_code)
            .expect("Sonic Pi source-code import");
        app.import_evidence_from(
            &updated.id,
            EvidenceRole::CodeGeneratedAudioFile,
            &code_audio,
        )
        .expect("code-generated audio import");
        let disclosed = app
            .generate_artwork_disclosure(&updated.id, Some("AI-assisted".into()))
            .expect("local artwork disclosure")
            .track
            .expect("disclosed track");
        assert!(disclosed
            .evidence
            .iter()
            .any(|item| item.role == EvidenceRole::AiArtworkEdited && item.verified));
        let repeated = app
            .generate_artwork_disclosure(&updated.id, Some("AI-assisted".into()))
            .expect("idempotent disclosure request");
        assert!(repeated.message.contains("bereits vorhanden"));
        assert_eq!(
            repeated
                .track
                .expect("repeated disclosure track")
                .evidence
                .iter()
                .filter(|item| item.role == EvidenceRole::AiArtworkEdited)
                .count(),
            1
        );
        let disclosed_artwork = disclosed
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::AiArtworkEdited)
            .expect("locally disclosed artwork");
        app.import_evidence_from(
            &updated.id,
            EvidenceRole::FinalArtwork,
            &app.root()
                .join(&disclosed.relative_path)
                .join(&disclosed_artwork.relative_path),
        )
        .expect("final artwork import from disclosed bytes");
        let global_one = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &subscription_one,
                Some("2026-07-01".into()),
                Some("2026-08-01".into()),
            )
            .expect("first global subscription evidence");
        let global_two = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &subscription_two,
                Some("2026-08-02".into()),
                Some("2026-08-31".into()),
            )
            .expect("second global subscription evidence");
        app.attach_global_evidence(&updated.id, &global_one.evidence.id)
            .expect("first portable subscription copy");
        let portable = app
            .attach_global_evidence(&updated.id, &global_two.evidence.id)
            .expect("second portable subscription copy");
        assert_eq!(
            portable
                .evidence
                .iter()
                .filter(|item| { item.role == EvidenceRole::SubscriptionPayment })
                .count(),
            2
        );
        let terms = app
            .register_global_terms_evidence(
                &terms_source,
                EvidenceMetadata {
                    document_title: "Suno Terms of Service".into(),
                    provider: "Suno, Inc.".into(),
                    source_url: "https://suno.com/terms".into(),
                    retrieval_date: "2026-08-02".into(),
                    effective_date: "2026-07-01".into(),
                    applicable_production_period: "2026-08-01 to 2026-08-02".into(),
                    factual_note: "Offline archived terms fixture.".into(),
                    ..EvidenceMetadata::default()
                },
            )
            .expect("terms evidence with core metadata");
        assert!(app
            .load_track(&updated.id)
            .expect("track with terms")
            .evidence
            .iter()
            .any(|item| {
                item.role == EvidenceRole::SunoTermsRights
                    && item.source_global_evidence_id.as_deref() == Some(terms.evidence.id.as_str())
            }));

        let generated = app
            .generate_documents(&updated.id, false)
            .expect("document generation")
            .track
            .expect("generated track detail");
        assert!(
            generated.documents.current,
            "documents were stale immediately after generation: {:?}",
            generated.documents
        );
        app.calculate_hashes(&updated.id)
            .expect("SHA-256 generation and verification");
        let validation = app.validate_track(&updated.id).expect("native gate");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
        let finalized = app.finalize_track(&updated.id).expect("finalization");
        let final_detail = finalized.track.expect("finalized track detail");
        assert_eq!(final_detail.status, TrackStatus::Finalized);
        assert!(final_detail.certificate.valid);

        let track_root = app.root().join(&final_detail.relative_path);
        for relative in [
            certificate::CERTIFICATE_FILE,
            certificate::MANIFEST_FILE,
            certificate::CERTIFICATE_HASH_FILE,
            certificate::PDF_FILE,
        ] {
            assert!(track_root.join(relative).is_file(), "generated {relative}");
        }
        certificate::verify(&track_root).expect("certificate integrity");

        // Re-render the exact persisted snapshot in an independent root. This
        // exercises determinism across the manifest, Markdown, PDF, and their
        // certificate hash set rather than checking each renderer in isolation.
        let original_manifest_bytes =
            fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("manifest snapshot");
        let original_manifest_json: serde_json::Value =
            serde_json::from_slice(&original_manifest_bytes).expect("manifest snapshot JSON");
        let certificate_steps: Vec<StepState> =
            serde_json::from_value(original_manifest_json["steps"].clone())
                .expect("certificate workflow snapshot");
        let persisted_snapshot = app
            .persistence
            .track(&updated.id)
            .expect("persisted finalized snapshot");
        let persisted_evidence = app
            .persistence
            .evidence(&updated.id)
            .expect("persisted finalized evidence");
        let persisted_deviations = app
            .persistence
            .deviations(&updated.id)
            .expect("persisted finalized deviations");
        let reproduction_root = directory.path().join("certificate-reproduction");
        fs::create_dir_all(reproduction_root.join("03_DOCUMENTATION"))
            .expect("reproduction documentation directory");
        fs::copy(
            track_root.join(integrity::HASH_FILE),
            reproduction_root.join(integrity::HASH_FILE),
        )
        .expect("reproduction SHA256SUMS");
        certificate::generate(
            &reproduction_root,
            &persisted_snapshot,
            &persisted_snapshot.profile_snapshot,
            &certificate_steps,
            &persisted_evidence,
            &persisted_deviations,
            persisted_snapshot
                .certificate
                .certificate_id
                .as_deref()
                .expect("persisted certificate ID"),
            persisted_snapshot
                .certificate
                .finalized_at
                .as_deref()
                .expect("persisted finalization timestamp"),
            "normalized-snapshot-reproduction",
            CertificateRenderOptions {
                language: persisted_snapshot.certificate.certificate_language,
                bilingual: persisted_snapshot.certificate.bilingual,
            },
        )
        .expect("re-render identical normalized snapshot");
        for relative in [
            certificate::MANIFEST_FILE,
            certificate::CERTIFICATE_FILE,
            certificate::PDF_FILE,
            certificate::CERTIFICATE_HASH_FILE,
        ] {
            assert_eq!(
                fs::read(track_root.join(relative)).expect("original certificate artifact"),
                fs::read(reproduction_root.join(relative))
                    .expect("reproduced certificate artifact"),
                "same normalized snapshot produced different bytes for {relative}"
            );
        }

        let manifest = String::from_utf8(original_manifest_bytes).expect("manifest text");
        assert!(!manifest.contains(app.root().to_string_lossy().as_ref()));
        assert!(manifest.contains("\"relative_path\": \".\""));
        assert!(manifest.contains("01_RELEASE/End To End.wav"));
        assert!(manifest.contains("02_SUNO/suno-export.wav"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_AI_ORIGINAL.png"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_AI_EDITED.png"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_FINAL.png"));
        assert!(manifest.contains("sourceGlobalEvidenceId"));
        let manifest_json: serde_json::Value =
            serde_json::from_str(&manifest).expect("manifest JSON");
        assert_eq!(manifest_json["schema_version"].as_u64(), Some(5));
        assert_eq!(
            manifest_json["evidence_derived_metadata"]["suno_created_timestamp"].as_str(),
            Some("2026-08-02T06:38:06Z")
        );
        assert_eq!(
            manifest_json["evidence_derived_metadata"]["suno_id"].as_str(),
            Some(P0_SUNO_ID)
        );
        let manifest_suno = manifest_json["evidence"]
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item["role"].as_str() == Some("suno_final_export"))
            })
            .expect("Suno final export manifest record");
        assert_eq!(
            manifest_suno["metadata"]["sunoId"].as_str(),
            Some(P0_SUNO_ID)
        );
        assert_eq!(
            manifest_suno["metadata"]["sunoCreatedTimestamp"].as_str(),
            Some("2026-08-02T06:38:06Z")
        );
        assert!(manifest_suno["sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64));
        assert_eq!(
            manifest_json["system_verification"]["fact_origins"]["final_suno_generation_id"]
                .as_str(),
            Some("user_confirmed_fact")
        );
        assert_eq!(
            manifest_json["system_verification"]["fact_origins"]["final_suno_generation_date"]
                .as_str(),
            Some("evidence_derived_metadata")
        );
        assert_eq!(
            manifest_json["system_verification"]["fact_origins"]["download_export_date"].as_str(),
            Some("evidence_derived_metadata")
        );
        assert_eq!(
            manifest_json["system_verification"]["fact_origins"]["last_editing_date"].as_str(),
            Some("user_confirmed_fact")
        );
        assert_eq!(
            manifest_json["system_verification"]["release_identical_to_suno_export"].as_bool(),
            Some(true)
        );
        assert!(manifest_json["system_verification"]["byte_identical_pairs"]
            .as_array()
            .is_some_and(|pairs| !pairs.is_empty()));

        let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
            .expect("Markdown certificate");
        assert!(markdown.contains("Final generation date [Evidence-derived metadata]: 2026-08-02"));
        assert!(
            markdown.contains("Final generation ID [User-confirmed fact]: generation-end-to-end")
        );
        assert!(markdown.contains("Suno Studio metadata detected: **YES**"));
        assert!(!markdown.contains("Suno project/version ID"));
        assert!(!markdown.contains("project-end-to-end-v1"));
        assert!(markdown.contains("Release identical to Suno final export: **YES**"));
        assert!(markdown.contains("Download/export date [Evidence-derived metadata]: 2026-08-02"));
        assert!(markdown.contains("Last editing date [User-confirmed fact]: 2026-08-03"));
        assert!(!markdown.contains("2026-08-02T06:38:06Z"));
        assert!(!markdown.contains(P0_SUNO_ID));
        assert!(markdown.contains("Suno plan at generation [User-confirmed fact]: Pro"));
        assert!(markdown.contains("Suno Instrumental Mode Selected [User-confirmed fact]: YES"));
        assert!(markdown.contains("Vocal Lyrics Present [User-confirmed fact]: NO"));
        assert!(markdown.contains("Suno Generation Text Field"));
        assert!(markdown.contains("Suno Terms of Service"));
        assert!(markdown.contains("Suno, Inc."));
        assert!(markdown.contains("AI Transparency Assessment"));
        for relative in [
            "03_DOCUMENTATION/README.md",
            "03_DOCUMENTATION/AI_USAGE.md",
            "03_DOCUMENTATION/SHA256SUMS.txt",
        ] {
            assert!(track_root.join(relative).is_file(), "missing {relative}");
        }

        let source_code_records = manifest_json["evidence"]
            .as_array()
            .expect("manifest evidence")
            .iter()
            .filter(|item| item["role"].as_str() == Some("source_code_file"))
            .count();
        let code_audio_records = manifest_json["evidence"]
            .as_array()
            .expect("manifest evidence")
            .iter()
            .filter(|item| item["role"].as_str() == Some("code_generated_audio_file"))
            .count();
        let subscription_records = manifest_json["evidence"]
            .as_array()
            .expect("manifest evidence")
            .iter()
            .filter(|item| item["role"].as_str() == Some("subscription_payment"))
            .count();
        assert_eq!(source_code_records, 1);
        assert_eq!(code_audio_records, 1);
        assert_eq!(subscription_records, 2);

        let manifest_anchor = final_detail
            .finalization_anchors
            .iter()
            .find(|anchor| anchor.artifact == TimestampReferencedArtifact::EvidenceManifest)
            .expect("final manifest anchor")
            .clone();
        let timestamp_source = fixture_root.join("external-timestamp.json");
        fs::write(
            &timestamp_source,
            b"{\"timestamp\":\"2026-08-04T12:00:00Z\"}\n",
        )
        .expect("external timestamp fixture");
        let timestamped = app
            .attach_external_timestamp_from(
                &updated.id,
                &timestamp_source,
                ExternalTimestampInput {
                    provider: "Example Timestamp Provider".into(),
                    timestamp_type: TimestampType::ElectronicTimestamp,
                    timestamp_value: "2026-08-04T12:00:00Z".into(),
                    referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
                    other_referenced_artifact: String::new(),
                    referenced_sha256: manifest_anchor.sha256,
                    external_reference_id: "E2E-TIMESTAMP-1".into(),
                    provider_verification_url: "https://timestamp.example/e2e".into(),
                    note: "Optional post-finalization evidence fixture.".into(),
                },
            )
            .expect("external timestamp attachment");
        assert_eq!(timestamped.external_timestamps.len(), 1);
        assert_eq!(
            timestamped.external_timestamps[0].referenced_hash_match,
            Some(true)
        );
        let revision = app
            .create_revision(&updated.id)
            .expect("archive complete timestamped snapshot as revision")
            .track
            .expect("new revision detail");
        assert_eq!(revision.external_timestamps.len(), 1);
        assert!(revision.external_timestamps[0].integrity_verified);
        assert!(WalkDir::new(track_root.join(".archive/revisions"))
            .into_iter()
            .filter_map(std::result::Result::ok)
            .any(|entry| entry.file_name() == "TIMESTAMP_RECORD.json"));
        if std::env::var_os("SUNODM_KEEP_ACCEPTANCE_FIXTURE").is_some() {
            let retained = directory.keep();
            eprintln!("retained acceptance fixture: {}", retained.display());
        }
    }

    #[test]
    fn finalized_certificate_fields_cross_check_sqlite_track_evidence_hashes_and_manifest() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let created = app
            .create_track(CreateTrackInput {
                title: "Certificate Field Cross Check".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("track creation");
        let updated = app
            .update_track(
                &created.id,
                TrackPatch {
                    production_end_date: Some("2026-08-02".into()),
                    suno_model: Some("v4.5".into()),
                    suno_project_url: Some("https://suno.com/song/certificate-cross-check".into()),
                    suno_project_version_id: Some("project-cross-check-v1".into()),
                    suno_final_generation_id: Some("generation-cross-check".into()),
                    suno_final_generation_date: Some("2026-08-02".into()),
                    suno_final_generation_time: Some("09:10".into()),
                    suno_download_export_date: Some("2026-08-03".into()),
                    suno_plan_at_generation: Some("Pro".into()),
                    final_export_date: Some("2026-08-03".into()),
                    instrumental_track: Some(true),
                    vocal_lyrics_present: Some(false),
                    suno_lyrics_field_content: Some(false),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(false),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(false),
                    commercial_use_intended: Some(false),
                    generative_ai_used: Some(false),
                    artwork_origin: Some("human".into()),
                    depicts_real_person: Some(false),
                    depicts_real_event: Some(false),
                    contains_trademark: Some(false),
                    ..TrackPatch::default()
                },
            )
            .expect("track facts");

        let fixture_root = directory.path().join("fixtures");
        fs::create_dir(&fixture_root).expect("fixture directory");
        let suno_export = fixture_root.join("cross-check-suno.wav");
        let release_master = fixture_root.join("cross-check-release.wav");
        let final_artwork = fixture_root.join("cross-check-final.png");
        fs::write(&suno_export, p0_screening_wav(None, 41)).expect("Suno export fixture");
        fs::write(&release_master, p0_screening_wav(None, 47)).expect("release fixture");
        image::RgbaImage::from_pixel(64, 64, image::Rgba([32, 64, 96, 255]))
            .save(&final_artwork)
            .expect("final artwork fixture");
        app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
            .expect("Suno evidence import");
        app.import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
            .expect("release evidence import");
        app.import_evidence_from(&updated.id, EvidenceRole::FinalArtwork, &final_artwork)
            .expect("final artwork import");
        app.update_track(
            &updated.id,
            TrackPatch {
                release_filename_difference_confirmed: Some(true),
                suno_export_filename_difference_confirmed: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("explicit filename difference confirmations");

        let deviation_detail = app
            .add_deviation(
                &updated.id,
                DeviationInput {
                    description: "Resolved certificate cross-check note".into(),
                    blocking: true,
                },
            )
            .expect("blocking deviation fixture");
        let deviation_id = deviation_detail
            .blocking_deviations
            .iter()
            .find(|deviation| deviation.blocking && !deviation.resolved)
            .expect("unresolved fixture deviation")
            .id
            .clone();
        app.resolve_deviation(&updated.id, &deviation_id)
            .expect("resolved deviation fixture");

        app.generate_documents(&updated.id, false)
            .expect("document generation");
        app.calculate_hashes(&updated.id)
            .expect("SHA256SUMS generation");
        let validation = app.validate_track(&updated.id).expect("native gate");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
        let finalized = app
            .finalize_track(&updated.id)
            .expect("real finalization")
            .track
            .expect("finalized detail");
        assert_eq!(finalized.status, TrackStatus::Finalized);

        let stored_track = app
            .persistence
            .track(&updated.id)
            .expect("SQLite track record");
        let stored_evidence = app
            .persistence
            .evidence(&updated.id)
            .expect("SQLite evidence records");
        let stored_steps = app
            .persistence
            .stored_steps(&updated.id)
            .expect("SQLite step states");
        let stored_deviations = app
            .persistence
            .deviations(&updated.id)
            .expect("SQLite deviations");
        let track_root = app.root().join(&stored_track.relative_path);
        let sums_bytes = fs::read(track_root.join(integrity::HASH_FILE)).expect("SHA256SUMS bytes");
        let sums =
            parse_sha256sums(std::str::from_utf8(&sums_bytes).expect("SHA256SUMS must be UTF-8"));
        let manifest_bytes =
            fs::read(track_root.join(certificate::MANIFEST_FILE)).expect("evidence manifest bytes");
        let manifest: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).expect("evidence manifest JSON");
        let certificate_text = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
            .expect("certificate document");
        let certificate = parse_certificate_document(&certificate_text);
        let pdf_bytes =
            fs::read(track_root.join(certificate::PDF_FILE)).expect("technical PDF bytes");
        let mut pdf_warnings = Vec::new();
        let parsed_pdf = printpdf::PdfDocument::parse(
            &pdf_bytes,
            &printpdf::PdfParseOptions::default(),
            &mut pdf_warnings,
        )
        .expect("parse technical PDF");
        let pdf_text = parsed_pdf
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        let compact_pdf_text = pdf_text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let certificate_hashes = parse_sha256sums(
            &fs::read_to_string(track_root.join(certificate::CERTIFICATE_HASH_FILE))
                .expect("certificate hash set"),
        );

        let certificate_id = stored_track
            .certificate
            .certificate_id
            .as_deref()
            .expect("stored certificate ID");
        let finalized_at = stored_track
            .certificate
            .finalized_at
            .as_deref()
            .expect("stored finalization timestamp");
        assert_eq!(
            finalized.certificate.certificate_id.as_deref(),
            Some(certificate_id)
        );
        assert_eq!(
            finalized.certificate.finalized_at.as_deref(),
            Some(finalized_at)
        );
        assert_eq!(
            manifest_string(&manifest, "/certificate/id"),
            certificate_id
        );
        assert_eq!(certificate.fields["Certificate ID"], certificate_id);
        assert!(compact_pdf_text.contains(certificate_id));
        assert_eq!(
            manifest_string(&manifest, "/certificate/format_version"),
            certificate::CERTIFICATE_FORMAT_VERSION
        );
        assert_eq!(
            certificate.fields["Certificate schema"],
            certificate::CERTIFICATE_FORMAT_VERSION
        );

        assert_eq!(finalized.title, stored_track.fields.title);
        assert_eq!(
            manifest_string(&manifest, "/track/title"),
            stored_track.fields.title
        );
        assert_eq!(
            certificate.fields["Documented title [User-confirmed fact]"],
            stored_track.fields.title
        );
        assert!(pdf_text.contains(&stored_track.fields.title));
        assert_eq!(
            manifest_string(&manifest, "/artist/name"),
            stored_track.profile_snapshot.artist_name
        );
        assert_eq!(
            certificate.fields["Artist [User-confirmed fact]"],
            stored_track.profile_snapshot.artist_name
        );
        assert_eq!(
            manifest_string(&manifest, "/workflow/id"),
            stored_track.workflow_id
        );
        assert_eq!(
            manifest_string(&manifest, "/workflow/version"),
            stored_track.workflow_version
        );
        assert_eq!(
            certificate.fields["Workflow"],
            format!(
                "{} / {}",
                stored_track.workflow_id, stored_track.workflow_version
            )
        );
        assert_eq!(
            manifest_string(&manifest, "/workflow/application_version"),
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            certificate.fields["Application version"],
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(
            manifest_string(&manifest, "/finalization/timestamp"),
            finalized_at
        );
        assert_eq!(certificate.fields["Finalized at"], finalized_at);

        let manifest_evidence = manifest["evidence"]
            .as_array()
            .expect("manifest evidence array");
        let verified_evidence = stored_evidence
            .iter()
            .filter(|item| {
                item.verified && item.sha256.is_some() && item.verification_error.is_none()
            })
            .collect::<Vec<_>>();
        assert_eq!(manifest_evidence.len(), verified_evidence.len());
        assert_eq!(
            certificate.fields["Evidence file count"],
            verified_evidence.len().to_string()
        );
        let manifest_evidence_by_id = manifest_evidence
            .iter()
            .map(|item| {
                let id = item["id"].as_str().expect("manifest evidence ID");
                (id, item)
            })
            .collect::<BTreeMap<_, _>>();
        for evidence in &verified_evidence {
            let manifest_item = manifest_evidence_by_id
                .get(evidence.id.as_str())
                .unwrap_or_else(|| panic!("manifest evidence missing: {}", evidence.id));
            assert_eq!(manifest_item["role"].as_str(), Some(evidence.role.as_str()));
            assert_eq!(
                manifest_item["relativePath"].as_str(),
                Some(evidence.relative_path.as_str())
            );
            assert_eq!(manifest_item["sha256"].as_str(), evidence.sha256.as_deref());
        }

        let release = verified_evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("stored release WAV");
        let artwork = verified_evidence
            .iter()
            .find(|item| item.role == EvidenceRole::FinalArtwork)
            .expect("stored final artwork");
        assert_eq!(
            sums.get(&release.relative_path),
            release.sha256.as_ref(),
            "release WAV hash must agree between SHA256SUMS and SQLite"
        );
        assert_eq!(
            sums.get(&artwork.relative_path),
            artwork.sha256.as_ref(),
            "final artwork hash must agree between SHA256SUMS and SQLite"
        );
        assert_eq!(
            manifest["hashes"]
                .get(&release.relative_path)
                .and_then(serde_json::Value::as_str),
            release.sha256.as_deref()
        );
        assert_eq!(
            manifest["hashes"]
                .get(&artwork.relative_path)
                .and_then(serde_json::Value::as_str),
            artwork.sha256.as_deref()
        );
        assert_eq!(
            certificate.fields["Release audio SHA-256"],
            release.sha256.as_deref().unwrap()
        );
        assert_eq!(
            certificate.fields["Final artwork SHA-256"],
            artwork.sha256.as_deref().unwrap()
        );

        let sums_sha =
            sha256_file(&track_root.join(integrity::HASH_FILE)).expect("SHA256SUMS file hash");
        let manifest_sha = sha256_file(&track_root.join(certificate::MANIFEST_FILE))
            .expect("evidence manifest file hash");
        assert_eq!(
            manifest_string(&manifest, "/certificate/sha256sums_sha256"),
            sums_sha
        );
        assert_eq!(certificate.fields["SHA256SUMS.txt SHA-256"], sums_sha);
        assert_eq!(
            certificate.fields["Evidence manifest SHA-256"],
            manifest_sha
        );
        assert_eq!(certificate_hashes.len(), 4);
        assert_eq!(
            certificate_hashes.get(certificate::PDF_FILE),
            Some(
                &sha256_file(&track_root.join(certificate::PDF_FILE)).expect("technical PDF hash")
            )
        );
        assert!(compact_pdf_text.contains(release.sha256.as_deref().unwrap()));
        assert_eq!(
            manifest["hashes"],
            serde_json::to_value(&sums).expect("serialize parsed SHA256SUMS")
        );

        assert_eq!(
            manifest["deviations"],
            serde_json::to_value(&stored_deviations).unwrap()
        );
        let open_blocking = stored_deviations
            .iter()
            .filter(|deviation| deviation.blocking && !deviation.resolved)
            .count();
        assert_eq!(
            certificate.fields["Blocking deviations"],
            open_blocking.to_string()
        );

        let manifest_steps = manifest["steps"].as_array().expect("manifest steps");
        let manifest_na_reasons = manifest_steps
            .iter()
            .filter(|step| step["status"].as_str() == Some("N_A"))
            .map(|step| {
                (
                    step["id"].as_str().expect("N/A step ID").to_owned(),
                    step["naReason"].as_str().expect("N/A reason").to_owned(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(manifest_na_reasons, certificate.na_reasons);
        assert!(certificate.na_reasons.is_empty());
        assert!(stored_steps
            .iter()
            .all(|step| step.status != StepStatus::NotApplicable));
        let manifest_completed = manifest_steps
            .iter()
            .filter(|step| matches!(step["status"].as_str(), Some("PASS" | "N_A")))
            .map(|step| {
                let id = step["id"].as_str().expect("completed step ID").to_owned();
                let status = match step["status"].as_str().expect("completed status") {
                    "PASS" => "PASS",
                    "N_A" => "N/A",
                    status => panic!("unexpected completed status: {status}"),
                };
                (id, status.to_owned())
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(certificate.completed_steps, manifest_completed);
    }

    #[test]
    fn global_subscription_evidence_requires_pdf_signature_and_relevant_dates() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Commercial Coverage".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: true,
                library: TrackLibraryPlacement::default(),
            })
            .expect("commercial track");
        app.update_track(
            &track.id,
            TrackPatch {
                production_end_date: Some("2026-08-03".into()),
                ..TrackPatch::default()
            },
        )
        .expect("production range");

        let fixtures = directory.path().join("fixtures");
        fs::create_dir(&fixtures).expect("fixture directory");
        let disguised_pdf = fixtures.join("disguised.pdf");
        fs::write(&disguised_pdf, b"plain text with a PDF extension")
            .expect("disguised PDF fixture");
        assert!(matches!(
            app.register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &disguised_pdf,
                Some("2026-08-01".into()),
                Some("2026-08-31".into()),
            ),
            Err(AppError::Validation(_))
        ));

        let narrow_pdf = fixtures.join("narrow-subscription.pdf");
        fs::write(&narrow_pdf, b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n")
            .expect("narrow subscription PDF");
        let narrow = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &narrow_pdf,
                Some("2026-08-02".into()),
                Some("2026-08-31".into()),
            )
            .expect("valid global PDF with narrow coverage");
        app.attach_global_evidence(&track.id, &narrow.evidence.id)
            .expect("partially overlapping subscription may be combined with another receipt");

        let irrelevant_pdf = fixtures.join("irrelevant-subscription.pdf");
        fs::write(&irrelevant_pdf, b"%PDF-1.7\nirrelevant\n%%EOF\n")
            .expect("irrelevant subscription PDF");
        let irrelevant = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &irrelevant_pdf,
                Some("2026-09-01".into()),
                Some("2026-09-30".into()),
            )
            .expect("valid but irrelevant global evidence");
        assert!(matches!(
            app.attach_global_evidence(&track.id, &irrelevant.evidence.id),
            Err(AppError::Validation(_))
        ));

        let covering_pdf = fixtures.join("covering-subscription.pdf");
        let covering_bytes = b"%PDF-1.7\n1 0 obj\n<</Type /Receipt>>\nendobj\n%%EOF\n";
        fs::write(&covering_pdf, covering_bytes).expect("covering subscription PDF");
        let covering = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &covering_pdf,
                Some("2026-07-01".into()),
                Some("2026-08-31".into()),
            )
            .expect("covering global evidence");
        let attached = app
            .attach_global_evidence(&track.id, &covering.evidence.id)
            .expect("portable track copy");
        let portable = attached
            .evidence
            .iter()
            .find(|item| {
                item.source_global_evidence_id.as_deref() == Some(covering.evidence.id.as_str())
            })
            .expect("subscription evidence attached");
        assert!(portable.verified);
        assert_eq!(
            portable.source_global_evidence_id.as_deref(),
            Some(covering.evidence.id.as_str())
        );
        assert_eq!(portable.coverage_start.as_deref(), Some("2026-07-01"));
        assert_eq!(portable.coverage_end.as_deref(), Some("2026-08-31"));
        assert!(portable
            .relative_path
            .starts_with("04_LICENSES/subscription_"));
        assert_eq!(
            fs::read(
                app.root()
                    .join(&attached.relative_path)
                    .join(&portable.relative_path)
            )
            .expect("portable evidence bytes"),
            covering_bytes
        );
        assert_eq!(portable.sha256, covering.evidence.sha256);
    }

    #[test]
    fn externally_deleted_evidence_remains_loadable_and_invalidates_finalized_state() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized =
            finalize_acceptance_track(&app, &directory.path().join("fixtures"), "Deleted Evidence");
        let deleted = finalized
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence")
            .clone();
        let track_root = app.root().join(&finalized.relative_path);
        fs::remove_file(track_root.join(&deleted.relative_path))
            .expect("external evidence deletion");

        let loaded = app
            .load_track(&finalized.id)
            .expect("track remains loadable after external deletion");
        let missing = loaded
            .evidence
            .iter()
            .find(|item| item.id == deleted.id)
            .expect("missing evidence remains indexed");
        assert!(!missing.verified);
        assert_eq!(missing.size_bytes, 0);
        assert_eq!(
            missing.verification_error.as_deref(),
            Some("Evidence file is missing.")
        );
        assert_eq!(loaded.status, TrackStatus::Finalized);
        assert!(!loaded.certificate.valid);
        assert!(loaded.certificate.invalidated_at.is_some());
        assert!(loaded
            .certificate
            .invalidation_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("changed after finalization")));
    }

    #[test]
    fn legacy_scan_is_read_only_and_indexes_evidence_as_historically_unverified() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Session");
        for relative in ["01_RELEASE", "02_SUNO", "03_DOCUMENTATION"] {
            fs::create_dir_all(legacy_root.join(relative)).expect("legacy folder");
        }
        fs::write(
            legacy_root.join("01_RELEASE/legacy-master.wav"),
            b"RIFF\x08\0\0\0WAVElegacy master",
        )
        .expect("legacy release");
        fs::write(
            legacy_root.join("02_SUNO/legacy-export.wav"),
            b"RIFF\x08\0\0\0WAVElegacy Suno export",
        )
        .expect("legacy Suno evidence");
        fs::write(
            legacy_root.join("03_DOCUMENTATION/README.md"),
            b"# Existing legacy notes\n",
        )
        .expect("legacy documentation");
        let before = track_tree_snapshot(&legacy_root);

        let scan = app.scan_workspace().expect("legacy scan");
        assert_eq!(scan.discovered, 1);
        assert_eq!(scan.indexed, 1);
        assert_eq!(scan.unchanged, 0);
        let candidate = scan.candidates.first().expect("legacy candidate");
        assert_eq!(candidate.name, "Legacy Session");
        assert_eq!(candidate.relative_path, "Legacy Session");
        assert_eq!(candidate.status, "NOT_VERIFIED");
        assert!(candidate
            .recognized_folders
            .contains(&"01_RELEASE".to_owned()));
        assert!(candidate
            .documents
            .contains(&"03_DOCUMENTATION/README.md".to_owned()));
        assert!(candidate
            .evidence_files
            .contains(&"01_RELEASE/legacy-master.wav".to_owned()));
        assert!(candidate
            .evidence_files
            .contains(&"02_SUNO/legacy-export.wav".to_owned()));
        assert_eq!(track_tree_snapshot(&legacy_root), before);

        let record = app
            .persistence
            .track_by_relative_path("Legacy Session")
            .expect("legacy index lookup")
            .expect("indexed legacy record");
        assert!(record.legacy);
        assert_eq!(record.fields.title, "Legacy Session");
        let evidence = app
            .persistence
            .evidence(&record.id)
            .expect("indexed legacy evidence");
        assert_eq!(evidence.len(), 2);
        assert!(evidence.iter().all(|item| !item.verified));
        assert!(evidence.iter().all(|item| {
            item.verification_error
                .as_deref()
                .is_some_and(|error| error.contains("not been independently verified"))
        }));

        app.update_profile(complete_profile())
            .expect("complete current profile");
        let adopted = app
            .adopt_legacy_profile(&record.id)
            .expect("explicit legacy profile adoption");
        assert_eq!(adopted.profile_snapshot, complete_profile());

        fs::write(
            legacy_root.join("02_SUNO/later-export.wav"),
            b"RIFF\x08\0\0\0WAVElater legacy evidence",
        )
        .expect("later legacy evidence");
        let before_rescan = track_tree_snapshot(&legacy_root);
        let rescan = app.scan_workspace().expect("idempotent legacy rescan");
        assert_eq!(rescan.indexed, 0);
        assert_eq!(rescan.unchanged, 1);
        assert_eq!(track_tree_snapshot(&legacy_root), before_rescan);
        let reconciled = app
            .persistence
            .evidence(&record.id)
            .expect("reconciled legacy evidence");
        assert_eq!(reconciled.len(), 3);
        assert!(reconciled.iter().all(|item| !item.verified));
    }

    #[test]
    fn reopening_scanned_legacy_track_preserves_existing_certificate_sentinel_in_place() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Certificate Sentinel");
        fs::create_dir_all(legacy_root.join("06_CERTIFICATE"))
            .expect("legacy certificate directory");
        fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy release directory");
        let sentinel = legacy_root.join("06_CERTIFICATE/sentinel.bin");
        let sentinel_bytes = b"historical certificate bytes\0must stay in place";
        fs::write(&sentinel, sentinel_bytes).expect("legacy certificate sentinel");
        fs::write(
            legacy_root.join("01_RELEASE/legacy.wav"),
            b"RIFF\x08\0\0\0WAVElegacy release",
        )
        .expect("legacy release fixture");

        let scan = app.scan_workspace().expect("legacy scan");
        assert_eq!(scan.indexed, 1);
        let indexed = app
            .persistence
            .track_by_relative_path("Legacy Certificate Sentinel")
            .expect("legacy lookup")
            .expect("legacy track indexed");
        assert!(indexed.legacy);
        assert_eq!(
            fs::read(&sentinel).expect("sentinel after scan"),
            sentinel_bytes
        );
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("workspace reopen");
        assert_eq!(
            fs::read(&sentinel).expect("sentinel after reopen"),
            sentinel_bytes
        );
        assert!(!legacy_root.join(".archive/recovery").exists());
        let detail = reopened
            .load_track(&indexed.id)
            .expect("reopened legacy track");
        assert!(detail.legacy.unwrap_or(false));
    }

    #[test]
    fn removing_indexed_legacy_evidence_archives_it_and_rescan_does_not_reindex_it() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Evidence Removal");
        fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy evidence directory");
        let original = legacy_root.join("02_SUNO/legacy-source.wav");
        let original_bytes = b"RIFF\x08\0\0\0WAVErecoverable legacy source";
        fs::write(&original, original_bytes).expect("legacy evidence fixture");
        app.scan_workspace().expect("initial legacy scan");
        let track = app
            .persistence
            .track_by_relative_path("Legacy Evidence Removal")
            .expect("legacy lookup")
            .expect("legacy track indexed");
        let item = app
            .persistence
            .evidence(&track.id)
            .expect("legacy evidence index")
            .into_iter()
            .find(|item| item.relative_path == "02_SUNO/legacy-source.wav")
            .expect("indexed legacy evidence");
        assert_eq!(item.provenance, EvidenceProvenance::IndexedLegacy);

        let removed = app
            .remove_evidence(&track.id, &item.id)
            .expect("archive indexed legacy evidence");
        assert!(!removed
            .evidence
            .iter()
            .any(|evidence| evidence.id == item.id));
        assert!(!original.exists());
        let removal_entries = fs::read_dir(legacy_root.join(".archive/removals"))
            .expect("removal archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("removal entries");
        assert_eq!(removal_entries.len(), 1);
        let removal = removal_entries[0].path();
        assert_eq!(
            fs::read(removal.join("legacy-source.wav")).expect("recoverable archived bytes"),
            original_bytes
        );
        let metadata = fs::read_to_string(removal.join("removal.json")).expect("removal metadata");
        assert!(metadata.contains("02_SUNO/legacy-source.wav"));
        assert!(metadata.contains("indexed_legacy"));

        let rescan = app.scan_workspace().expect("legacy rescan after removal");
        assert_eq!(rescan.indexed, 0);
        assert_eq!(rescan.unchanged, 1);
        assert!(app
            .persistence
            .evidence(&track.id)
            .expect("evidence after rescan")
            .is_empty());
        assert!(!rescan.candidates[0]
            .evidence_files
            .iter()
            .any(|relative| relative == "02_SUNO/legacy-source.wav"));
        assert_eq!(
            fs::read(removal.join("legacy-source.wav"))
                .expect("archive remains recoverable after rescan"),
            original_bytes
        );
    }

    #[test]
    fn managed_track_scan_recovers_unindexed_file_then_archives_and_allows_regular_reimport() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let track = app
            .create_track(CreateTrackInput {
                title: "Managed Crash Recovery".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("managed track");
        assert!(!track.legacy.unwrap_or(true));
        let track_root = workspace.join(&track.relative_path);
        let crash_file = track_root.join("02_SUNO/crash-copy.wav");
        let crash_bytes = b"RIFF\x08\0\0\0WAVEcopied before SQLite commit";
        fs::write(&crash_file, crash_bytes).expect("simulate copy-before-commit crash");
        assert!(app
            .persistence
            .evidence(&track.id)
            .expect("empty evidence index")
            .is_empty());

        let scan = app
            .scan_workspace()
            .expect("managed track source-of-truth scan");
        assert_eq!(scan.indexed, 0);
        assert_eq!(scan.unchanged, 1);
        let recovered = app
            .persistence
            .evidence(&track.id)
            .expect("recovered evidence index");
        assert_eq!(recovered.len(), 1);
        let recovered_item = &recovered[0];
        assert_eq!(recovered_item.relative_path, "02_SUNO/crash-copy.wav");
        assert_eq!(recovered_item.role, EvidenceRole::SunoFinalExport);
        assert_eq!(recovered_item.provenance, EvidenceProvenance::IndexedLegacy);
        assert!(!recovered_item.verified);
        assert!(recovered_item
            .verification_error
            .as_deref()
            .is_some_and(|message| message.contains("Recovered unindexed track evidence")));

        let removed = app
            .remove_evidence(&track.id, &recovered_item.id)
            .expect("archive recovered unindexed evidence");
        assert!(removed.evidence.is_empty());
        assert!(!crash_file.exists());
        let removal_entries = fs::read_dir(track_root.join(".archive/removals"))
            .expect("recovery removals")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("recovery removal entries");
        assert_eq!(removal_entries.len(), 1);
        let removal = removal_entries[0].path();
        assert_eq!(
            fs::read(removal.join("crash-copy.wav")).expect("recoverable crash copy"),
            crash_bytes
        );

        let fixtures = directory.path().join("fixtures");
        fs::create_dir(&fixtures).expect("fixture directory");
        let source = fixtures.join("crash-copy.wav");
        fs::write(&source, b"RIFF\x08\0\0\0WAVEregular reimport")
            .expect("regular reimport fixture");
        let reimported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("regular evidence reimport");
        let managed = reimported
            .evidence
            .iter()
            .find(|item| item.relative_path == "02_SUNO/crash-copy.wav")
            .expect("managed reimport");
        assert_eq!(managed.provenance, EvidenceProvenance::ManagedCopy);
        assert!(managed.verified);
        assert_ne!(managed.id, recovered_item.id);
        assert_eq!(
            fs::read(&crash_file).expect("managed reimport bytes"),
            b"RIFF\x08\0\0\0WAVEregular reimport"
        );
    }

    #[test]
    fn legacy_scan_never_arbitrarily_selects_duplicate_singular_candidates() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Artwork Candidates");
        fs::create_dir_all(legacy_root.join("05_ARTWORK")).expect("legacy artwork directory");
        fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy Suno directory");
        fs::write(
            legacy_root.join("05_ARTWORK/a_final.png"),
            b"\x89PNG\r\n\x1a\nfirst legacy final",
        )
        .expect("first final artwork candidate");
        fs::write(
            legacy_root.join("05_ARTWORK/b_final.png"),
            b"\x89PNG\r\n\x1a\nsecond legacy final",
        )
        .expect("second final artwork candidate");
        fs::write(
            legacy_root.join("02_SUNO/a-export.wav"),
            b"RIFF\x08\0\0\0WAVEfirst legacy Suno candidate",
        )
        .expect("first Suno candidate");
        fs::write(
            legacy_root.join("02_SUNO/b-export.wav"),
            b"RIFF\x08\0\0\0WAVEsecond legacy Suno candidate",
        )
        .expect("second Suno candidate");

        app.scan_workspace().expect("legacy artwork scan");
        let track = app
            .persistence
            .track_by_relative_path("Legacy Artwork Candidates")
            .expect("legacy lookup")
            .expect("legacy artwork track indexed");
        let evidence = app
            .persistence
            .evidence(&track.id)
            .expect("legacy artwork evidence");
        let ambiguous_items = evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::Other)
            .collect::<Vec<_>>();
        assert!(!evidence.iter().any(|item| matches!(
            item.role,
            EvidenceRole::FinalArtwork | EvidenceRole::SunoFinalExport
        )));
        assert_eq!(ambiguous_items.len(), 4);
        assert!(evidence.iter().all(|item| {
            item.provenance == EvidenceProvenance::IndexedLegacy && !item.verified
        }));
        assert!(ambiguous_items.iter().all(|item| item
            .verification_error
            .as_deref()
            .is_some_and(|message| message.contains("ambiguous duplicate"))));
    }

    #[test]
    fn legacy_verification_rejects_disguised_file_types_and_accepts_valid_magic_bytes() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Type Verification");
        fs::create_dir_all(legacy_root.join("01_RELEASE")).expect("legacy release directory");
        fs::create_dir_all(legacy_root.join("02_SUNO")).expect("legacy Suno directory");
        fs::write(
            legacy_root.join("01_RELEASE/final.wav"),
            b"plain text disguised as wave audio",
        )
        .expect("disguised legacy WAV");
        fs::write(
            legacy_root.join("02_SUNO/valid-export.wav"),
            b"RIFF\x08\0\0\0WAVEvalid legacy export",
        )
        .expect("valid legacy WAV");
        app.scan_workspace().expect("legacy type scan");
        let track = app
            .persistence
            .track_by_relative_path("Legacy Type Verification")
            .expect("legacy lookup")
            .expect("legacy track indexed");
        app.update_profile(complete_profile())
            .expect("complete current profile");
        app.adopt_legacy_profile(&track.id)
            .expect("adopt profile for controlled native validation");
        app.update_track(
            &track.id,
            TrackPatch {
                production_start_date: Some("2026-08-01".into()),
                production_end_date: Some("2026-08-02".into()),
                ..TrackPatch::default()
            },
        )
        .expect("legacy production range");
        let indexed = app
            .persistence
            .evidence(&track.id)
            .expect("legacy evidence index");
        let disguised = indexed
            .iter()
            .find(|item| item.relative_path == "01_RELEASE/final.wav")
            .expect("disguised indexed evidence")
            .clone();
        let valid = indexed
            .iter()
            .find(|item| item.relative_path == "02_SUNO/valid-export.wav")
            .expect("valid indexed evidence")
            .clone();
        assert_eq!(disguised.role, EvidenceRole::ReleaseWav);
        assert_eq!(valid.role, EvidenceRole::SunoFinalExport);
        assert!(!disguised.verified);
        assert!(!valid.verified);

        let rejected = app
            .verify_evidence(&track.id, Some(&disguised.id))
            .expect("controlled disguised-type rejection");
        let rejected_item = rejected
            .evidence
            .iter()
            .find(|item| item.id == disguised.id)
            .expect("rejected disguised evidence remains indexed");
        assert!(!rejected_item.verified);
        assert!(rejected_item
            .verification_error
            .as_deref()
            .is_some_and(|message| message.contains("type verification failed")));
        assert!(rejected
            .missing_items
            .iter()
            .any(|item| item.contains("finale Release-Audiodatei")));
        let validation = app
            .validate_track(&track.id)
            .expect("legacy validation remains controlled");
        assert!(!validation.valid);
        assert!(validation
            .missing_items
            .iter()
            .any(|item| item.contains("finale Release-Audiodatei")));

        let accepted = app
            .verify_evidence(&track.id, Some(&valid.id))
            .expect("valid legacy magic-byte verification");
        let accepted_item = app
            .persistence
            .evidence_item(&track.id, &valid.id)
            .expect("stored verified valid legacy evidence");
        assert!(accepted_item.verified);
        assert!(accepted_item.verification_error.is_none());
        assert_eq!(accepted_item.provenance, EvidenceProvenance::IndexedLegacy);
        let disguised_after = app
            .persistence
            .evidence_item(&track.id, &disguised.id)
            .expect("disguised evidence still indexed");
        assert!(!disguised_after.verified);
        assert!(!accepted.missing_items.is_empty());
    }

    #[test]
    fn blocking_deviation_prevents_validation_and_finalization_until_resolved() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Blocking Deviation",
        );
        let with_deviation = app
            .add_deviation(
                &ready.id,
                DeviationInput {
                    description: "Rights review is still open".into(),
                    blocking: true,
                },
            )
            .expect("blocking deviation");
        let deviation = with_deviation
            .blocking_deviations
            .iter()
            .find(|item| item.blocking && !item.resolved)
            .expect("unresolved blocking deviation")
            .clone();

        let blocked = app.validate_track(&ready.id).expect("blocked validation");
        assert!(!blocked.valid);
        assert!(blocked
            .blocking_items
            .iter()
            .any(|item| item.contains("Rights review is still open")));
        assert!(matches!(
            app.finalize_track(&ready.id),
            Err(AppError::Validation(_))
        ));
        assert!(!app
            .root()
            .join(&ready.relative_path)
            .join(certificate::CERTIFICATE_FILE)
            .exists());

        let resolved = app
            .resolve_deviation(&ready.id, &deviation.id)
            .expect("resolve deviation");
        assert!(resolved
            .blocking_deviations
            .iter()
            .any(|item| item.id == deviation.id && item.resolved && item.resolved_at.is_some()));
        let allowed = app
            .validate_track(&ready.id)
            .expect("validation after resolution");
        assert!(
            allowed.valid,
            "missing={:?}; blocking={:?}",
            allowed.missing_items, allowed.blocking_items
        );
        let finalized = app
            .finalize_track(&ready.id)
            .expect("finalization after resolution")
            .track
            .expect("finalized detail");
        assert_eq!(finalized.status, TrackStatus::Finalized);
        assert!(finalized.certificate.valid);
    }

    #[test]
    fn external_change_invalidates_certificate_state_without_rewriting_certificate_files() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "External Mutation",
        );
        let track_root = app.root().join(&finalized.relative_path);
        let certificate_before = certificate_file_snapshot(&track_root);
        let release = finalized
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence");
        fs::write(
            track_root.join(&release.relative_path),
            b"RIFF\x08\0\0\0WAVEexternally changed release",
        )
        .expect("external track mutation");

        let loaded = app
            .load_track(&finalized.id)
            .expect("load externally changed finalized track");
        assert_eq!(loaded.status, TrackStatus::Finalized);
        assert!(!loaded.integrity.verified);
        assert!(!loaded.certificate.valid);
        assert!(loaded.certificate.invalidated_at.is_some());
        assert_eq!(certificate_file_snapshot(&track_root), certificate_before);
        certificate::verify(&track_root).expect("unchanged certificate file set remains intact");
    }

    #[test]
    fn technical_pdf_mutation_fails_certificate_verification_and_invalidates_state() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized =
            finalize_acceptance_track(&app, &directory.path().join("fixtures"), "PDF Mutation");
        let track_root = app.root().join(&finalized.relative_path);
        let pdf_path = track_root.join(certificate::PDF_FILE);
        let mut changed = fs::read(&pdf_path).expect("technical PDF");
        changed.push(b'X');
        fs::write(&pdf_path, changed).expect("mutate technical PDF");

        let error = certificate::verify(&track_root).expect_err("modified PDF must fail");
        assert!(error.to_string().contains(certificate::PDF_FILE));
        let loaded = app
            .load_track(&finalized.id)
            .expect("load track with modified PDF");
        assert_eq!(loaded.status, TrackStatus::Finalized);
        assert!(!loaded.certificate.valid);
        assert!(loaded.certificate.invalidated_at.is_some());
    }

    #[test]
    fn workspace_scan_does_not_index_the_root_pdf_as_evidence() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "PDF Scan Exclusion",
        );
        let evidence_count = finalized.evidence.len();

        app.scan_workspace().expect("workspace scan");
        let rescanned = app.load_track(&finalized.id).expect("rescanned track");
        assert_eq!(rescanned.evidence.len(), evidence_count);
        assert!(!rescanned
            .evidence
            .iter()
            .any(|item| item.relative_path == certificate::PDF_FILE));
        assert!(rescanned.certificate.valid);
    }

    #[test]
    fn corrupted_certificate_can_be_preserved_in_a_recovery_revision() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Certificate Recovery",
        );
        let track_root = app.root().join(&finalized.relative_path);
        fs::write(
            track_root.join(certificate::CERTIFICATE_HASH_FILE),
            b"damaged certificate hash list\n",
        )
        .expect("damage certificate hashes");

        let invalid = app
            .load_track(&finalized.id)
            .expect("invalid finalized snapshot remains loadable");
        assert!(!invalid.certificate.valid);
        let revision = app
            .create_revision(&finalized.id)
            .expect("recovery revision archives even an invalid set")
            .track
            .expect("active recovery detail");
        assert_ne!(revision.status, TrackStatus::Finalized);
        let archives = fs::read_dir(track_root.join(".archive/revisions"))
            .expect("revision archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("archive entries");
        assert_eq!(archives.len(), 1);
        let archive = archives[0].path();
        assert!(archive.join("certificate/CERTIFICATE_SHA256.txt").is_file());
        assert!(archive.join(certificate::PDF_FILE).is_file());
        assert!(archive.join(integrity::HASH_FILE).is_file());
        let metadata =
            fs::read_to_string(archive.join("revision.json")).expect("revision metadata");
        assert!(metadata.contains("invalid_or_incomplete"));
    }

    #[test]
    fn finalization_with_historical_certificate_sentinel_collides_before_marker_creation() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Historical Certificate Collision",
        );
        let track_root = app.root().join(&ready.relative_path);
        let sentinel = track_root.join("06_CERTIFICATE/historical-sentinel.bin");
        let sentinel_bytes = b"historical certificate bytes must never be overwritten";
        fs::write(&sentinel, sentinel_bytes).expect("historical certificate sentinel");
        let marker = track_root.join(".archive/finalization-in-progress.json");

        let error = app
            .finalize_track(&ready.id)
            .expect_err("historical certificate content must block finalization");
        assert!(matches!(error, AppError::Collision(_)));
        assert_eq!(
            fs::read(&sentinel).expect("unchanged historical sentinel"),
            sentinel_bytes
        );
        assert!(
            !marker.exists(),
            "collision must precede marker publication"
        );
        assert_eq!(
            fs::read_dir(track_root.join("06_CERTIFICATE"))
                .expect("certificate directory")
                .collect::<std::result::Result<Vec<_>, _>>()
                .expect("certificate entries")
                .len(),
            1
        );
        let unchanged = app.load_track(&ready.id).expect("track remains loadable");
        assert_ne!(unchanged.status, TrackStatus::Finalized);
        assert!(!unchanged.certificate.valid);
    }

    #[test]
    fn finalization_never_overwrites_an_existing_root_pdf() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Historical PDF Collision",
        );
        let track_root = app.root().join(&ready.relative_path);
        let pdf = track_root.join(certificate::PDF_FILE);
        let sentinel = b"existing technical PDF must not be overwritten";
        fs::write(&pdf, sentinel).expect("historical PDF sentinel");

        let error = app
            .finalize_track(&ready.id)
            .expect_err("existing PDF must block finalization");
        assert!(matches!(error, AppError::Collision(_)));
        assert_eq!(fs::read(&pdf).expect("unchanged PDF sentinel"), sentinel);
        assert!(!track_root
            .join(".archive/finalization-in-progress.json")
            .exists());
        assert!(
            directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
                .expect("empty certificate directory")
        );
    }

    #[test]
    fn finalization_database_commit_failure_rolls_back_publication_and_reopens_cleanly() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let ready = prepare_ready_track(
            &app,
            &directory.path().join("fixtures"),
            "Database Commit Failure",
        );
        let track_root = workspace.join(&ready.relative_path);

        let error = app
            .finalize_track_impl(
                &ready.id,
                Some(FinalizationFailure::DatabaseCommit),
                &mut |_| {},
            )
            .expect_err("injected database commit failure");
        assert!(error
            .to_string()
            .contains("Injected finalization database commit failure"));
        let stored = app
            .persistence
            .track(&ready.id)
            .expect("stored active track");
        assert_ne!(stored.status, TrackStatus::Finalized);
        assert!(!stored.certificate.valid);
        assert!(
            directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
                .expect("empty certificate directory")
        );
        assert!(!track_root.join(certificate::PDF_FILE).exists());
        assert!(!track_root
            .join(".archive/finalization-in-progress.json")
            .exists());
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("clean workspace reopen");
        let detail = reopened.load_track(&ready.id).expect("load active track");
        assert_ne!(detail.status, TrackStatus::Finalized);
        assert!(!detail.certificate.valid);
        assert!(
            directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
                .expect("empty certificate after reopen")
        );
        assert!(!track_root.join(certificate::PDF_FILE).exists());
    }

    #[test]
    fn technical_pdf_failures_leave_no_partial_finalization() {
        for (failure, label) in [
            (FinalizationFailure::PdfGeneration, "generation"),
            (FinalizationFailure::PdfStaging, "staging"),
            (FinalizationFailure::PdfPublication, "publication"),
            (
                FinalizationFailure::PostPublishVerification,
                "post-publish-verification",
            ),
        ] {
            let directory = tempdir().expect("temporary directory");
            let workspace = directory.path().join("workspace");
            let app = WorkspaceApp::open(&workspace, true).expect("workspace");
            app.update_profile(complete_profile()).expect("profile");
            let ready = prepare_ready_track(
                &app,
                &directory.path().join("fixtures"),
                &format!("PDF Failure {label}"),
            );
            let track_root = workspace.join(&ready.relative_path);

            app.finalize_track_impl(&ready.id, Some(failure), &mut |_| {})
                .expect_err("injected PDF finalization failure");
            let stored = app
                .persistence
                .track(&ready.id)
                .expect("stored active track");
            assert_ne!(stored.status, TrackStatus::Finalized, "{label}");
            assert!(!stored.certificate.valid, "{label}");
            assert!(!track_root.join(certificate::PDF_FILE).exists(), "{label}");
            assert!(
                directory_is_empty_or_missing(&track_root.join(certificate::CERTIFICATE_DIR))
                    .expect("empty live certificate"),
                "{label}"
            );
            assert!(
                !track_root
                    .join(".archive/finalization-in-progress.json")
                    .exists(),
                "{label}"
            );
            let staging = track_root.join(".archive/certificate-staging");
            assert!(
                !staging.exists()
                    || fs::read_dir(&staging)
                        .expect("certificate staging")
                        .next()
                        .is_none(),
                "{label}"
            );
        }
    }

    #[test]
    fn workspace_reopen_recovers_filesystem_database_commit_windows() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let active = app
            .create_track(CreateTrackInput {
                title: "Interrupted Finalization".into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended: false,
                library: TrackLibraryPlacement::default(),
            })
            .expect("active track");
        let active_root = workspace.join(&active.relative_path);
        fs::write(
            active_root.join("06_CERTIFICATE/orphan-certificate.md"),
            b"published before database commit",
        )
        .expect("orphan certificate fixture");
        fs::write(
            active_root.join(certificate::PDF_FILE),
            b"root PDF published before database commit",
        )
        .expect("orphan root PDF fixture");
        let interrupted_transaction = "interrupted-finalization-fixture";
        fs::write(
            active_root.join(".archive/finalization-in-progress.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1,
                "transaction_id": interrupted_transaction,
                "track_id": active.id,
                "certificate_id": "SDM-interrupted-fixture",
                "started_at": "2026-08-13T12:00:00Z"
            }))
            .expect("finalization recovery marker"),
        )
        .expect("write finalization recovery marker");
        let interrupted_stage = active_root
            .join(".archive/certificate-staging")
            .join(interrupted_transaction);
        fs::create_dir_all(&interrupted_stage).expect("interrupted staging directory");
        fs::write(
            interrupted_stage.join("EVIDENCE_MANIFEST.json"),
            b"staged before process exit",
        )
        .expect("interrupted staged artifact");
        drop(app);

        let reopened = WorkspaceApp::open(&workspace, false).expect("recovered workspace");
        assert!(
            directory_is_empty_or_missing(&active_root.join("06_CERTIFICATE"))
                .expect("empty live certificate")
        );
        let recovered_files = WalkDir::new(active_root.join(".archive/recovery"))
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("recovery archive");
        assert!(recovered_files
            .iter()
            .any(|entry| entry.file_name() == "orphan-certificate.md"));
        assert_eq!(
            fs::read(
                active_root
                    .join(".archive/recovery")
                    .join(interrupted_transaction)
                    .join("certificate/orphan-certificate.md")
            )
            .expect("marker-selected recovery snapshot"),
            b"published before database commit"
        );
        assert_eq!(
            fs::read(
                active_root
                    .join(".archive/recovery")
                    .join(interrupted_transaction)
                    .join(certificate::PDF_FILE)
            )
            .expect("recovered root PDF"),
            b"root PDF published before database commit"
        );
        assert_eq!(
            fs::read(
                active_root
                    .join(".archive/recovery")
                    .join(interrupted_transaction)
                    .join("certificate-staging/EVIDENCE_MANIFEST.json")
            )
            .expect("correlated staging recovery"),
            b"staged before process exit"
        );
        assert!(!interrupted_stage.exists());
        assert!(!active_root
            .join(".archive/finalization-in-progress.json")
            .exists());

        let finalized = finalize_acceptance_track(
            &reopened,
            &directory.path().join("fixtures"),
            "Interrupted Revision",
        );
        let finalized_root = workspace.join(&finalized.relative_path);
        let wrong_archive = finalized_root.join(".archive/revisions/zzzz-wrong-certificate");
        fs::create_dir(&wrong_archive).expect("wrong revision archive");
        fs::create_dir(wrong_archive.join("certificate")).expect("wrong certificate directory");
        fs::write(
            wrong_archive.join("certificate/DOCUMENTATION_CERTIFICATE.md"),
            b"wrong certificate snapshot",
        )
        .expect("wrong archived certificate");
        let mut wrong_certificate = finalized.certificate.clone();
        wrong_certificate.certificate_id = Some("SDM-different-revision".into());
        fs::write(
            wrong_archive.join("revision.json"),
            serde_json::to_vec(&serde_json::json!({
                "track_id": finalized.id,
                "previous_certificate": wrong_certificate,
                "reason": "older unrelated revision"
            }))
            .expect("wrong revision metadata"),
        )
        .expect("wrong revision metadata file");
        let crash_archive = finalized_root.join(".archive/revisions/crash-fixture");
        fs::create_dir(&crash_archive).expect("crash archive");
        fs::write(
            crash_archive.join("revision.json"),
            serde_json::to_vec(&serde_json::json!({
                "track_id": finalized.id,
                "previous_certificate": finalized.certificate,
                "reason": "simulated process exit before database commit"
            }))
            .expect("crash metadata"),
        )
        .expect("crash metadata file");
        fs::rename(
            finalized_root.join(certificate::CERTIFICATE_DIR),
            crash_archive.join("certificate"),
        )
        .expect("simulate revision publish before DB commit");
        fs::rename(
            finalized_root.join(certificate::PDF_FILE),
            crash_archive.join(certificate::PDF_FILE),
        )
        .expect("simulate revision PDF archive before DB commit");
        fs::create_dir(finalized_root.join(certificate::CERTIFICATE_DIR))
            .expect("empty live certificate directory");
        drop(reopened);

        let recovered = WorkspaceApp::open(&workspace, false).expect("revision recovery");
        certificate::verify(&finalized_root).expect("certificate restored to live snapshot");
        assert!(!crash_archive.join("certificate").exists());
        assert!(!crash_archive.join(certificate::PDF_FILE).exists());
        assert!(finalized_root.join(certificate::PDF_FILE).is_file());
        assert!(wrong_archive.join("certificate").is_dir());
        assert_eq!(
            fs::read(wrong_archive.join("certificate/DOCUMENTATION_CERTIFICATE.md"))
                .expect("unrelated archived certificate remains untouched"),
            b"wrong certificate snapshot"
        );
        let detail = recovered
            .load_track(&finalized.id)
            .expect("recovered finalized track");
        assert_eq!(detail.status, TrackStatus::Finalized);
        assert!(detail.certificate.valid);
    }

    #[test]
    fn always_disclosure_policy_requires_final_artwork_to_match_local_disclosure_output() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let fixtures = directory.path().join("fixtures");
        let base = prepare_ready_track(&app, &fixtures, "Disclosure Lineage");
        app.update_track(
            &base.id,
            TrackPatch {
                artwork_origin: Some("ai_assisted".into()),
                ai_image_service: Some("Local Tool".into()),
                human_artwork_modifications: Some(vec!["Visible disclosure added locally".into()]),
                depicts_real_person: Some(true),
                real_person_notes: Some("Documented real-person depiction".into()),
                depicts_real_event: Some(false),
                contains_trademark: Some(false),
                disclosure_applied: Some(true),
                disclosure_text: Some("AI-assisted".into()),
                ..TrackPatch::default()
            },
        )
        .expect("AI artwork facts");
        let ai_original = fixtures.join("lineage-original.png");
        let manually_edited = fixtures.join("manually-imported-edited.png");
        image::RgbaImage::from_pixel(640, 640, image::Rgba([20, 40, 80, 255]))
            .save(&ai_original)
            .expect("AI original fixture");
        image::RgbaImage::from_pixel(640, 640, image::Rgba([180, 30, 60, 255]))
            .save(&manually_edited)
            .expect("manually edited artwork fixture");
        app.import_evidence_from(&base.id, EvidenceRole::AiArtworkOriginal, &ai_original)
            .expect("AI original import");
        let imported = app
            .import_evidence_from(&base.id, EvidenceRole::AiArtworkEdited, &manually_edited)
            .expect("manually edited artwork import");
        let imported_edited = imported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::AiArtworkEdited)
            .expect("imported AI-edited evidence")
            .clone();
        assert_eq!(imported_edited.provenance, EvidenceProvenance::ManagedCopy);
        assert_eq!(imported.fields.disclosure_applied, Some(true));
        let track_root = app.root().join(&base.relative_path);
        let with_matching_import = app
            .import_evidence_from(
                &base.id,
                EvidenceRole::FinalArtwork,
                &track_root.join(&imported_edited.relative_path),
            )
            .expect("hash-equal final artwork import");
        let imported_final = with_matching_import
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::FinalArtwork)
            .expect("hash-equal imported final evidence")
            .clone();
        assert_eq!(imported_final.sha256, imported_edited.sha256);
        app.generate_documents(&base.id, false)
            .expect("documents with imported artwork pair");
        app.calculate_hashes(&base.id)
            .expect("hashes with imported artwork pair");
        let rejected = app
            .validate_track(&base.id)
            .expect("imported-lineage gate evaluation");
        assert!(!rejected.valid);
        assert!(rejected
            .missing_items
            .iter()
            .any(|item| item.contains("AI-Kennzeichnung")));
        assert_eq!(
            rejected
                .missing_items
                .iter()
                .filter(|item| item.contains("finale Artwork"))
                .count(),
            1,
            "final artwork must be required exactly once"
        );
        assert!(matches!(
            app.finalize_track(&base.id),
            Err(AppError::Validation(_))
        ));

        app.remove_evidence(&base.id, &imported_final.id)
            .expect("remove untrusted final artwork");
        let after_imported_edit_removal = app
            .remove_evidence(&base.id, &imported_edited.id)
            .expect("remove manually imported AI-edited artwork");
        assert_eq!(
            after_imported_edit_removal.fields.disclosure_applied,
            Some(true),
            "a manually asserted flag alone must not become trusted provenance"
        );
        let disclosed = app
            .generate_artwork_disclosure(&base.id, Some("AI-assisted".into()))
            .expect("local disclosure output")
            .track
            .expect("disclosed track");
        let generated = disclosed
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::AiArtworkEdited)
            .expect("locally generated AI-edited evidence")
            .clone();
        assert_eq!(
            generated.provenance,
            EvidenceProvenance::GeneratedDisclosure
        );
        assert_eq!(
            generated.derived_from_evidence_id.as_deref(),
            disclosed
                .evidence
                .iter()
                .find(|item| item.role == EvidenceRole::AiArtworkOriginal)
                .map(|item| item.id.as_str())
        );
        assert_eq!(
            generated.generator_version.as_deref(),
            Some(crate::artwork::DISCLOSURE_GENERATOR_VERSION)
        );
        let with_linked_final = app
            .import_evidence_from(
                &base.id,
                EvidenceRole::FinalArtwork,
                &track_root.join(&generated.relative_path),
            )
            .expect("import final artwork from disclosed bytes");
        let linked = with_linked_final
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::FinalArtwork)
            .expect("linked final evidence");
        assert_eq!(linked.sha256, generated.sha256);
        assert_eq!(
            fs::read(track_root.join(&linked.relative_path)).expect("linked final bytes"),
            fs::read(track_root.join(&generated.relative_path)).expect("disclosure output bytes")
        );
        app.generate_documents(&base.id, false)
            .expect("current linked documents");
        app.calculate_hashes(&base.id)
            .expect("current linked hashes");
        let accepted = app
            .validate_track(&base.id)
            .expect("linked gate evaluation");
        assert!(
            accepted.valid,
            "missing={:?}; blocking={:?}",
            accepted.missing_items, accepted.blocking_items
        );
        let finalized = app
            .finalize_track(&base.id)
            .expect("linked artwork finalization")
            .track
            .expect("finalized linked track");
        assert_eq!(finalized.status, TrackStatus::Finalized);
        assert!(finalized.certificate.valid);
    }

    #[test]
    fn missing_evidence_can_be_removed_and_reimported_during_recovery_revision() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let fixtures = directory.path().join("fixtures");
        let finalized = finalize_acceptance_track(&app, &fixtures, "Missing Evidence Recovery");
        let release = finalized
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("release evidence")
            .clone();
        let track_root = app.root().join(&finalized.relative_path);
        let managed_path = track_root.join(&release.relative_path);
        fs::remove_file(&managed_path).expect("external evidence deletion");
        let invalid = app
            .load_track(&finalized.id)
            .expect("load invalidated finalized track");
        assert!(!invalid.certificate.valid);
        assert!(invalid
            .evidence
            .iter()
            .any(|item| item.id == release.id && !item.verified));

        let revision = app
            .create_revision(&finalized.id)
            .expect("start recovery revision")
            .track
            .expect("active recovery track");
        assert_eq!(revision.status, TrackStatus::Active);
        let removed = app
            .remove_evidence(&finalized.id, &release.id)
            .expect("remove metadata for externally missing evidence");
        assert!(!removed.evidence.iter().any(|item| item.id == release.id));
        assert!(!managed_path.exists());

        let reimported = app
            .import_evidence_from(
                &finalized.id,
                EvidenceRole::ReleaseWav,
                &fixtures.join("release-master.wav"),
            )
            .expect("reimport same managed target name");
        let replacement = reimported
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::ReleaseWav)
            .expect("replacement release evidence");
        assert_ne!(replacement.id, release.id);
        assert_eq!(replacement.relative_path, release.relative_path);
        assert!(replacement.verified);
        assert!(managed_path.is_file());

        app.update_track(
            &finalized.id,
            TrackPatch {
                release_filename_difference_confirmed: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("reconfirm the reimported release filename difference");

        app.generate_documents(&finalized.id, false)
            .expect("regenerate recovery documents");
        app.calculate_hashes(&finalized.id)
            .expect("regenerate recovery hashes");
        let recovered_gate = app
            .validate_track(&finalized.id)
            .expect("validate recovered revision");
        assert!(
            recovered_gate.valid,
            "missing={:?}; blocking={:?}",
            recovered_gate.missing_items, recovered_gate.blocking_items
        );
        let recovered = app
            .finalize_track(&finalized.id)
            .expect("finalize recovered revision")
            .track
            .expect("recovered finalized track");
        assert_eq!(recovered.status, TrackStatus::Finalized);
        assert!(recovered.certificate.valid);
        let manifest = fs::read_to_string(track_root.join(certificate::MANIFEST_FILE))
            .expect("recovered manifest");
        let markdown = fs::read_to_string(track_root.join(certificate::CERTIFICATE_FILE))
            .expect("recovered Markdown certificate");
        assert!(manifest.contains(".archive/revisions/"));
        assert!(markdown.contains(".archive/revisions/"));
        let pdf = fs::read(track_root.join(certificate::PDF_FILE)).expect("recovered PDF");
        let mut warnings = Vec::new();
        let pdf_text = printpdf::PdfDocument::parse(
            &pdf,
            &printpdf::PdfParseOptions::default(),
            &mut warnings,
        )
        .expect("parse recovered PDF")
        .extract_text()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        assert!(pdf_text.contains(".archive/revisions/"));
    }

    #[test]
    fn workflow_upgrade_archives_finalized_v18_and_requires_fresh_v19_outputs() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized =
            finalize_acceptance_track(&app, &directory.path().join("fixtures"), "Workflow Upgrade");
        assert_eq!(finalized.workflow_version, "1.8");
        let track_root = app.root().join(&finalized.relative_path);
        let certificate_before = certificate_file_snapshot(&track_root);
        let hashes_before =
            fs::read(track_root.join(integrity::HASH_FILE)).expect("read finalized SHA256SUMS");
        app.persistence
            .save_step(
                &finalized.id,
                &StepState {
                    id: "source".into(),
                    status: StepStatus::Fail,
                    na_reason: None,
                    updated_at: Some("2026-08-13T12:00:00Z".into()),
                },
            )
            .expect("stored old-workflow override");

        let workflow_v19 = workflow::config_with_version_for_test("1.9")
            .expect("test-only workflow 1.9 configuration");
        let upgraded = app
            .re_evaluate_track_with_workflow(&finalized.id, &workflow_v19)
            .expect("explicit workflow reevaluation")
            .track
            .expect("upgraded track detail");

        assert_eq!(upgraded.status, TrackStatus::Active);
        assert_eq!(upgraded.workflow_id, "suno-track");
        assert_eq!(upgraded.workflow_version, "1.9");
        assert!(!upgraded.documents.current);
        assert!(!upgraded.integrity.generated);
        assert!(!upgraded.integrity.verified);
        assert!(!upgraded.certificate.valid);
        assert!(upgraded.certificate.certificate_id.is_none());
        assert!(!track_root.join(integrity::HASH_FILE).exists());
        assert!(app
            .persistence
            .stored_steps(&finalized.id)
            .expect("stored steps after upgrade")
            .is_empty());

        let archives = fs::read_dir(track_root.join(".archive/revisions"))
            .expect("revision archive")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("revision entries");
        assert_eq!(archives.len(), 1);
        let archive = archives[0].path();
        assert_eq!(
            fs::read(archive.join(integrity::HASH_FILE)).expect("archived SHA256SUMS"),
            hashes_before
        );
        for (relative, bytes) in certificate_before {
            let archived = if relative == certificate::PDF_FILE {
                archive.join(certificate::PDF_FILE)
            } else {
                let file_name = Path::new(&relative)
                    .file_name()
                    .expect("certificate file name");
                archive.join("certificate").join(file_name)
            };
            assert_eq!(
                fs::read(archived).expect("archived certificate byte snapshot"),
                bytes
            );
        }

        assert!(matches!(
            app.re_evaluate_track_with_workflow(&finalized.id, &workflow_v19),
            Err(AppError::Validation(_))
        ));
        assert_eq!(
            fs::read_dir(track_root.join(".archive/revisions"))
                .expect("unchanged revision archive")
                .count(),
            1
        );
    }

    const P0_SUNO_ID: &str = "6c8a40fd-32bf-4c7b-ab59-23579ff95828";
    const P0_SECOND_SUNO_ID: &str = "7d9b51ae-43cf-4d8c-bc6a-3468a00a6929";
    const P0_THIRD_SUNO_ID: &str = "8ea062bf-54d0-4e9d-cd7b-4579b11b7a3a";

    fn p0_suno_comment(timestamp: &str) -> String {
        p0_suno_comment_with_id(timestamp, P0_SUNO_ID)
    }

    fn p0_suno_comment_with_id(timestamp: &str, id: &str) -> String {
        format!("made with suno studio; created={timestamp}; id={id}")
    }

    /// Test-only RIFF encoder. It intentionally does not call any production
    /// parser or fixture utility, so malformed parser offsets cannot be masked
    /// by an encoder sharing the same implementation.
    fn p0_pcm_wav(comment: Option<&str>) -> Vec<u8> {
        let entries = comment
            .map(|value| vec![(*b"ICMT", value.as_bytes().to_vec())])
            .unwrap_or_default();
        p0_pcm_wav_with_info_entries(&entries)
    }

    /// A longer, non-silent PCM fixture for tests that deliberately exercise
    /// the real bundled Chromaprint sidecar. Keep the regular P0 WAV tiny so
    /// metadata-parser tests retain their existing 10 ms assertions.
    fn p0_screening_wav(comment: Option<&str>, seed: u8) -> Vec<u8> {
        let entries = comment
            .map(|value| vec![(*b"ICMT", value.as_bytes().to_vec())])
            .unwrap_or_default();
        let frames = 48_000_usize * 4;
        let mut audio = Vec::with_capacity(frames * 4);
        for frame in 0..frames {
            let phase = (frame * (17 + usize::from(seed))) % 109;
            let sample = ((phase as i32 * 2 - 108) * 220) as i16;
            for _ in 0..2 {
                audio.extend_from_slice(&sample.to_le_bytes());
            }
        }
        p0_pcm_wav_with_info_entries_and_audio(&entries, &audio)
    }

    fn p0_pcm_wav_with_info_entries(entries: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
        p0_pcm_wav_with_info_entries_and_audio(entries, &vec![0; 1_920])
    }

    fn p0_pcm_wav_with_info_entries_and_audio(
        entries: &[([u8; 4], Vec<u8>)],
        audio: &[u8],
    ) -> Vec<u8> {
        fn append_chunk(destination: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
            destination.extend_from_slice(id);
            destination.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            destination.extend_from_slice(payload);
            if payload.len() % 2 == 1 {
                destination.push(0);
            }
        }

        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&2_u16.to_le_bytes());
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&192_000_u32.to_le_bytes());
        fmt.extend_from_slice(&4_u16.to_le_bytes());
        fmt.extend_from_slice(&16_u16.to_le_bytes());

        let mut chunks = Vec::new();
        append_chunk(&mut chunks, b"fmt ", &fmt);
        append_chunk(&mut chunks, b"data", audio);
        if !entries.is_empty() {
            let mut info = b"INFO".to_vec();
            for (id, entry) in entries {
                let mut value = entry.clone();
                value.push(0);
                append_chunk(&mut info, id, &value);
            }
            append_chunk(&mut chunks, b"LIST", &info);
        }

        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(&chunks);
        wav
    }

    fn p0_track(
        app: &WorkspaceApp,
        title: &str,
        post_export_editing: Option<bool>,
        commercial_use_intended: bool,
    ) -> TrackDetail {
        if app
            .profile()
            .expect("P0 profile")
            .artist_name
            .trim()
            .is_empty()
        {
            app.update_profile(complete_profile())
                .expect("P0 profile setup");
        }
        let created = app
            .create_track(CreateTrackInput {
                title: title.into(),
                production_start_date: "2026-08-01".into(),
                commercial_use_intended,
                library: TrackLibraryPlacement::default(),
            })
            .expect("P0 track creation");
        let Some(post_export_editing) = post_export_editing else {
            return created;
        };
        app.update_track(
            &created.id,
            TrackPatch {
                post_export_editing_performed: Some(post_export_editing),
                post_export_editing_details: post_export_editing
                    .then(|| "Mastering after the Suno export".into()),
                ..TrackPatch::default()
            },
        )
        .expect("P0 post-export fact")
    }

    fn p0_evidence(detail: &TrackDetail, role: EvidenceRole) -> &EvidenceItem {
        detail
            .evidence
            .iter()
            .find(|item| item.role == role)
            .expect("P0 evidence role")
    }

    #[derive(Debug, PartialEq, Eq)]
    struct P0RawFinalizedRows {
        track_data_json: String,
        track_updated_at: String,
        track_status: String,
        track_workflow_version: String,
        evidence_metadata_json: String,
        evidence_imported_at: String,
        evidence_verified: i64,
        evidence_verification_error: Option<String>,
        evidence_sha256: Option<String>,
        evidence_size_bytes: i64,
    }

    fn p0_raw_finalized_rows(
        app: &WorkspaceApp,
        track_id: &str,
        evidence_id: &str,
    ) -> P0RawFinalizedRows {
        let connection = app.persistence.open().expect("raw workspace database");
        let (track_data_json, track_updated_at, track_status, track_workflow_version) = connection
            .query_row(
                "SELECT data_json,updated_at,status,workflow_version FROM tracks WHERE id=?1",
                [track_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("raw finalized track row");
        let (
            evidence_metadata_json,
            evidence_imported_at,
            evidence_verified,
            evidence_verification_error,
            evidence_sha256,
            evidence_size_bytes,
        ) = connection
            .query_row(
                "SELECT metadata_json,imported_at,verified,verification_error,sha256,size_bytes FROM evidence WHERE track_id=?1 AND id=?2",
                rusqlite::params![track_id, evidence_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("raw finalized evidence row");
        P0RawFinalizedRows {
            track_data_json,
            track_updated_at,
            track_status,
            track_workflow_version,
            evidence_metadata_json,
            evidence_imported_at,
            evidence_verified,
            evidence_verification_error,
            evidence_sha256,
            evidence_size_bytes,
        }
    }

    #[test]
    fn p0_suno_import_persists_exact_metadata_and_derives_authoritative_dates() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Metadata Import", None, false);
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");
        let source = directory.path().join("suno-metadata.wav");
        let wav = p0_pcm_wav(Some(&raw));
        fs::write(&source, &wav).expect("write Suno WAV");

        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("import Suno WAV");
        let evidence = p0_evidence(&imported, EvidenceRole::SunoFinalExport);

        assert!(evidence.verified);
        assert!(evidence
            .sha256
            .as_deref()
            .is_some_and(|hash| !hash.is_empty()));
        assert_eq!(evidence.size_bytes, wav.len() as u64);
        assert_eq!(evidence.metadata.original_file_name, "suno-metadata.wav");
        assert_eq!(evidence.metadata.file_extension, "wav");
        assert_eq!(evidence.metadata.mime_type, "audio/wav");
        assert_eq!(evidence.metadata.audio_format, "WAV");
        assert_eq!(evidence.metadata.audio_channels, Some(2));
        assert_eq!(evidence.metadata.audio_sample_rate_hz, Some(48_000));
        assert_eq!(evidence.metadata.audio_duration_milliseconds, Some(10));
        assert_eq!(evidence.metadata.audio_bit_depth, Some(16));
        assert_eq!(
            evidence.metadata.embedded_metadata,
            vec![EmbeddedMetadata {
                key: "ICMT".into(),
                value: raw.clone(),
            }]
        );
        assert!(evidence.metadata.suno_studio_detected);
        assert_eq!(
            evidence.metadata.suno_created_timestamp,
            "2026-08-17T06:38:06Z"
        );
        assert_eq!(evidence.metadata.suno_created_date, "2026-08-17");
        assert_eq!(evidence.metadata.suno_id, P0_SUNO_ID);
        assert_eq!(evidence.metadata.suno_raw_metadata, raw);
        assert_eq!(imported.fields.suno_final_generation_id, P0_SUNO_ID);
        assert_eq!(
            imported.automation.final_generation_id_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(
            imported.automation.final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(imported.fields.production_end_date, "2026-08-17");
        assert_eq!(
            imported.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(imported.fields.suno_download_export_date, "2026-08-17");
        assert_eq!(
            imported.automation.download_export_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert!(imported.fields.final_export_date.is_empty());
    }

    #[test]
    fn p0_final_generation_id_follows_wav_only_while_system_owned() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Automatic Generation ID", Some(false), false);
        let first_source = directory.path().join("automatic-id-first.wav");
        fs::write(
            &first_source,
            p0_pcm_wav(Some(&p0_suno_comment_with_id(
                "2026-08-17T06:38:06Z",
                P0_SUNO_ID,
            ))),
        )
        .expect("first Suno WAV");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
            .expect("import automatic ID");
        let evidence_id = p0_evidence(&imported, EvidenceRole::SunoFinalExport)
            .id
            .clone();
        assert_eq!(imported.fields.suno_final_generation_id, P0_SUNO_ID);
        assert_eq!(
            imported.automation.final_generation_id_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        let replacement_source = directory.path().join("automatic-id-replacement.wav");
        fs::write(
            &replacement_source,
            p0_pcm_wav(Some(&p0_suno_comment_with_id(
                "2026-08-18T06:38:06Z",
                P0_SECOND_SUNO_ID,
            ))),
        )
        .expect("replacement Suno WAV");
        let replaced = app
            .replace_evidence_from(
                &track.id,
                &evidence_id,
                EvidenceRole::SunoFinalExport,
                &replacement_source,
            )
            .expect("replace automatic ID");
        assert_eq!(replaced.fields.suno_final_generation_id, P0_SECOND_SUNO_ID);
        assert_eq!(
            replaced.automation.final_generation_id_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        let overridden = app
            .update_track(
                &track.id,
                TrackPatch {
                    suno_final_generation_id: Some("manually-confirmed-generation-id".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("record manual ID");
        assert_eq!(
            overridden.fields.suno_final_generation_id,
            "manually-confirmed-generation-id"
        );
        assert_eq!(
            overridden.automation.final_generation_id_origin,
            FactOrigin::UserConfirmedFact
        );

        let third_source = directory.path().join("automatic-id-third.wav");
        fs::write(
            &third_source,
            p0_pcm_wav(Some(&p0_suno_comment_with_id(
                "2026-08-19T06:38:06Z",
                P0_THIRD_SUNO_ID,
            ))),
        )
        .expect("third Suno WAV");
        let preserved = app
            .replace_evidence_from(
                &track.id,
                &evidence_id,
                EvidenceRole::SunoFinalExport,
                &third_source,
            )
            .expect("replace while manual ID exists");
        assert_eq!(
            preserved.fields.suno_final_generation_id,
            "manually-confirmed-generation-id"
        );
        assert_eq!(
            preserved.automation.final_generation_id_origin,
            FactOrigin::UserConfirmedFact
        );

        let manual_track = p0_track(&app, "P0 Preexisting Generation ID", Some(false), false);
        let manual_track = app
            .update_track(
                &manual_track.id,
                TrackPatch {
                    suno_final_generation_id: Some("preexisting-manual-id".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("record preexisting manual ID");
        let manual_source = directory.path().join("manual-id.wav");
        fs::write(
            &manual_source,
            p0_pcm_wav(Some(&p0_suno_comment_with_id(
                "2026-08-17T06:38:06Z",
                P0_SUNO_ID,
            ))),
        )
        .expect("manual-ID Suno WAV");
        let preserved_manual = app
            .import_evidence_from(
                &manual_track.id,
                EvidenceRole::SunoFinalExport,
                &manual_source,
            )
            .expect("import alongside manual ID");
        assert_eq!(
            preserved_manual.fields.suno_final_generation_id,
            "preexisting-manual-id"
        );
        assert_eq!(
            preserved_manual.automation.final_generation_id_origin,
            FactOrigin::UserConfirmedFact
        );
    }

    #[test]
    fn p0_no_post_editing_derives_production_end_and_identical_release_passes() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Identical Audio", Some(false), false);
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");
        let bytes = p0_pcm_wav(Some(&raw));
        let suno_source = directory.path().join("separate-suno.wav");
        let release_source = directory.path().join("separate-release.wav");
        fs::write(&suno_source, &bytes).expect("write Suno source");
        fs::write(&release_source, &bytes).expect("write release source");

        app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &suno_source)
            .expect("import Suno WAV");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::ReleaseWav, &release_source)
            .expect("import byte-identical release WAV");
        let suno = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
        let release = p0_evidence(&imported, EvidenceRole::ReleaseWav);

        assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(imported.fields.production_end_date, "2026-08-17");
        assert_eq!(imported.fields.suno_download_export_date, "2026-08-17");
        assert_eq!(imported.fields.final_export_date, "2026-08-17");
        assert_eq!(
            imported.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(suno.sha256, release.sha256);
        assert!(imported.automation.release_identical_to_suno_export);
        assert!(imported.automation.byte_identical_pairs.iter().any(|pair| {
            pair.sha256 == suno.sha256.clone().unwrap_or_default()
                && matches!(
                    (pair.left_role, pair.right_role),
                    (EvidenceRole::SunoFinalExport, EvidenceRole::ReleaseWav)
                        | (EvidenceRole::ReleaseWav, EvidenceRole::SunoFinalExport)
                )
        }));
    }

    #[test]
    fn p0_metadata_date_remains_authoritative_when_post_export_editing_changes() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");

        let edited_first = p0_track(&app, "P0 Editing Before Import", Some(true), false);
        let edited_first_source = directory.path().join("edited-first.wav");
        fs::write(&edited_first_source, p0_pcm_wav(Some(&raw))).expect("write first WAV");
        let edited_first = app
            .import_evidence_from(
                &edited_first.id,
                EvidenceRole::SunoFinalExport,
                &edited_first_source,
            )
            .expect("import after editing answer");
        assert_eq!(edited_first.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(edited_first.fields.production_end_date, "2026-08-17");
        assert_eq!(edited_first.fields.suno_download_export_date, "2026-08-17");
        assert!(edited_first.fields.final_export_date.is_empty());
        assert_eq!(
            edited_first.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        let automated_first = p0_track(&app, "P0 Editing After Import", Some(false), false);
        let automated_source = directory.path().join("automated-first.wav");
        fs::write(&automated_source, p0_pcm_wav(Some(&raw))).expect("write second WAV");
        let automated_first = app
            .import_evidence_from(
                &automated_first.id,
                EvidenceRole::SunoFinalExport,
                &automated_source,
            )
            .expect("derive production end before editing answer changes");
        assert_eq!(automated_first.fields.production_end_date, "2026-08-17");
        assert_eq!(automated_first.fields.final_export_date, "2026-08-17");

        let editing_enabled = app
            .update_track(
                &automated_first.id,
                TrackPatch {
                    post_export_editing_performed: Some(true),
                    post_export_editing_details: Some("Mastering after export".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("record post-export editing");
        assert_eq!(editing_enabled.fields.production_end_date, "2026-08-17");
        assert!(editing_enabled.fields.final_export_date.is_empty());
        assert_eq!(
            editing_enabled.automation.final_export_origin,
            FactOrigin::NotDocumented
        );

        let editing_date = app
            .update_track(
                &automated_first.id,
                TrackPatch {
                    final_export_date: Some("2026-08-19".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("record the actual desktop editing date");
        assert_eq!(editing_date.fields.final_export_date, "2026-08-19");
        assert_eq!(
            editing_date.automation.final_export_origin,
            FactOrigin::UserConfirmedFact
        );
        assert_eq!(
            editing_enabled.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        let manual_end = app
            .update_track(
                &automated_first.id,
                TrackPatch {
                    production_end_date: Some("2026-08-20".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("attempt a manual production-end override");
        assert_eq!(manual_end.fields.production_end_date, "2026-08-17");
        assert_eq!(
            manual_end.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
    }

    #[test]
    fn p0_plain_wav_uses_manual_fallback_without_inventing_suno_values() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Plain WAV", Some(false), false);
        let source = directory.path().join("plain.wav");
        fs::write(&source, p0_pcm_wav(None)).expect("write plain WAV");

        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("plain WAV import remains valid");
        let evidence = p0_evidence(&imported, EvidenceRole::SunoFinalExport);

        assert_eq!(evidence.metadata.audio_format, "WAV");
        assert_eq!(evidence.metadata.audio_channels, Some(2));
        assert!(!evidence.metadata.suno_studio_detected);
        assert!(evidence.metadata.suno_created_timestamp.is_empty());
        assert!(evidence.metadata.suno_created_date.is_empty());
        assert!(evidence.metadata.suno_id.is_empty());
        assert!(evidence.metadata.suno_raw_metadata.is_empty());
        assert!(imported.fields.suno_final_generation_id.is_empty());
        assert!(imported.fields.suno_final_generation_date.is_empty());
        assert!(imported.fields.production_end_date.is_empty());
        assert_eq!(
            imported.automation.final_generation_origin,
            FactOrigin::NotDocumented
        );
        assert!(imported.automation.consistency_issues.is_empty());
    }

    #[test]
    fn p0_optional_control_metadata_is_ignored_without_losing_valid_suno_facts() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Optional Control", Some(false), false);
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");
        let source = directory.path().join("optional-control.wav");
        fs::write(
            &source,
            p0_pcm_wav_with_info_entries(&[
                (*b"IART", b"unsafe\x01artist".to_vec()),
                (*b"ICMT", raw.as_bytes().to_vec()),
            ]),
        )
        .expect("WAV with optional control metadata");

        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("safe metadata subset imports");
        let item = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
        assert_eq!(
            item.metadata.embedded_metadata,
            vec![EmbeddedMetadata {
                key: "ICMT".into(),
                value: raw.clone(),
            }]
        );
        assert_eq!(item.metadata.suno_raw_metadata, raw);
        assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(imported.fields.production_end_date, "2026-08-17");
    }

    #[test]
    fn p0_control_in_suno_comment_imports_wav_without_derived_facts() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Unsafe Suno Comment", Some(false), false);
        let unsafe_raw = format!(
            "made with suno studio; note=unsafe\x01text; created=2026-08-17T06:38:06Z; id={P0_SUNO_ID}"
        );
        let source = directory.path().join("unsafe-suno-comment.wav");
        fs::write(
            &source,
            p0_pcm_wav_with_info_entries(&[(*b"ICMT", unsafe_raw.into_bytes())]),
        )
        .expect("WAV with unsafe Suno comment");

        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("unsafe optional metadata does not reject WAV");
        let item = p0_evidence(&imported, EvidenceRole::SunoFinalExport);
        assert!(item.metadata.embedded_metadata.is_empty());
        assert!(!item.metadata.suno_studio_detected);
        assert!(item.metadata.suno_raw_metadata.is_empty());
        assert!(imported.fields.suno_final_generation_date.is_empty());
        assert!(imported.fields.production_end_date.is_empty());
    }

    #[test]
    fn p0_metadata_date_replaces_a_manual_fallback_without_a_conflict() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Manual Conflict", None, false);
        let manual = app
            .update_track(
                &track.id,
                TrackPatch {
                    suno_final_generation_date: Some("2026-08-16".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("manual generation date");
        let raw = p0_suno_comment("2026-08-17T06:38:06Z");
        let source = directory.path().join("conflicting.wav");
        fs::write(&source, p0_pcm_wav(Some(&raw))).expect("write conflicting WAV");

        let imported = app
            .import_evidence_from(&manual.id, EvidenceRole::SunoFinalExport, &source)
            .expect("import authoritative metadata");

        assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(imported.fields.production_end_date, "2026-08-17");
        assert_eq!(
            imported.automation.final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(
            imported.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert!(!imported
            .automation
            .consistency_issues
            .iter()
            .any(|issue| matches!(
                issue.code.as_str(),
                "suno_generation_date_conflict" | "production_end_date_conflict"
            )));

        let overridden = app
            .update_track(
                &manual.id,
                TrackPatch {
                    suno_final_generation_date: Some("2026-08-15".into()),
                    production_end_date: Some("2026-08-20".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("native reconciliation rejects submitted overrides");
        assert_eq!(overridden.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(overridden.fields.production_end_date, "2026-08-17");
        assert!(!overridden
            .missing_items
            .iter()
            .any(|item| item.contains("Suno-Erzeugungsdatum")));
    }

    #[test]
    fn p0_replacing_current_suno_export_updates_only_system_owned_values() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Replace Suno", Some(false), false);
        let track = app
            .update_track(
                &track.id,
                TrackPatch {
                    suno_download_export_date: Some("2026-08-21".into()),
                    final_export_date: Some("2026-08-22".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("manual export dates");
        let first_source = directory.path().join("replace-first.wav");
        fs::write(
            &first_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("first Suno WAV");
        let first = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
            .expect("first Suno import");
        let evidence_id = p0_evidence(&first, EvidenceRole::SunoFinalExport)
            .id
            .clone();
        assert_eq!(first.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(first.fields.production_end_date, "2026-08-17");

        let second_source = directory.path().join("replace-second.wav");
        fs::write(
            &second_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-18T06:38:06Z"))),
        )
        .expect("second Suno WAV");
        let second = app
            .replace_evidence_from(
                &track.id,
                &evidence_id,
                EvidenceRole::SunoFinalExport,
                &second_source,
            )
            .expect("replace current Suno export");
        assert_eq!(second.fields.suno_final_generation_date, "2026-08-18");
        assert_eq!(second.fields.production_end_date, "2026-08-18");
        assert_eq!(second.fields.suno_download_export_date, "2026-08-18");
        assert_eq!(second.fields.final_export_date, "2026-08-18");
        assert_eq!(
            second.automation.final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(
            second.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );

        let manual_end = app
            .update_track(
                &track.id,
                TrackPatch {
                    production_end_date: Some("2026-08-20".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("attempt manual production end override");
        assert_eq!(manual_end.fields.production_end_date, "2026-08-18");
        assert_eq!(
            manual_end.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        let third_source = directory.path().join("replace-third.wav");
        fs::write(
            &third_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-19T06:38:06Z"))),
        )
        .expect("third Suno WAV");
        let third = app
            .replace_evidence_from(
                &track.id,
                &evidence_id,
                EvidenceRole::SunoFinalExport,
                &third_source,
            )
            .expect("replace current Suno export again");

        assert_eq!(third.fields.suno_final_generation_date, "2026-08-19");
        assert_eq!(third.fields.production_end_date, "2026-08-19");
        assert_eq!(
            third.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(third.fields.suno_download_export_date, "2026-08-19");
        assert_eq!(third.fields.final_export_date, "2026-08-19");
        assert_eq!(
            third
                .evidence
                .iter()
                .filter(|item| item.role == EvidenceRole::SunoFinalExport)
                .count(),
            1
        );
        let current = p0_evidence(&third, EvidenceRole::SunoFinalExport);
        assert_eq!(current.id, evidence_id);
        assert_eq!(
            current.metadata.suno_created_timestamp,
            "2026-08-19T06:38:06Z"
        );
    }

    #[test]
    fn p0_removing_suno_export_clears_only_automatic_values() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Remove Automatic", Some(false), false);
        let track = app
            .update_track(
                &track.id,
                TrackPatch {
                    suno_download_export_date: Some("2026-08-21".into()),
                    final_export_date: Some("2026-08-22".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("manual non-derived dates");
        let source = directory.path().join("remove-automatic.wav");
        fs::write(
            &source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("Suno WAV");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("Suno import");
        let evidence_id = p0_evidence(&imported, EvidenceRole::SunoFinalExport)
            .id
            .clone();

        let removed = app
            .remove_evidence(&track.id, &evidence_id)
            .expect("remove current Suno export");
        assert!(removed.fields.suno_final_generation_id.is_empty());
        assert!(removed.fields.suno_final_generation_date.is_empty());
        assert!(removed.fields.production_end_date.is_empty());
        assert!(removed.fields.suno_download_export_date.is_empty());
        assert!(removed.fields.final_export_date.is_empty());
        assert_eq!(
            removed.automation.final_generation_id_origin,
            FactOrigin::NotDocumented
        );
        assert_eq!(
            removed.automation.final_generation_origin,
            FactOrigin::NotDocumented
        );
        assert_eq!(
            removed.automation.production_end_origin,
            FactOrigin::NotDocumented
        );
        assert_eq!(
            removed.automation.download_export_origin,
            FactOrigin::NotDocumented
        );
        assert_eq!(
            removed.automation.final_export_origin,
            FactOrigin::NotDocumented
        );

        let manual_track = p0_track(&app, "P0 Remove Preserves Manual", Some(false), false);
        let manual_source = directory.path().join("remove-manual.wav");
        fs::write(
            &manual_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("second Suno WAV");
        let manual_track = app
            .import_evidence_from(
                &manual_track.id,
                EvidenceRole::SunoFinalExport,
                &manual_source,
            )
            .expect("second Suno import");
        let manual_id = p0_evidence(&manual_track, EvidenceRole::SunoFinalExport)
            .id
            .clone();
        let removed_manual = app
            .remove_evidence(&manual_track.id, &manual_id)
            .expect("remove export before recording a fallback");
        assert!(removed_manual.fields.suno_final_generation_date.is_empty());
        assert!(removed_manual.fields.production_end_date.is_empty());
        let manual_fallback = app
            .update_track(
                &manual_track.id,
                TrackPatch {
                    suno_final_generation_date: Some("2026-08-17".into()),
                    production_end_date: Some("2026-08-20".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("record manual fallbacks without metadata");
        assert_eq!(
            manual_fallback.fields.suno_final_generation_date,
            "2026-08-17"
        );
        assert_eq!(manual_fallback.fields.production_end_date, "2026-08-20");
        assert_eq!(
            manual_fallback.automation.final_generation_origin,
            FactOrigin::UserConfirmedFact
        );
        assert_eq!(
            manual_fallback.automation.production_end_origin,
            FactOrigin::UserConfirmedFact
        );
    }

    #[test]
    fn p0_metadata_derived_generation_date_feeds_subscription_coverage() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "P0 Subscription Coverage", Some(false), true);
        let source = directory.path().join("coverage-suno.wav");
        fs::write(
            &source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("coverage Suno WAV");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("metadata-derived generation date");
        assert_eq!(imported.fields.suno_final_generation_date, "2026-08-17");

        let receipt = directory.path().join("coverage-subscription.pdf");
        fs::write(&receipt, b"%PDF-1.7\nsubscription receipt\n%%EOF\n").expect("subscription PDF");
        let global = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &receipt,
                Some("2026-08-01".into()),
                Some("2026-08-31".into()),
            )
            .expect("global subscription evidence");
        let attached = app
            .attach_global_evidence(&track.id, &global.evidence.id)
            .expect("attach covering subscription evidence");
        let stored = app.persistence.track(&track.id).expect("stored track");

        assert_eq!(
            workflow::subscription_generation_coverage(&stored, &attached.evidence),
            CoverageStatus::Yes
        );
        assert_eq!(
            stored.field_origins.suno_final_generation_date,
            Some(EvidenceDerivedField {
                value: "2026-08-17".into(),
                original_value: "2026-08-17T06:38:06Z".into(),
                evidence_id: p0_evidence(&attached, EvidenceRole::SunoFinalExport)
                    .id
                    .clone(),
                evidence_sha256: p0_evidence(&attached, EvidenceRole::SunoFinalExport)
                    .sha256
                    .clone()
                    .expect("Suno SHA-256"),
            })
        );
    }

    #[test]
    fn p0_finalized_pre_metadata_record_is_not_backfilled_on_load() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let fixture_root = directory.path().join("fixtures");
        let ready = prepare_ready_track(&app, &fixture_root, "P0 Frozen Legacy Metadata");
        let existing = p0_evidence(&ready, EvidenceRole::SunoFinalExport).clone();
        let source = fixture_root.join("suno-export.wav");
        fs::write(
            &source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-02T06:38:06Z"))),
        )
        .expect("real Suno WAV bytes");
        let replaced = app
            .replace_evidence_from(
                &ready.id,
                &existing.id,
                EvidenceRole::SunoFinalExport,
                &source,
            )
            .expect("replace fixture with real WAV");
        assert!(
            p0_evidence(&replaced, EvidenceRole::SunoFinalExport)
                .metadata
                .suno_studio_detected
        );
        app.update_track(
            &ready.id,
            TrackPatch {
                suno_export_filename_difference_confirmed: Some(true),
                ..TrackPatch::default()
            },
        )
        .expect("reconfirm fixture filename");

        // Simulate a pre-feature evidence JSON row: the managed file already
        // contains Suno metadata, while the persisted additive fields do not.
        let mut legacy_evidence = app
            .persistence
            .evidence_item(&ready.id, &existing.id)
            .expect("stored Suno evidence");
        let original_file_name = legacy_evidence.metadata.original_file_name.clone();
        legacy_evidence.metadata = EvidenceMetadata {
            original_file_name,
            ..EvidenceMetadata::default()
        };
        app.persistence
            .save_evidence(&ready.id, &legacy_evidence)
            .expect("seed pre-metadata evidence row");
        let mut legacy_track = app.persistence.track(&ready.id).expect("stored track");
        legacy_track.field_origins = Default::default();
        app.persistence
            .save_track(&legacy_track)
            .expect("seed pre-metadata track row");

        app.generate_documents(&ready.id, false)
            .expect("legacy-compatible documents");
        app.calculate_hashes(&ready.id)
            .expect("legacy-compatible hashes");
        let validation = app
            .validate_track(&ready.id)
            .expect("legacy-compatible gate");
        assert!(
            validation.valid,
            "missing={:?}; blocking={:?}",
            validation.missing_items, validation.blocking_items
        );
        let finalized = app
            .finalize_track(&ready.id)
            .expect("finalize pre-metadata record")
            .track
            .expect("finalized detail");
        let track_root = app.root().join(&finalized.relative_path);
        let certificate_before = certificate_file_snapshot(&track_root);

        // Simulate exact pre-feature JSON shapes. This catches a load path
        // that deserializes additive defaults and needlessly writes the
        // expanded current structures back.
        let connection = app.persistence.open().expect("raw workspace database");
        connection
            .execute(
                "UPDATE evidence SET metadata_json='{}' WHERE track_id=?1 AND id=?2",
                rusqlite::params![ready.id, existing.id],
            )
            .expect("seed exact legacy metadata JSON");
        let stored_track_json: String = connection
            .query_row(
                "SELECT data_json FROM tracks WHERE id=?1",
                [ready.id.as_str()],
                |row| row.get(0),
            )
            .expect("read current track JSON");
        let mut legacy_track_json: serde_json::Value =
            serde_json::from_str(&stored_track_json).expect("parse current track JSON");
        legacy_track_json
            .as_object_mut()
            .expect("track JSON object")
            .remove("fieldOrigins");
        connection
            .execute(
                "UPDATE tracks SET data_json=?1 WHERE id=?2",
                rusqlite::params![legacy_track_json.to_string(), ready.id],
            )
            .expect("seed exact legacy track JSON");
        drop(connection);
        let record_before = app.persistence.track(&ready.id).expect("frozen record");
        let metadata_before = app
            .persistence
            .evidence_item(&ready.id, &existing.id)
            .expect("frozen evidence")
            .metadata;
        assert!(!metadata_before.suno_studio_detected);
        assert!(metadata_before.suno_created_timestamp.is_empty());
        let raw_before = p0_raw_finalized_rows(&app, &ready.id, &existing.id);
        assert_eq!(raw_before.evidence_metadata_json, "{}");
        assert!(!raw_before.track_data_json.contains("fieldOrigins"));

        let loaded = app.load_track(&ready.id).expect("load finalized record");
        let metadata_after = p0_evidence(&loaded, EvidenceRole::SunoFinalExport)
            .metadata
            .clone();
        let record_after = app.persistence.track(&ready.id).expect("record after load");
        let raw_after = p0_raw_finalized_rows(&app, &ready.id, &existing.id);

        assert_eq!(loaded.status, TrackStatus::Finalized);
        assert!(loaded.certificate.valid);
        assert_eq!(metadata_after, metadata_before);
        assert_eq!(raw_after, raw_before);
        assert_eq!(record_after.updated_at, record_before.updated_at);
        assert_eq!(record_after.fields, record_before.fields);
        assert_eq!(record_after.field_origins, record_before.field_origins);
        assert_eq!(certificate_file_snapshot(&track_root), certificate_before);

        app.list_tracks().expect("list finalized legacy track");
        assert_eq!(
            p0_raw_finalized_rows(&app, &ready.id, &existing.id),
            raw_before
        );
        drop(app);

        let reopened = WorkspaceApp::open(&directory.path().join("workspace"), false)
            .expect("reopen workspace");
        let reopened_track = reopened
            .load_track(&ready.id)
            .expect("load finalized track after reopen");
        assert_eq!(reopened_track.status, TrackStatus::Finalized);
        assert_eq!(
            p0_raw_finalized_rows(&reopened, &ready.id, &existing.id),
            raw_before
        );
        assert_eq!(certificate_file_snapshot(&track_root), certificate_before);

        let revision = reopened
            .create_revision(&ready.id)
            .expect("explicit mutable revision")
            .track
            .expect("revision detail");
        let analyzed = p0_evidence(&revision, EvidenceRole::SunoFinalExport);
        assert_eq!(revision.status, TrackStatus::Active);
        assert!(analyzed.metadata.suno_studio_detected);
        assert_eq!(
            analyzed.metadata.suno_created_timestamp,
            "2026-08-02T06:38:06Z"
        );
    }

    #[test]
    fn evidence_import_rolls_back_file_and_database_when_track_commit_fails() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "Atomic Import", Some(false), false);
        let source = directory.path().join("atomic-import.wav");
        fs::write(
            &source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("atomic import WAV");
        let relative = evidence::managed_relative_path(
            &track.fields.title,
            &EvidenceRole::SunoFinalExport,
            &source,
        )
        .expect("planned managed path");
        let track_before = serde_json::to_value(
            app.persistence
                .track(&track.id)
                .expect("track before import"),
        )
        .expect("serialize track before import");
        app.persistence
            .open()
            .expect("database")
            .execute_batch(
                "CREATE TRIGGER injected_track_save_failure BEFORE UPDATE ON tracks
                 BEGIN SELECT RAISE(ABORT, 'injected track save failure'); END;",
            )
            .expect("failure trigger");

        assert!(matches!(
            app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source),
            Err(AppError::Database(_))
        ));
        assert!(app
            .persistence
            .evidence(&track.id)
            .expect("evidence after failed import")
            .is_empty());
        assert_eq!(
            serde_json::to_value(
                app.persistence
                    .track(&track.id)
                    .expect("track after import")
            )
            .expect("serialize track after import"),
            track_before
        );
        assert!(!app
            .root()
            .join(&track.relative_path)
            .join(relative)
            .exists());
    }

    #[test]
    fn evidence_replace_rolls_back_bytes_evidence_and_track_together() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "Atomic Replace", Some(false), false);
        let first_source = directory.path().join("atomic-first.wav");
        fs::write(
            &first_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("first replacement WAV");
        let imported = app
            .import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &first_source)
            .expect("initial import");
        let previous = p0_evidence(&imported, EvidenceRole::SunoFinalExport).clone();
        let track_before = serde_json::to_value(
            app.persistence
                .track(&track.id)
                .expect("track before replace"),
        )
        .expect("serialize track before replace");
        let evidence_before = serde_json::to_value(
            app.persistence
                .evidence_item(&track.id, &previous.id)
                .expect("evidence before replace"),
        )
        .expect("serialize evidence before replace");
        let track_root = app.root().join(&track.relative_path);
        let previous_path = track_root.join(&previous.relative_path);
        let previous_bytes = fs::read(&previous_path).expect("previous managed bytes");
        let second_source = directory.path().join("atomic-second.wav");
        fs::write(
            &second_source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-18T06:38:06Z"))),
        )
        .expect("second replacement WAV");
        let second_relative = evidence::managed_relative_path(
            &track.fields.title,
            &EvidenceRole::SunoFinalExport,
            &second_source,
        )
        .expect("second managed path");
        app.persistence
            .open()
            .expect("database")
            .execute_batch(
                "CREATE TRIGGER injected_track_save_failure BEFORE UPDATE ON tracks
                 BEGIN SELECT RAISE(ABORT, 'injected track save failure'); END;",
            )
            .expect("failure trigger");

        assert!(matches!(
            app.replace_evidence_from(
                &track.id,
                &previous.id,
                EvidenceRole::SunoFinalExport,
                &second_source,
            ),
            Err(AppError::Database(_))
        ));
        assert_eq!(
            serde_json::to_value(
                app.persistence
                    .track(&track.id)
                    .expect("track after replace")
            )
            .expect("serialize track after replace"),
            track_before
        );
        assert_eq!(
            serde_json::to_value(
                app.persistence
                    .evidence_item(&track.id, &previous.id)
                    .expect("evidence after replace")
            )
            .expect("serialize evidence after replace"),
            evidence_before
        );
        assert_eq!(
            fs::read(&previous_path).expect("restored managed bytes"),
            previous_bytes
        );
        assert!(!track_root.join(second_relative).exists());
    }

    #[test]
    fn active_v15_reevaluation_adopts_authoritative_metadata_dates() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let track = p0_track(&app, "Active 1.5 Metadata Upgrade", None, false);
        let source = directory.path().join("active-v15.wav");
        fs::write(
            &source,
            p0_pcm_wav(Some(&p0_suno_comment("2026-08-17T06:38:06Z"))),
        )
        .expect("Suno WAV");
        app.import_evidence_from(&track.id, EvidenceRole::SunoFinalExport, &source)
            .expect("initial metadata import");

        let mut stale = app.persistence.track(&track.id).expect("stored track");
        stale.workflow_version = "1.5".into();
        stale.fields.suno_final_generation_date = "2026-08-16".into();
        stale.fields.production_end_date = "2026-08-18".into();
        stale.field_origins = Default::default();
        app.persistence
            .save_track(&stale)
            .expect("seed 1.5 fallback dates");

        let upgraded = app
            .re_evaluate_track(&track.id)
            .expect("explicit 1.8 reevaluation")
            .track
            .expect("reevaluated track");

        assert_eq!(upgraded.workflow_version, "1.8");
        assert_eq!(upgraded.fields.suno_final_generation_date, "2026-08-17");
        assert_eq!(upgraded.fields.production_end_date, "2026-08-17");
        assert_eq!(
            upgraded.automation.final_generation_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
        assert_eq!(
            upgraded.automation.production_end_origin,
            FactOrigin::EvidenceDerivedMetadata
        );
    }

    #[test]
    fn outdated_workflow_blocks_new_outputs_until_explicit_reevaluation() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let created = p0_track(&app, "Outdated Workflow", None, false);
        let mut stale = app.persistence.track(&created.id).expect("stored track");
        stale.workflow_version = "1.3".into();
        stale.fields.production_end_date = "2026-08-01".into();
        app.persistence
            .save_track(&stale)
            .expect("seed old workflow");

        let validation = app.validate_track(&created.id).expect("validation result");
        assert!(!validation.valid);
        assert!(validation
            .blocking_items
            .iter()
            .any(|item| item.contains("Re-evaluate the track explicitly")));
        for result in [
            app.generate_documents(&created.id, false),
            app.calculate_hashes(&created.id),
            app.finalize_track(&created.id),
        ] {
            assert!(matches!(
                result,
                Err(AppError::Validation(message))
                    if message.contains("Re-evaluate the track explicitly")
            ));
        }

        let upgraded = app
            .re_evaluate_track(&created.id)
            .expect("explicit reevaluation")
            .track
            .expect("reevaluated track");
        assert_eq!(upgraded.workflow_version, "1.8");
    }

    #[test]
    fn superseded_tracks_reject_content_and_workflow_mutations() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        let created = p0_track(&app, "Superseded Snapshot", None, false);
        let mut superseded = app.persistence.track(&created.id).expect("stored track");
        superseded.status = TrackStatus::Superseded;
        superseded.workflow_version = "1.3".into();
        app.persistence
            .save_track(&superseded)
            .expect("seed superseded snapshot");

        assert!(matches!(
            app.update_track(
                &created.id,
                TrackPatch {
                    title: Some("Changed title".into()),
                    ..TrackPatch::default()
                },
            ),
            Err(AppError::Finalized)
        ));
        assert!(matches!(
            app.update_track_library(&created.id, TrackLibraryPlacement::default()),
            Err(AppError::Finalized)
        ));
        assert!(matches!(
            app.import_evidence_from(
                &created.id,
                EvidenceRole::SunoFinalExport,
                directory.path().join("unused.wav").as_path(),
            ),
            Err(AppError::Finalized)
        ));
        assert!(matches!(
            app.re_evaluate_track(&created.id),
            Err(AppError::Finalized)
        ));
    }

    #[test]
    fn revision_with_missing_suno_bytes_is_created_without_partial_failure() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized = finalize_acceptance_track(
            &app,
            &directory.path().join("fixtures"),
            "Missing Suno Revision",
        );
        let suno = p0_evidence(&finalized, EvidenceRole::SunoFinalExport).clone();
        let track_root = app.root().join(&finalized.relative_path);
        fs::remove_file(track_root.join(&suno.relative_path)).expect("remove Suno bytes");

        let revision = app
            .create_revision(&finalized.id)
            .expect("revision despite missing Suno bytes")
            .track
            .expect("active revision");
        assert_eq!(revision.status, TrackStatus::Active);
        assert!(revision
            .evidence
            .iter()
            .any(|item| item.id == suno.id && !item.verified));
        assert_eq!(
            fs::read_dir(track_root.join(".archive/revisions"))
                .expect("revision archive")
                .count(),
            1
        );
    }
}
