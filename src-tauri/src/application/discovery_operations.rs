use super::*;

impl WorkspaceApp {
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
        candidates.sort_by_key(|candidate| candidate.name.to_lowercase());
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

    pub(super) fn mutable_track(&self, id: &str) -> Result<TrackRecord> {
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

    pub(super) fn reconcile_unindexed_evidence(
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

    pub(super) fn write_track_identity(&self, track: &TrackRecord) -> Result<()> {
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

    pub(super) fn existing_album_spelling(
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

    pub(super) fn move_track_directory(
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

    pub(super) fn rollback_track_move(
        &self,
        source_relative: &str,
        target_relative: &str,
    ) -> Result<()> {
        self.move_track_directory(source_relative, target_relative, true)
    }

    pub(super) fn remove_empty_library_directory(&self, relative: &Path) -> Result<()> {
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

    pub(super) fn reconcile_physical_library(&self) -> Result<()> {
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

        for track in tracks {
            self.reconcile_physical_track(track, &identities, &mut claimed)?;
        }
        Ok(())
    }

    fn reconcile_physical_track(
        &self,
        mut track: TrackRecord,
        identities: &HashMap<String, String>,
        claimed: &mut HashSet<String>,
    ) -> Result<()> {
        let original_path = track.relative_path.clone();
        let Some(moved_from) =
            self.reconcile_track_location(&mut track, identities, claimed, &original_path)?
        else {
            return Ok(());
        };
        if let Some(inferred) = physical_library_from_relative(&track.relative_path)? {
            track.library = inferred;
        }
        if track.relative_path != original_path
            || track.library != self.persistence.track(&track.id)?.library
        {
            if let Err(error) = self.persistence.save_track(&track) {
                if let Some(source) = moved_from {
                    if let Err(rollback) = self.rollback_track_move(&track.relative_path, &source) {
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
        Ok(())
    }

    fn reconcile_track_location(
        &self,
        track: &mut TrackRecord,
        identities: &HashMap<String, String>,
        claimed: &mut HashSet<String>,
        original_path: &str,
    ) -> Result<Option<Option<String>>> {
        let stored_exists = self.root.join(original_path).is_dir();
        if stored_exists && !track.legacy && Path::new(original_path).components().count() == 1 {
            let leaf = Path::new(original_path)
                .file_name()
                .ok_or_else(|| AppError::Data("Stored track path has no folder name.".into()))?;
            let parent = physical_library_parent(&track.library)?;
            let target = portable_relative(&parent.join(leaf));
            self.move_track_directory(original_path, &target, false)?;
            claimed.remove(original_path);
            claimed.insert(target.clone());
            track.relative_path = target;
            return Ok(Some(Some(original_path.to_owned())));
        }
        if stored_exists {
            return Ok(Some(None));
        }
        let discovered = identities.get(&track.id).cloned().or_else(|| {
            find_unclaimed_track_in_library(&self.root, &track.library, claimed)
                .ok()
                .flatten()
        });
        let Some(relative_path) = discovered else {
            return Ok(None);
        };
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
        Ok(Some(None))
    }

    pub(super) fn rename_managed_release_evidence(
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

    pub(super) fn rollback_release_evidence_renames(
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

    pub(super) fn migrate_legacy_release_evidence(&self, track: &TrackRecord) -> Result<bool> {
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
            .filter(Self::is_legacy_release_candidate)
        {
            if self.migrate_legacy_release_item(track, &root, previous)? {
                migrated = true;
            }
        }
        Ok(migrated)
    }

    fn is_legacy_release_candidate(item: &EvidenceItem) -> bool {
        let relative = Path::new(&item.relative_path);
        let extension = relative
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        item.provenance == EvidenceProvenance::ManagedCopy
            && matches!(
                item.role,
                EvidenceRole::ReleaseWav | EvidenceRole::ReleaseMp3 | EvidenceRole::ReleaseMp4
            )
            && relative.parent() == Some(Path::new("01_RELEASE"))
            && relative.file_name().and_then(|value| value.to_str())
                == Some(item.file_name.as_str())
            && relative.file_stem().and_then(|value| value.to_str()) == Some("suno_final_export")
            && extension
                .as_deref()
                .is_some_and(|extension| item.role.allowed_extensions().contains(&extension))
    }

    fn migrate_legacy_release_item(
        &self,
        track: &TrackRecord,
        root: &Path,
        previous: EvidenceItem,
    ) -> Result<bool> {
        let Ok(planned) = evidence::managed_relative_path(
            &track.fields.title,
            &previous.role,
            Path::new(&previous.file_name),
        ) else {
            return Ok(false);
        };
        let planned_portable = portable_relative(&planned);
        if planned_portable == previous.relative_path
            || contained_path(root, &planned, false)?.exists()
            || self
                .persistence
                .evidence_by_relative_path(&track.id, &planned_portable)?
                .is_some()
        {
            return Ok(false);
        }
        let source = contained_path(root, Path::new(&previous.relative_path), false)?;
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(AppError::io(&source, error)),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Ok(false);
        }
        let target = contained_path(root, &planned, false)?;
        fs::rename(&source, &target).map_err(|error| AppError::io(&target, error))?;
        let mut updated = previous.clone();
        updated.relative_path = planned_portable;
        updated.file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::Validation("Managed release file name is invalid.".into()))?
            .to_owned();
        if let Err(error) = self.persistence.save_evidence(&track.id, &updated) {
            let _ = fs::rename(&target, &source);
            return Err(error);
        }
        Ok(true)
    }

    pub(super) fn track_root(&self, track: &TrackRecord) -> Result<PathBuf> {
        contained_path(&self.root, Path::new(&track.relative_path), true)
    }
}
