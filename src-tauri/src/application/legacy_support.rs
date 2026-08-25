use super::*;

pub(super) struct LegacyInspection {
    pub(super) recognized_folders: Vec<String>,
    pub(super) documents: Vec<String>,
    pub(super) evidence_files: Vec<String>,
    pub(super) hash_manifest_present: bool,
    pub(super) has_managed_document_collision: bool,
}

pub(super) fn inspect_legacy(root: &Path) -> Result<LegacyInspection> {
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
            && portable != certificate::PDF_FILE_DE
            && !portable.starts_with(".archive/")
            && !portable.starts_with(".summary/")
            && !portable.starts_with("03_DOCUMENTATION/AUDIO_SCREENING/")
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

pub(super) fn infer_legacy_role(relative: &str) -> EvidenceRole {
    let value = relative.to_ascii_lowercase();
    if value.contains("suno_original") {
        EvidenceRole::ArtworkSunoOriginal
    } else if value.contains("ai_original") {
        EvidenceRole::AiArtworkOriginal
    } else if value.contains("ai_edited") {
        EvidenceRole::AiArtworkEdited
    } else if (value.contains("human_edited") || value.contains("_edited"))
        && [".png", ".jpg", ".jpeg"]
            .iter()
            .any(|extension| value.ends_with(extension))
    {
        EvidenceRole::HumanEditedArtwork
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

pub(super) fn summary_from_detail(detail: &TrackDetail) -> TrackSummary {
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

pub(super) fn final_artwork_evidence_id(evidence: &[EvidenceItem]) -> Option<String> {
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

pub(super) fn now() -> String {
    Utc::now().to_rfc3339()
}

pub(super) fn provider_response_staging_path(
    track_root: &Path,
    extension: &str,
) -> Result<PathBuf> {
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

pub(super) fn referenced_finalization_snapshot_id(
    track: &TrackRecord,
    certificate_id: &str,
) -> String {
    track
        .certificate
        .finalization_snapshot_id
        .clone()
        // Pre-schema tracks have no separately persisted snapshot UUID. The
        // Certificate ID is already immutable and archived, so a namespaced
        // fallback keeps the association explicit without modifying history.
        .unwrap_or_else(|| format!("legacy-finalization-snapshot:{certificate_id}"))
}

/// A persisted summary is only presentation state. A positive RFC 3161 claim
/// must be reconstructed from an intact, certificate-bound sidecar whose
/// immutable provider record contains every required verification result.
pub(super) fn currently_verified_provider_timestamp(record: &ExternalTimestampRecord) -> bool {
    let Some(metadata) = record.provider_metadata.as_ref() else {
        return false;
    };
    record.integrity_verified
        && record.referenced_artifact == crate::model::TimestampReferencedArtifact::EvidenceManifest
        && record.referenced_hash_match == Some(true)
        && record
            .actual_sha256
            .eq_ignore_ascii_case(&record.referenced_sha256)
        && metadata.verification_result == ExternalTimestampStatus::Verified
        && metadata.response_structure_valid == Some(true)
        && metadata.provider_digest_match == Some(true)
        && !metadata.request_nonce.trim().is_empty()
        && !metadata.response_nonce.trim().is_empty()
        && metadata.nonce_match == Some(true)
        && !metadata.policy_oid.trim().is_empty()
        && (metadata.requested_policy_oid.trim().is_empty() || metadata.policy_match == Some(true))
        && metadata.signature_verified == Some(true)
        && metadata.trust_chain_verified == Some(true)
        && metadata.cryptographic_verifier
            == crate::external_timestamp::RFC3161_CRYPTOGRAPHIC_VERIFIER
        && !metadata.trust_anchor_sha256.is_empty()
        && metadata
            .trust_anchor_sha256
            .iter()
            .all(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Convert only unambiguous historical Suno content answers when the user
/// explicitly starts a revision or upgrades the workflow. Ordinary reads and
/// saves intentionally do not infer new semantic facts.
pub(super) fn migrate_legacy_suno_semantics(fields: &mut crate::model::TrackFields) -> bool {
    if let Some(classification) = fields.suno_content_classification {
        let mut changed = fields.suno_lyrics_field_content.is_some()
            || !fields.suno_lyrics_content_types.is_empty();
        fields.suno_lyrics_field_content = None;
        fields.suno_lyrics_content_types.clear();
        if classification == SunoContentClassification::Empty {
            changed |= fields.suno_lyrics_content_source.is_some()
                || !fields.suno_lyrics_field_text.is_empty()
                || !fields.suno_lyrics_other_content_type.is_empty();
            fields.suno_lyrics_content_source = None;
            fields.suno_lyrics_field_text.clear();
            fields.suno_lyrics_other_content_type.clear();
        } else if classification != SunoContentClassification::Other {
            changed |= !fields.suno_lyrics_other_content_type.is_empty();
            fields.suno_lyrics_other_content_type.clear();
        }
        return changed;
    }

    let classification = if fields.suno_lyrics_field_content == Some(false) {
        Some(SunoContentClassification::Empty)
    } else if fields.suno_lyrics_content_types.is_empty() {
        None
    } else {
        let only_vocal = fields
            .suno_lyrics_content_types
            .iter()
            .all(|value| *value == SunoLyricsContentType::VocalLyrics);
        let only_instructions = fields.suno_lyrics_content_types.iter().all(|value| {
            matches!(
                value,
                SunoLyricsContentType::StructureInstructions
                    | SunoLyricsContentType::SoundInstructions
                    | SunoLyricsContentType::ArrangementInstructions
            )
        });
        let only_other = fields
            .suno_lyrics_content_types
            .iter()
            .all(|value| *value == SunoLyricsContentType::Other);
        let has_vocal = fields
            .suno_lyrics_content_types
            .contains(&SunoLyricsContentType::VocalLyrics);
        let has_instruction = fields.suno_lyrics_content_types.iter().any(|value| {
            matches!(
                value,
                SunoLyricsContentType::StructureInstructions
                    | SunoLyricsContentType::SoundInstructions
                    | SunoLyricsContentType::ArrangementInstructions
            )
        });
        let contains_only_vocal_and_instructions =
            fields.suno_lyrics_content_types.iter().all(|value| {
                matches!(
                    value,
                    SunoLyricsContentType::VocalLyrics
                        | SunoLyricsContentType::StructureInstructions
                        | SunoLyricsContentType::SoundInstructions
                        | SunoLyricsContentType::ArrangementInstructions
                )
            });

        if only_vocal {
            Some(SunoContentClassification::VocalLyricsOnly)
        } else if only_instructions {
            Some(SunoContentClassification::StructureOnly)
        } else if only_other {
            Some(SunoContentClassification::Other)
        } else if has_vocal && has_instruction && contains_only_vocal_and_instructions {
            Some(SunoContentClassification::Mixed)
        } else {
            // Historical `mixed`, `other` combined with another value, and all
            // other unclear combinations require an explicit new decision.
            None
        }
    };

    let Some(classification) = classification else {
        return false;
    };
    fields.suno_content_classification = Some(classification);
    fields.suno_lyrics_field_content = None;
    fields.suno_lyrics_content_types.clear();
    if classification == SunoContentClassification::Empty {
        fields.suno_lyrics_content_source = None;
        fields.suno_lyrics_field_text.clear();
        fields.suno_lyrics_other_content_type.clear();
    } else if classification != SunoContentClassification::Other {
        fields.suno_lyrics_other_content_type.clear();
    }
    true
}

pub(super) fn workflow_version_mismatch(track: &TrackRecord) -> Result<Option<String>> {
    let current = workflow::config()?;
    if track.workflow_id == current.id && track.workflow_version == current.version {
        return Ok(None);
    }
    Ok(Some(format!(
        "Workflow {} {} is stored for this track, but {} {} is current. Re-evaluate the track explicitly before generating documents, hashes, or a certificate.",
        track.workflow_id, track.workflow_version, current.id, current.version
    )))
}

pub(super) fn ensure_current_workflow(track: &TrackRecord) -> Result<()> {
    if let Some(message) = workflow_version_mismatch(track)? {
        return Err(AppError::Validation(message));
    }
    Ok(())
}

pub(super) fn invalidate_state(track: &mut TrackRecord, reason: &str) {
    track.certificate.valid = false;
    track.certificate.invalidated_at = Some(now());
    track.certificate.invalidation_reason = Some(reason.into());
}

pub(super) fn mark_content_changed(track: &mut TrackRecord) {
    track.documents.current = false;
    track.integrity = IntegrityState::default();
}
