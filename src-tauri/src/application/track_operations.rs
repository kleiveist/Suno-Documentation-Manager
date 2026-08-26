use super::*;

impl WorkspaceApp {
    pub fn create_track(&self, input: CreateTrackInput) -> Result<TrackDetail> {
        validate_track_title(&input.title)?;
        let profile = self.profile()?;
        validate_profile(&profile, true)?;
        validate_optional_date("Production start", &input.production_start_date)?;
        let library =
            self.existing_album_spelling(normalize_track_library(input.library.clone())?)?;
        let relative_path = physical_track_relative(&library, &input.title)?;
        let (target, parent, parent_existed) = self.prepare_track_directory(&relative_path)?;
        if let Err(error) = Self::create_track_directory_tree(&target) {
            Self::cleanup_track_creation(&target, &parent, parent_existed);
            return Err(error);
        }
        let track = Self::new_track_record(input, profile, library, relative_path)?;
        if let Err(error) = self.persistence.save_track(&track) {
            Self::cleanup_track_creation(&target, &parent, parent_existed);
            return Err(error);
        }
        if let Err(error) = self.write_track_identity(&track) {
            let _ = self.persistence.delete_track(&track.id);
            Self::cleanup_track_creation(&target, &parent, parent_existed);
            return Err(error);
        }
        if let Err(error) = self.attach_default_terms_evidence(&track) {
            let _ = self.persistence.delete_track(&track.id);
            Self::cleanup_track_creation(&target, &parent, parent_existed);
            return Err(error);
        }
        self.load_track(&track.id)
    }

    fn prepare_track_directory(&self, relative_path: &str) -> Result<(PathBuf, PathBuf, bool)> {
        if self
            .persistence
            .track_by_relative_path(relative_path)?
            .is_some()
        {
            return Err(AppError::Collision(relative_path.to_owned()));
        }
        let target_relative = Path::new(relative_path);
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
            return Err(AppError::Collision(relative_path.to_owned()));
        }
        Ok((target, parent, parent_existed))
    }

    fn create_track_directory_tree(target: &Path) -> Result<()> {
        fs::create_dir(target).map_err(|error| AppError::io(target, error))?;
        for folder in TRACK_FOLDERS {
            ensure_contained_directory(target, Path::new(folder))?;
        }
        Ok(())
    }

    fn cleanup_track_creation(target: &Path, parent: &Path, parent_existed: bool) {
        let _ = fs::remove_dir_all(target);
        if !parent_existed {
            let _ = fs::remove_dir(parent);
        }
    }

    fn new_track_record(
        input: CreateTrackInput,
        profile: Profile,
        library: TrackLibraryPlacement,
        relative_path: String,
    ) -> Result<TrackRecord> {
        let created_at = now();
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
        Ok(TrackRecord {
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
            created_at: created_at.clone(),
            updated_at: created_at,
            legacy: false,
        })
    }

    fn attach_default_terms_evidence(&self, track: &TrackRecord) -> Result<()> {
        for global in self
            .persistence
            .global_evidence()?
            .into_iter()
            .filter(|item| item.evidence.role == EvidenceRole::SunoTermsRights)
        {
            self.attach_global_evidence(&track.id, &global.evidence.id)?;
        }
        Ok(())
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
        let prepared = self.prepare_folder_import(
            &input,
            &proposal,
            plans,
            &source_root,
            commercial_use_intended,
        )?;
        let mut imported = Vec::new();
        for (plan, create_input) in prepared {
            imported.push(self.import_prepared_folder_track(plan, create_input)?);
        }
        Ok(imported)
    }

    fn prepare_folder_import(
        &self,
        input: &FolderImportExecutionInput,
        proposal: &FolderImportProposal,
        plans: Vec<folder_import::ImportTrackPlan>,
        source_root: &Path,
        commercial_use_intended: bool,
    ) -> Result<Vec<(folder_import::ImportTrackPlan, CreateTrackInput)>> {
        let mut prepared = Vec::new();
        let mut target_paths = HashSet::new();
        for plan in plans {
            let create_input = self.prepare_folder_import_track(
                input,
                proposal,
                &plan,
                source_root,
                commercial_use_intended,
                &mut target_paths,
            )?;
            prepared.push((plan, create_input));
        }
        Ok(prepared)
    }

    fn prepare_folder_import_track(
        &self,
        input: &FolderImportExecutionInput,
        proposal: &FolderImportProposal,
        plan: &folder_import::ImportTrackPlan,
        source_root: &Path,
        commercial_use_intended: bool,
        target_paths: &mut HashSet<String>,
    ) -> Result<CreateTrackInput> {
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
        if target.starts_with(source_root) {
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
        Ok(CreateTrackInput {
            title,
            production_start_date,
            commercial_use_intended,
            library,
        })
    }

    fn import_prepared_folder_track(
        &self,
        plan: folder_import::ImportTrackPlan,
        create_input: CreateTrackInput,
    ) -> Result<TrackDetail> {
        let mut track = self.create_track(create_input)?;
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
        Ok(track)
    }
}

impl WorkspaceApp {
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
        result.sort_by_key(|track| track.title.to_lowercase());
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

    pub(super) fn update_track_with_explicit_nulls(
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
        Self::validate_terms_unavailable_request(terms_unavailable_requested, &evidence)?;
        reconcile_evidence_derived_fields(&mut track, &evidence);
        if track.fields.title != previous_fields.title {
            track.fields.release_filename_difference_confirmed = None;
            track.fields.suno_export_filename_difference_confirmed = None;
        }
        validate_track_fields(&track.fields)?;
        let changed = track.fields != previous_fields || track.field_origins != previous_origins;
        let authoritative_release_renamed = if changed {
            self.persist_changed_track_update(&mut track, &previous_path, &previous_fields)?
        } else {
            false
        };
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

    fn validate_terms_unavailable_request(
        requested: bool,
        evidence: &[EvidenceItem],
    ) -> Result<()> {
        if requested
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
        Ok(())
    }

    fn persist_changed_track_update(
        &self,
        track: &mut TrackRecord,
        previous_path: &str,
        previous_fields: &crate::model::TrackFields,
    ) -> Result<bool> {
        let release_renames =
            self.rename_track_for_title_update(track, previous_path, previous_fields)?;
        let authoritative_release_renamed = release_renames
            .iter()
            .any(|(_, updated)| updated.role == EvidenceRole::ReleaseWav);
        if authoritative_release_renamed {
            // A managed filename is part of the screening source binding.
            // The bytes may be identical, but an old path must never keep
            // a prior fingerprint or provider result current.
            audio_screening::mark_screening_stale(&mut track.audio_screening);
        }
        mark_content_changed(track);
        track.status = TrackStatus::Active;
        track.updated_at = now();
        if let Err(error) = self.persistence.save_track(track) {
            return Err(self.rollback_failed_track_update(
                track,
                previous_path,
                &release_renames,
                error,
            ));
        }
        self.write_track_identity(track)?;
        Ok(authoritative_release_renamed)
    }

    fn rename_track_for_title_update(
        &self,
        track: &mut TrackRecord,
        previous_path: &str,
        previous_fields: &crate::model::TrackFields,
    ) -> Result<Vec<(EvidenceItem, EvidenceItem)>> {
        if track.fields.title == previous_fields.title {
            return Ok(Vec::new());
        }
        let target = physical_track_relative(&track.library, &track.fields.title)?;
        self.move_track_directory(previous_path, &target, false)?;
        track.relative_path = target;
        match self.rename_managed_release_evidence(track, &track.fields.title) {
            Ok(renamed) => Ok(renamed),
            Err(error) => {
                if let Err(rollback) = self.rollback_track_move(&track.relative_path, previous_path)
                {
                    return Err(AppError::Data(format!(
                        "Release rename failed ({error}); folder rollback failed: {rollback}"
                    )));
                }
                Err(error)
            }
        }
    }

    fn rollback_failed_track_update(
        &self,
        track: &TrackRecord,
        previous_path: &str,
        release_renames: &[(EvidenceItem, EvidenceItem)],
        error: AppError,
    ) -> AppError {
        let release_rollback = self.rollback_release_evidence_renames(track, release_renames);
        if track.relative_path != previous_path {
            if let Err(rollback) = self.rollback_track_move(&track.relative_path, previous_path) {
                return AppError::Data(format!(
                    "Track update failed ({error}); folder rollback failed: {rollback}"
                ));
            }
        }
        if let Err(rollback) = release_rollback {
            return AppError::Data(format!(
                "Track update failed ({error}); release rollback failed: {rollback}"
            ));
        }
        error
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

        let mut tracks = self.album_tracks(&old_title)?;
        let source_relative = PathBuf::from(safe_album_directory(&old_title)?);
        let target_relative = PathBuf::from(safe_album_directory(new_title)?);
        let source = contained_path(&self.root, &source_relative, true)?;
        let target = contained_path(&self.root, &target_relative, false)?;
        Self::validate_album_rename(
            &tracks,
            &old_title,
            &source_relative,
            &target_relative,
            &source,
            &target,
        )?;
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

    fn album_tracks(&self, title: &str) -> Result<Vec<TrackRecord>> {
        Ok(self
            .persistence
            .tracks()?
            .into_iter()
            .filter(|track| {
                track.library.section == TrackLibrarySection::Album
                    && track
                        .library
                        .album_title
                        .as_deref()
                        .is_some_and(|album| album.eq_ignore_ascii_case(title))
            })
            .collect())
    }

    fn validate_album_rename(
        tracks: &[TrackRecord],
        old_title: &str,
        source_relative: &Path,
        target_relative: &Path,
        source: &Path,
        target: &Path,
    ) -> Result<()> {
        if target.exists()
            && fs::canonicalize(target).map_err(|error| AppError::io(target, error))?
                != fs::canonicalize(source).map_err(|error| AppError::io(source, error))?
        {
            return Err(AppError::Collision(portable_relative(target_relative)));
        }
        for track in tracks {
            let relative = Path::new(&track.relative_path);
            if !relative.starts_with(source_relative) || relative.components().count() != 2 {
                return Err(AppError::Validation(format!(
                    "Track {} is not stored inside album folder {}.",
                    track.fields.title, old_title
                )));
            }
        }
        Ok(())
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
}
