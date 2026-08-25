use super::*;

pub(super) fn directory_is_empty_or_missing(path: &Path) -> Result<bool> {
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

pub(super) fn matching_revision_certificate(
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

pub(super) fn finalized_artifacts_need_revision_restore(
    track_root: &Path,
    live_certificate: &Path,
) -> Result<bool> {
    if directory_is_empty_or_missing(live_certificate)? {
        return Ok(true);
    }
    // A malformed live set is ordinary certificate invalidation, not proof of
    // an interrupted revision. Only a readable certificate hash set establishes
    // which root PDFs belong to this finalized snapshot and need recovery.
    if !matches!(certificate::expects_pdf(track_root), Ok(true)) {
        return Ok(false);
    }
    Ok(certificate::required_pdf_files(track_root)?
        .iter()
        .any(|name| !track_root.join(name).is_file()))
}

pub(super) fn restore_revision_artifacts(
    live_certificate: &Path,
    archived_certificate: &Path,
) -> Result<bool> {
    let Some(plan) = revision_restore_plan(live_certificate, archived_certificate)? else {
        return Ok(false);
    };
    execute_revision_restore(live_certificate, archived_certificate, plan)?;
    Ok(true)
}

struct RevisionRestorePlan {
    restore_certificate: bool,
    pdf_restores: Vec<(PathBuf, PathBuf)>,
}

fn revision_restore_plan(
    live_certificate: &Path,
    archived_certificate: &Path,
) -> Result<Option<RevisionRestorePlan>> {
    let revision_directory = archived_certificate
        .parent()
        .ok_or_else(|| AppError::Data("Revision certificate has no archive directory.".into()))?;
    let live_root = live_certificate
        .parent()
        .ok_or_else(|| AppError::Data("Live certificate has no track root.".into()))?;
    let restore_certificate = directory_is_empty_or_missing(live_certificate)?;
    let mut pdf_restores = Vec::new();
    for name in [certificate::PDF_FILE, certificate::PDF_FILE_DE] {
        let archived_pdf = revision_directory.join(name);
        let live_pdf = live_root.join(name);
        let archived_pdf_exists =
            regular_file_if_present(&archived_pdf, "The archived technical documentation PDF")?;
        let live_pdf_exists =
            regular_file_if_present(&live_pdf, "The live technical documentation PDF")?;
        if archived_pdf_exists && live_pdf_exists {
            if sha256_file(&archived_pdf)? != sha256_file(&live_pdf)? {
                return Err(AppError::Validation(format!(
                    "The live and archived technical documentation PDFs do not match: {name}."
                )));
            }
        } else if archived_pdf_exists {
            pdf_restores.push((archived_pdf, live_pdf));
        }
    }
    if !restore_certificate && pdf_restores.is_empty() {
        return Ok(None);
    }
    if restore_certificate && live_certificate.exists() {
        fs::remove_dir(live_certificate).map_err(|error| AppError::io(live_certificate, error))?;
    }
    Ok(Some(RevisionRestorePlan {
        restore_certificate,
        pdf_restores,
    }))
}

fn execute_revision_restore(
    live_certificate: &Path,
    archived_certificate: &Path,
    plan: RevisionRestorePlan,
) -> Result<()> {
    let mut certificate_moved = false;
    let mut pdf_moved = Vec::new();
    let restore_result = (|| -> Result<()> {
        if plan.restore_certificate {
            fs::rename(archived_certificate, live_certificate)
                .map_err(|error| AppError::io(live_certificate, error))?;
            certificate_moved = true;
        }
        for (archived_pdf, live_pdf) in &plan.pdf_restores {
            copy_new(archived_pdf, live_pdf)?;
            pdf_moved.push(live_pdf.clone());
        }
        Ok(())
    })();
    if let Err(cause) = restore_result {
        let mut rollback_errors = Vec::new();
        for live_pdf in pdf_moved.iter().rev() {
            if let Err(error) = fs::remove_file(live_pdf) {
                rollback_errors.push(format!("PDF rollback failed: {error}"));
            }
        }
        if certificate_moved {
            if let Err(error) = fs::rename(live_certificate, archived_certificate) {
                rollback_errors.push(format!("certificate rollback failed: {error}"));
            }
        }
        if plan.restore_certificate && !live_certificate.exists() {
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
    Ok(())
}

pub(super) fn regular_file_if_present(path: &Path, label: &str) -> Result<bool> {
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

pub(super) fn rollback_removed_file(archived: &Path, original: &Path, cause: AppError) -> AppError {
    match fs::rename(archived, original) {
        Ok(()) => cause,
        Err(rollback_error) => AppError::Data(format!(
            "Removal failed ({cause}); file rollback failed: {rollback_error}"
        )),
    }
}

pub(super) struct CertificateRollback {
    pub(super) error: AppError,
    pub(super) complete: bool,
}

pub(super) fn rollback_certificate_set(track_root: &Path, cause: AppError) -> CertificateRollback {
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
    let pdfs = match [
        contained_path(track_root, Path::new(certificate::PDF_FILE), false),
        contained_path(track_root, Path::new(certificate::PDF_FILE_DE), false),
    ]
    .into_iter()
    .collect::<Result<Vec<_>>>()
    {
        Ok(paths) => paths,
        Err(rollback_error) => {
            return CertificateRollback {
                error: AppError::Data(format!(
                    "Finalization failed ({cause}); PDF rollback path failed: {rollback_error}"
                )),
                complete: false,
            };
        }
    };
    for pdf in pdfs {
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

pub(super) struct RevisionRollbackContext<'a> {
    pub(super) live_certificate: &'a Path,
    pub(super) archived_certificate: &'a Path,
    pub(super) live_root: &'a Path,
    pub(super) archived_root: &'a Path,
    pub(super) revision_directory: &'a Path,
    pub(super) certificate_existed: bool,
    pub(super) pdf_moved: &'a [&'a str],
}

pub(super) fn rollback_revision_state(
    context: RevisionRollbackContext<'_>,
    cause: AppError,
) -> AppError {
    let (pdf_restored, mut rollback_errors) =
        restore_revision_pdfs(context.live_root, context.archived_root, context.pdf_moved);
    let certificate_restored = restore_revision_certificate(
        context.live_certificate,
        context.archived_certificate,
        context.certificate_existed,
        pdf_restored,
        &mut rollback_errors,
    );
    if pdf_restored && certificate_restored {
        cleanup_revision_rollback_artifacts(
            context.archived_certificate,
            context.archived_root,
            context.revision_directory,
            context.certificate_existed,
            context.pdf_moved,
            &mut rollback_errors,
        );
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

fn restore_revision_pdfs(
    live_root: &Path,
    archived_root: &Path,
    pdf_moved: &[&str],
) -> (bool, Vec<String>) {
    let mut rollback_errors = Vec::new();
    let mut restored = true;
    for name in pdf_moved {
        let live_pdf = live_root.join(name);
        let archived_pdf = archived_root.join(name);
        if live_pdf.exists() {
            rollback_errors.push(format!(
                "PDF rollback would overwrite {}",
                live_pdf.display()
            ));
            restored = false;
        } else if let Err(error) = copy_new(&archived_pdf, &live_pdf) {
            rollback_errors.push(format!("PDF rollback failed: {error}"));
            restored = false;
        }
    }
    (restored, rollback_errors)
}

fn restore_revision_certificate(
    live_certificate: &Path,
    archived_certificate: &Path,
    certificate_existed: bool,
    pdf_restored: bool,
    rollback_errors: &mut Vec<String>,
) -> bool {
    if !certificate_existed {
        return true;
    }
    if !pdf_restored {
        return false;
    }
    let live_can_be_replaced = match directory_is_empty_or_missing(live_certificate) {
        Ok(true) if live_certificate.exists() => match fs::remove_dir(live_certificate) {
            Ok(()) => true,
            Err(error) => {
                rollback_errors.push(format!("live certificate cleanup failed: {error}"));
                false
            }
        },
        Ok(true) => true,
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
    if !live_can_be_replaced {
        return false;
    }
    match fs::rename(archived_certificate, live_certificate) {
        Ok(()) => true,
        Err(error) => {
            rollback_errors.push(format!("certificate rollback failed: {error}"));
            false
        }
    }
}

fn cleanup_revision_rollback_artifacts(
    archived_certificate: &Path,
    archived_root: &Path,
    revision_directory: &Path,
    certificate_existed: bool,
    pdf_moved: &[&str],
    rollback_errors: &mut Vec<String>,
) {
    for name in pdf_moved {
        let archived_pdf = archived_root.join(name);
        if let Err(error) = fs::remove_file(&archived_pdf) {
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

/// `AUDIO_SCREENING` is system-owned and may be moved only as a whole. The
/// caller has already resolved both paths under the managed track root; this
/// helper still refuses links and collisions before restoring an interrupted
/// revision transaction.
pub(super) fn restore_audio_screening_directory(staged: &Path, live: &Path) -> Result<()> {
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

pub(super) fn regular_directory_if_present(path: &Path, label: &str) -> Result<bool> {
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

pub(super) fn cleanup_revision_staging(stage: &Path, cause: AppError) -> AppError {
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
