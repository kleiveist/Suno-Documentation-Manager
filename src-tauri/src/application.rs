use crate::certificate;
use crate::documents;
use crate::error::{AppError, Result};
use crate::evidence;
use crate::integrity;
use crate::model::{
    ActionResult, BlockingDeviation, CertificateState, CreateTrackInput, DeviationInput,
    DocumentPreview, DocumentState, EvidenceItem, EvidenceMetadata, EvidencePreview,
    EvidenceProvenance, EvidenceRole, GlobalEvidenceItem, IntegrityState, LegacyCandidate,
    OperationProgress, Profile, StepState, StepStatus, SubscriptionBillingCycle, TrackCoverPreview,
    TrackDetail, TrackLibraryPlacement, TrackLibrarySection, TrackPatch, TrackRecord, TrackStatus,
    TrackSummary, ValidationResult, WorkspaceScan, WorkspaceSummary,
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

#[derive(Debug, Clone)]
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
        let mut tracks = self.persistence.tracks()?;
        let updated_at = now();
        let mut changed = false;
        for track in &mut tracks {
            if matches!(
                track.status,
                TrackStatus::Finalized | TrackStatus::Superseded
            ) || track.profile_snapshot == *profile
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
        for track in self.persistence.tracks()? {
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
            track_count: self.persistence.tracks()?.len() as u32,
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
            suno_plan_at_creation: profile.suno_plan.clone(),
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
            fields,
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
        self.detail_from_record(track, false)
    }

    pub fn list_tracks(&self) -> Result<Vec<TrackSummary>> {
        self.reconcile_physical_library()?;
        let mut result = Vec::new();
        for track in self.persistence.tracks()? {
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
            if title == ".suno-doc" || title == SINGLES_DIRECTORY {
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
        self.detail_from_record(self.persistence.track(id)?, true)
    }

    pub fn update_track(&self, id: &str, patch: TrackPatch) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let previous_path = track.relative_path.clone();
        let previous_fields = track.fields.clone();
        apply_patch(&mut track.fields, patch);
        if track.fields.title != previous_fields.title {
            track.fields.release_filename_difference_confirmed = None;
            track.fields.suno_export_filename_difference_confirmed = None;
        }
        validate_track_fields(&track.fields)?;
        if track.fields != previous_fields {
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
        self.detail_from_record(track, false)
    }

    pub fn update_track_library(
        &self,
        id: &str,
        input: TrackLibraryPlacement,
    ) -> Result<TrackDetail> {
        self.reconcile_physical_library()?;
        let mut track = self.persistence.track(id)?;
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
        validate_required_production_range(&track)?;
        let global = self.persistence.global_evidence_item(evidence_id)?;
        if global.evidence.role != EvidenceRole::SubscriptionPayment {
            return Err(AppError::Validation(
                "Only subscription/payment evidence can satisfy the track subscription requirement."
                    .into(),
            ));
        }
        let start =
            global.evidence.coverage_start.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage start.".into())
            })?;
        let end =
            global.evidence.coverage_end.as_deref().ok_or_else(|| {
                AppError::Validation("Global evidence has no coverage end.".into())
            })?;
        validate_date_range("Subscription coverage", start, end)?;
        if start > track.fields.production_start_date.as_str()
            || end < track.fields.production_end_date.as_str()
        {
            return Err(AppError::Validation(
                "The selected subscription evidence does not cover the full production period."
                    .into(),
            ));
        }
        if track.fields.commercial_use_intended
            && !track.fields.suno_final_generation_date.trim().is_empty()
            && (start > track.fields.suno_final_generation_date.as_str()
                || end < track.fields.suno_final_generation_date.as_str())
        {
            return Err(AppError::Validation(
                "The selected subscription evidence does not cover the recorded final-generation date."
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
        mark_content_changed(&mut track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

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
        if role == EvidenceRole::SubscriptionPayment {
            return Err(AppError::Validation(
                "Register subscription evidence globally and attach a covering portable copy."
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
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        validate_evidence_metadata(&role, &metadata)?;
        let mut item = evidence::import(&track_root, &track.fields.title, role.clone(), source)?;
        item.metadata = metadata;
        if let Err(error) = self.persistence.save_evidence(id, &item) {
            if let Ok(path) = contained_path(&track_root, Path::new(&item.relative_path), true) {
                let _ = fs::remove_file(path);
            }
            return Err(error);
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
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

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
        mut metadata: EvidenceMetadata,
    ) -> Result<TrackDetail> {
        if role == EvidenceRole::SubscriptionPayment {
            return Err(AppError::Validation(
                "Replace subscription evidence in the global evidence register.".into(),
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
        metadata.original_file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_owned();
        validate_evidence_metadata(&role, &metadata)?;
        evidence::replace(
            &track_root,
            &track.fields.title,
            role,
            source,
            &previous,
            |item| {
                let mut item = item.clone();
                item.metadata = metadata.clone();
                self.persistence.save_evidence(id, &item)
            },
        )?;
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
        self.persistence.save_track(&track)?;
        self.detail_from_record(track, false)
    }

    pub fn remove_evidence(&self, id: &str, evidence_id: &str) -> Result<TrackDetail> {
        let mut track = self.mutable_track(id)?;
        let item = self.persistence.evidence_item(id, evidence_id)?;
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
        for item in items {
            if evidence_id.is_none() || evidence_id == Some(item.id.as_str()) {
                let verified = evidence::verify(&track_root, item)?;
                mismatch |= !verified.verified;
                self.persistence.save_evidence(id, &verified)?;
            }
        }
        if mismatch && track.status == TrackStatus::Finalized {
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
        validate_track_fields(&track.fields)?;
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
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
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
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

    pub fn validate_track(&self, id: &str) -> Result<ValidationResult> {
        let track = self.persistence.track(id)?;
        self.validation_for(track)
    }

    #[cfg(test)]
    pub fn finalize_track(&self, id: &str) -> Result<ActionResult> {
        self.finalize_track_with_progress(id, &mut |_| {})
    }

    pub fn finalize_track_with_progress(
        &self,
        id: &str,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        self.finalize_track_impl(
            id,
            #[cfg(test)]
            None,
            on_progress,
        )
    }

    fn finalize_track_impl(
        &self,
        id: &str,
        #[cfg(test)] failure: Option<FinalizationFailure>,
        on_progress: &mut impl FnMut(OperationProgress),
    ) -> Result<ActionResult> {
        on_progress(OperationProgress {
            stage: "validating_finalization_gate".into(),
            ..OperationProgress::default()
        });
        let mut track = self.mutable_track(id)?;
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
        let evidence = self.verified_evidence(&track)?;
        let deviations = self.persistence.deviations(id)?;
        let stored = self.persistence.stored_steps(id)?;
        let evaluation = workflow::evaluate(
            &track,
            &track.profile_snapshot,
            &evidence,
            &deviations,
            &stored,
        )?;
        let finalized_at = now();
        let certificate_id = format!("SDM-{}", Uuid::new_v4());
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
                &track,
                &track.profile_snapshot,
                &evaluation.steps,
                &evidence,
                &deviations,
                &certificate_id,
                &finalized_at,
                &transaction_id,
                certificate_failure,
            ),
            None => certificate::generate(
                &track_root,
                &track,
                &track.profile_snapshot,
                &evaluation.steps,
                &evidence,
                &deviations,
                &certificate_id,
                &finalized_at,
                &transaction_id,
            ),
        };
        #[cfg(not(test))]
        let publication = certificate::generate(
            &track_root,
            &track,
            &track.profile_snapshot,
            &evaluation.steps,
            &evidence,
            &deviations,
            &certificate_id,
            &finalized_at,
            &transaction_id,
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
            finalized_at: Some(finalized_at),
            workflow_version: Some(track.workflow_version.clone()),
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
        if let Err(error) =
            ensure_contained_directory(&root, Path::new(certificate::CERTIFICATE_DIR))
        {
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
        track.updated_at = now();
        if let Err(error) = self.persistence.save_track(&track) {
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
        Ok(ActionResult {
            message: format!("Previous certificate archived as revision {revision_id}."),
            track: Some(self.detail_from_record(track, false)?),
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
            track: Some(self.detail_from_record(track, false)?),
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
                    fields,
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
        if track.status == TrackStatus::Finalized {
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
                    EvidenceRole::ReleaseWav | EvidenceRole::FinalArtwork
                )
            })
            .map(|item| item.role)
            .collect::<HashSet<_>>();
        for file in files.iter().filter(|file| !indexed.contains(*file)) {
            let inferred_role = infer_legacy_role(file);
            let ambiguous_singular = matches!(
                inferred_role,
                EvidenceRole::ReleaseWav | EvidenceRole::FinalArtwork
            ) && !singular_roles.insert(inferred_role.clone());
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
        let tracks = self.persistence.tracks()?;
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
            let verified = evidence::inspect(&root, item)?;
            self.persistence.save_evidence(&track.id, &verified)?;
            result.push(verified);
        }
        Ok(result)
    }

    fn validation_for(&self, mut track: TrackRecord) -> Result<ValidationResult> {
        validate_profile(&track.profile_snapshot, true)?;
        validate_track_fields(&track.fields)?;
        validate_required_production_range(&track)?;
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
        if self.migrate_legacy_release_evidence(&track)? {
            mark_content_changed(&mut track);
            track.updated_at = now();
            self.persistence.save_track(&track)?;
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
        track.documents.current = documents::is_current(
            &root,
            &track,
            &track.profile_snapshot,
            &evidence,
            &first.steps,
        )?;
        let contains_large_evidence = evidence
            .iter()
            .any(|item| item.size_bytes > evidence::AUTOMATIC_HASH_LIMIT_BYTES);
        if root.join(integrity::HASH_FILE).is_file() && !contains_large_evidence {
            track.integrity = integrity::verify(&root).unwrap_or_else(|_| IntegrityState {
                generated: true,
                verified: false,
                file_count: 0,
                verified_count: 0,
                generated_at: None,
                verified_at: Some(now()),
                mismatch_files: vec![
                    "03_DOCUMENTATION/SHA256SUMS.txt (invalid or unreadable)".into()
                ],
            });
        } else if !root.join(integrity::HASH_FILE).is_file() {
            track.integrity = IntegrityState::default();
        }
        if inspect_finalized && track.status == TrackStatus::Finalized {
            let certificate_valid = certificate::verify(&root).is_ok();
            let evidence_valid = evidence.iter().all(|item| {
                item.verified && item.sha256.is_some() && item.verification_error.is_none()
            });
            if !track.integrity.verified || !certificate_valid || !evidence_valid {
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
        self.persistence.save_track(&track)?;
        let cover_evidence_id = final_artwork_evidence_id(&evidence);
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
            fields: track.fields.clone(),
            steps: evaluation.steps,
            evidence,
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
            fields: track.fields.clone(),
            steps: evaluation.steps,
            evidence,
            documents: track.documents.clone(),
            integrity: track.integrity.clone(),
            certificate: track.certificate.clone(),
            blocking_deviations: deviations,
            missing_items: evaluation.missing,
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

fn invalidate_state(track: &mut TrackRecord, reason: &str) {
    track.certificate.valid = false;
    track.certificate.invalidated_at = Some(now());
    track.certificate.invalidation_reason = Some(reason.into());
}

fn mark_content_changed(track: &mut TrackRecord) {
    track.documents.current = false;
    track.integrity = IntegrityState::default();
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
            "Suno project/version ID",
            fields.suno_project_version_id.as_str(),
            500,
        ),
        (
            "Suno final generation ID",
            fields.suno_final_generation_id.as_str(),
            500,
        ),
        (
            "Suno plan at creation",
            fields.suno_plan_at_creation.as_str(),
            200,
        ),
        ("AI image service", fields.ai_image_service.as_str(), 500),
        ("Disclosure text", fields.disclosure_text.as_str(), 80),
    ] {
        validate_short_text(name, value, max, false)?;
    }
    for (name, value, max) in [
        ("Lyrics", fields.lyrics_text.as_str(), 1_000_000),
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
    ] {
        validate_text_list(name, values, 100, 4_000)?;
    }
    for (name, value) in [
        ("Production start", fields.production_start_date.as_str()),
        ("Production end", fields.production_end_date.as_str()),
        ("Final export date", fields.final_export_date.as_str()),
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
        let export = parse_date("Final export date", &fields.final_export_date)?;
        if export < start {
            return Err(AppError::Validation(
                "Final export date cannot be before production start.".into(),
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
    if !fields.suno_final_generation_time.trim().is_empty()
        && chrono::NaiveTime::parse_from_str(fields.suno_final_generation_time.trim(), "%H:%M")
            .is_err()
    {
        return Err(AppError::Validation(
            "Final generation time must use HH:MM.".into(),
        ));
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
        ("Timestamp value", metadata.external_timestamp.as_str(), 200),
        ("Referenced hash", metadata.referenced_hash.as_str(), 512),
        (
            "Referenced artifact",
            metadata.referenced_artifact.as_str(),
            4000,
        ),
    ] {
        validate_multiline_text(name, value, max)?;
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
    match role {
        EvidenceRole::SunoTermsRights => {
            if metadata.document_title.trim().is_empty()
                || metadata.provider.trim().is_empty()
                || metadata.source_url.trim().is_empty()
                || metadata.retrieval_date.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "Suno terms evidence requires document title, provider/source, source URL, and retrieval date.".into(),
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
        || matches!(title, "." | "..")
        || title.eq_ignore_ascii_case(SINGLES_DIRECTORY)
        || title.eq_ignore_ascii_case(".suno-doc")
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
    if title.contains(['/', '\\']) || title.chars().any(char::is_control) {
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

fn discover_track_identities(root: &Path) -> Result<HashMap<String, String>> {
    let mut identities = HashMap::new();
    for entry in WalkDir::new(root)
        .min_depth(3)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
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
        if name == ".suno-doc" {
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

fn apply_patch(fields: &mut crate::model::TrackFields, patch: TrackPatch) {
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
    if let Some(value) = patch.suno_plan_at_creation {
        fields.suno_plan_at_creation = value;
    }
    if let Some(value) = patch.final_export_date {
        fields.final_export_date = value;
    }
    if let Some(value) = patch.instrumental_track {
        fields.instrumental_track = Some(value);
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
    fields.normalize_conditionals();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
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
        }
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
                    production_end_date: Some("2026-08-02".into()),
                    suno_model: Some("v4.5".into()),
                    suno_project_url: Some("https://suno.com/song/acceptance-track".into()),
                    suno_project_version_id: Some("acceptance-project-version".into()),
                    suno_final_generation_id: Some("acceptance-generation".into()),
                    suno_final_generation_date: Some("2026-08-02".into()),
                    suno_download_export_date: Some("2026-08-03".into()),
                    suno_plan_at_creation: Some("Pro".into()),
                    final_export_date: Some("2026-08-03".into()),
                    instrumental_track: Some(true),
                    lyrics_source: Some("instrumental".into()),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(false),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(false),
                    commercial_use_intended: Some(false),
                    artwork_origin: Some("none".into()),
                    ..TrackPatch::default()
                },
            )
            .expect("complete track facts");
        fs::create_dir_all(fixture_root).expect("fixture directory");
        let suno_export = fixture_root.join("suno-export.wav");
        let release_master = fixture_root.join("release-master.wav");
        fs::write(&suno_export, b"RIFF\x08\0\0\0WAVEsuno evidence").expect("Suno fixture");
        fs::write(&release_master, b"RIFF\x08\0\0\0WAVErelease evidence").expect("release fixture");
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
    fn terms_and_timestamp_imports_persist_factual_metadata_and_clear_unavailable_status() {
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

        let terms_source = directory.path().join("suno-terms-2026-08-01.md");
        fs::write(&terms_source, b"# Archived local terms fixture\n").expect("terms fixture");
        let terms = app
            .import_evidence_with_metadata_from(
                &track.id,
                EvidenceRole::SunoTermsRights,
                &terms_source,
                EvidenceMetadata {
                    document_title: "Suno Terms of Service".into(),
                    provider: "Suno".into(),
                    source_url: "https://suno.example/terms".into(),
                    retrieval_date: "2026-08-01".into(),
                    effective_date: "2026-07-15".into(),
                    factual_note: "Archived locally for the recorded production period.".into(),
                    ..EvidenceMetadata::default()
                },
            )
            .expect("terms import");
        assert_eq!(terms.fields.suno_terms_evidence_not_available, Some(false));
        let terms_item = terms
            .evidence
            .iter()
            .find(|item| item.role == EvidenceRole::SunoTermsRights)
            .expect("terms evidence");
        assert_eq!(
            terms_item.metadata.original_file_name,
            "suno-terms-2026-08-01.md"
        );
        assert_eq!(terms_item.metadata.provider, "Suno");

        let timestamp_source = directory.path().join("external-timestamp.json");
        fs::write(&timestamp_source, b"{\"timestamp\":\"fixture\"}\n").expect("timestamp fixture");
        let digest = "a".repeat(64);
        let timestamp = app
            .import_evidence_with_metadata_from(
                &track.id,
                EvidenceRole::ExternalTimestamp,
                &timestamp_source,
                EvidenceMetadata {
                    provider: "Example timestamp issuer".into(),
                    external_timestamp: "2026-08-02T12:00:00Z".into(),
                    referenced_hash: digest.clone(),
                    referenced_artifact: "06_CERTIFICATE/EVIDENCE_MANIFEST.json".into(),
                    ..EvidenceMetadata::default()
                },
            )
            .expect("timestamp import");
        assert!(timestamp.evidence.iter().any(|item| {
            item.role == EvidenceRole::ExternalTimestamp
                && item.metadata.referenced_hash == digest
                && item.metadata.original_file_name == "external-timestamp.json"
        }));

        let invalid_timestamp_source = directory.path().join("invalid-timestamp.json");
        fs::write(&invalid_timestamp_source, b"{}\n").expect("invalid timestamp fixture");
        let invalid = app.import_evidence_with_metadata_from(
            &track.id,
            EvidenceRole::ExternalTimestamp,
            &invalid_timestamp_source,
            EvidenceMetadata {
                provider: "Example timestamp issuer".into(),
                external_timestamp: "2026-08-02T12:00:00Z".into(),
                referenced_hash: "not-a-sha256".into(),
                referenced_artifact: "EVIDENCE_MANIFEST.json".into(),
                ..EvidenceMetadata::default()
            },
        );
        assert!(invalid
            .expect_err("invalid referenced hash")
            .to_string()
            .contains("SHA-256"));

        drop(app);
        let reopened = WorkspaceApp::open(&workspace, false).expect("reopened workspace");
        let evidence = reopened
            .load_track(&track.id)
            .expect("reopened track")
            .evidence;
        assert!(evidence.iter().any(|item| {
            item.role == EvidenceRole::SunoTermsRights
                && item.metadata.document_title == "Suno Terms of Service"
        }));
        assert!(evidence.iter().any(|item| {
            item.role == EvidenceRole::ExternalTimestamp
                && item.metadata.external_timestamp == "2026-08-02T12:00:00Z"
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

        for value in [
            &fields.lyrics_text,
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

        let before = fields.clone();
        apply_patch(
            &mut fields,
            TrackPatch {
                lyrics_text: Some("ignored hidden value".into()),
                real_person_notes: Some("ignored hidden note".into()),
                ..TrackPatch::default()
            },
        );
        assert_eq!(
            fields, before,
            "inactive-only patches must be semantic no-ops"
        );
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
                "## I. Evidence register" => Section::Other,
                "## J. Integrity anchors and workflow" => Section::Fields,
                "### Mandatory steps completed" => Section::CompletedSteps,
                "### N/A steps with reasons" => Section::NaReasons,
                "## Certificate statement" => Section::Other,
                _ => section,
            };
            if !line.starts_with("- ") {
                continue;
            }
            let entry = &line[2..];
            match section {
                Section::Fields => {
                    let (key, value) = entry
                        .split_once(": ")
                        .unwrap_or_else(|| panic!("malformed certificate field: {line}"));
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
                Section::NaReasons if entry != "None" => {
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
        assert_eq!(crate::persistence::SCHEMA_VERSION, 3);
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
                suno_plan_at_creation: Some("Historical Founder Plan".into()),
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
            assisted.fields.suno_plan_at_creation,
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
    fn older_track_json_defaults_to_single_library_section() {
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
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&materialized_json)
                .expect("materialized JSON")
                .pointer("/library/section")
                .and_then(serde_json::Value::as_str),
            Some("single")
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
    fn library_reclassification_preserves_finalized_track_state_and_files() {
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
        record.status = TrackStatus::Finalized;
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
            finalized_at: Some("2026-08-01T12:03:00Z".into()),
            workflow_version: Some(record.workflow_version.clone()),
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
            .expect("finalized album reassignment");
        assert_eq!(album.status, TrackStatus::Finalized);
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
            .expect("finalized single reassignment");
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
    fn authoritative_release_and_artwork_roles_are_singular() {
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
        for title in ["../escape", "/absolute", r"..\\escape"] {
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
        for title in ["../x", r"a\b", "...", "🚀"] {
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
            finalized_at: Some("2026-08-13T12:00:00Z".into()),
            workflow_version: Some(record.workflow_version.clone()),
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
                    suno_final_generation_date: Some("2026-08-02".into()),
                    suno_final_generation_time: Some("14:35".into()),
                    suno_download_export_date: Some("2026-08-03".into()),
                    suno_plan_at_creation: Some("Pro".into()),
                    final_export_date: Some("2026-08-03".into()),
                    lyrics_source: Some("instrumental".into()),
                    instrumental_track: Some(true),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(false),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(false),
                    commercial_use_intended: Some(true),
                    suno_terms_evidence_not_available: Some(true),
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
        let subscription = fixture_root.join("subscription.pdf");
        fs::write(&suno_export, b"RIFF\x08\0\0\0WAVEsuno evidence").expect("Suno fixture");
        fs::write(&release_master, b"RIFF\x08\0\0\0WAVErelease evidence").expect("release fixture");
        image::RgbaImage::from_pixel(640, 640, image::Rgba([24, 48, 96, 255]))
            .save(&ai_original)
            .expect("AI artwork fixture");
        fs::write(
            &subscription,
            b"%PDF-1.7\n1 0 obj\n<</Type /Receipt>>\nendobj\n%%EOF\n",
        )
        .expect("subscription fixture");
        app.import_evidence_from(&updated.id, EvidenceRole::SunoFinalExport, &suno_export)
            .expect("Suno evidence import");
        app.import_evidence_from(&updated.id, EvidenceRole::ReleaseWav, &release_master)
            .expect("release evidence import");
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
        let global = app
            .register_global_evidence(
                EvidenceRole::SubscriptionPayment,
                &subscription,
                Some("2026-07-01".into()),
                Some("2026-08-31".into()),
            )
            .expect("global subscription evidence");
        let portable = app
            .attach_global_evidence(&updated.id, &global.evidence.id)
            .expect("portable subscription copy");
        assert!(portable.evidence.iter().any(|item| {
            item.role == EvidenceRole::SubscriptionPayment
                && item.source_global_evidence_id.as_deref() == Some(global.evidence.id.as_str())
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
        let manifest =
            fs::read_to_string(track_root.join(certificate::MANIFEST_FILE)).expect("manifest text");
        assert!(!manifest.contains(app.root().to_string_lossy().as_ref()));
        assert!(manifest.contains("\"relative_path\": \".\""));
        assert!(manifest.contains("01_RELEASE/End To End.wav"));
        assert!(manifest.contains("02_SUNO/suno-export.wav"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_AI_ORIGINAL.png"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_AI_EDITED.png"));
        assert!(manifest.contains("05_ARTWORK/End-To-End_FINAL.png"));
        assert!(manifest.contains("sourceGlobalEvidenceId"));
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
                    suno_plan_at_creation: Some("Pro".into()),
                    final_export_date: Some("2026-08-03".into()),
                    lyrics_source: Some("instrumental".into()),
                    instrumental_track: Some(true),
                    suno_style_prompt: Some("cinematic synthwave, driving bass".into()),
                    external_audio_uploaded: Some(false),
                    own_audio_uploaded: Some(false),
                    code_based_generation: Some(false),
                    third_party_samples_uploaded: Some(false),
                    human_editing_performed: Some(false),
                    post_export_editing_performed: Some(false),
                    commercial_use_intended: Some(false),
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
        fs::write(&suno_export, b"RIFF\x08\0\0\0WAVEsuno cross-check")
            .expect("Suno export fixture");
        fs::write(&release_master, b"RIFF\x08\0\0\0WAVErelease cross-check")
            .expect("release fixture");
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

        let na_reason = "AI transparency is not applicable to human artwork.";
        app.set_step_status(
            &updated.id,
            "ai_transparency",
            StepStatus::NotApplicable,
            Some(na_reason.into()),
        )
        .expect("store justified N/A");
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
        assert_eq!(
            certificate
                .na_reasons
                .get("ai_transparency")
                .map(String::as_str),
            Some(na_reason)
        );
        assert!(stored_steps.iter().any(|step| {
            step.id == "ai_transparency"
                && step.status == StepStatus::NotApplicable
                && step.na_reason.as_deref() == Some(na_reason)
        }));
        let manifest_completed = manifest_steps
            .iter()
            .filter(|step| matches!(step["status"].as_str(), Some("PASS" | "N_A")))
            .map(|step| {
                let id = step["id"].as_str().expect("completed step ID").to_owned();
                let status = match step["status"].as_str().expect("completed status") {
                    "PASS" => "Pass",
                    "N_A" => "NotApplicable",
                    status => panic!("unexpected completed status: {status}"),
                };
                (id, status.to_owned())
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(certificate.completed_steps, manifest_completed);
    }

    #[test]
    fn global_subscription_evidence_requires_pdf_signature_and_covering_dates() {
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
        assert!(matches!(
            app.attach_global_evidence(&track.id, &narrow.evidence.id),
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
            .find(|item| item.role == EvidenceRole::SubscriptionPayment)
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
    fn legacy_scan_keeps_one_final_artwork_singular_and_marks_duplicate_ambiguous() {
        let directory = tempdir().expect("temporary directory");
        let workspace = directory.path().join("workspace");
        let app = WorkspaceApp::open(&workspace, true).expect("workspace");
        let legacy_root = workspace.join("Legacy Artwork Candidates");
        fs::create_dir_all(legacy_root.join("05_ARTWORK")).expect("legacy artwork directory");
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
        let final_items = evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::FinalArtwork)
            .collect::<Vec<_>>();
        let ambiguous_items = evidence
            .iter()
            .filter(|item| item.role == EvidenceRole::Other)
            .collect::<Vec<_>>();
        assert_eq!(final_items.len(), 1);
        assert_eq!(ambiguous_items.len(), 1);
        assert_eq!(final_items[0].relative_path, "05_ARTWORK/a_final.png");
        assert_eq!(ambiguous_items[0].relative_path, "05_ARTWORK/b_final.png");
        assert!(evidence.iter().all(|item| {
            item.provenance == EvidenceProvenance::IndexedLegacy && !item.verified
        }));
        assert!(ambiguous_items[0]
            .verification_error
            .as_deref()
            .is_some_and(|message| message.contains("ambiguous duplicate")));
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
    fn workflow_upgrade_archives_finalized_v13_and_requires_fresh_v14_outputs() {
        let directory = tempdir().expect("temporary directory");
        let app = WorkspaceApp::open(&directory.path().join("workspace"), true).expect("workspace");
        app.update_profile(complete_profile()).expect("profile");
        let finalized =
            finalize_acceptance_track(&app, &directory.path().join("fixtures"), "Workflow Upgrade");
        assert_eq!(finalized.workflow_version, "1.3");
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

        let workflow_v14 = workflow::config_with_version_for_test("1.4")
            .expect("test-only workflow 1.4 configuration");
        let upgraded = app
            .re_evaluate_track_with_workflow(&finalized.id, &workflow_v14)
            .expect("explicit workflow reevaluation")
            .track
            .expect("upgraded track detail");

        assert_eq!(upgraded.status, TrackStatus::Active);
        assert_eq!(upgraded.workflow_id, "suno-track");
        assert_eq!(upgraded.workflow_version, "1.4");
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
            app.re_evaluate_track_with_workflow(&finalized.id, &workflow_v14),
            Err(AppError::Validation(_))
        ));
        assert_eq!(
            fs::read_dir(track_root.join(".archive/revisions"))
                .expect("unchanged revision archive")
                .count(),
            1
        );
    }
}
