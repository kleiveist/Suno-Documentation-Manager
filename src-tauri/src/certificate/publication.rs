use super::*;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CertificatePublicationFailure {
    StagingDirectoryCreate,
    ManifestWrite,
    CertificateWrite,
    PdfWrite,
    CertificateHashWrite,
    CertificatePublishRename,
    PdfPublish,
    PostPublishVerification,
}

#[cfg(test)]
impl CertificatePublicationFailure {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::StagingDirectoryCreate => "staging-directory-create",
            Self::ManifestWrite => "manifest-write",
            Self::CertificateWrite => "certificate-write",
            Self::PdfWrite => "pdf-write",
            Self::CertificateHashWrite => "certificate-hash-write",
            Self::CertificatePublishRename => "certificate-publish-rename",
            Self::PdfPublish => "pdf-publish",
            Self::PostPublishVerification => "post-publish-verification",
        }
    }

    pub(super) fn stage_id(self) -> String {
        format!("failure-injection-{}", self.label())
    }
}

#[cfg(test)]
impl CertificateGenerationFailure {
    pub(super) fn publication_failure(self) -> Option<CertificatePublicationFailure> {
        match self {
            Self::PdfGeneration => None,
            Self::PdfStaging => Some(CertificatePublicationFailure::PdfWrite),
            Self::PdfPublication => Some(CertificatePublicationFailure::PdfPublish),
            Self::PostPublishVerification => {
                Some(CertificatePublicationFailure::PostPublishVerification)
            }
        }
    }
}

#[cfg(test)]
pub(super) fn inject_certificate_publication_failure(
    configured: Option<CertificatePublicationFailure>,
    phase: CertificatePublicationFailure,
) -> Result<()> {
    if configured == Some(phase) {
        return Err(AppError::Data(format!(
            "Injected certificate publication failure at {}.",
            phase.label()
        )));
    }
    Ok(())
}

pub(super) struct CertificateSetBytes<'a> {
    pub(super) manifest: &'a [u8],
    pub(super) certificate: &'a [u8],
    pub(super) pdf_en: &'a [u8],
    pub(super) pdf_de: &'a [u8],
    pub(super) certificate_hashes: &'a [u8],
}

struct PublicationPaths {
    stage: PathBuf,
    staged_certificate_dir: PathBuf,
    destination: PathBuf,
    pdf_destinations: [PathBuf; 2],
}

#[derive(Default)]
struct PublicationState {
    destination_started_empty: bool,
    certificate_published: bool,
    pdf_published: Vec<PathBuf>,
}

pub(super) fn publish_certificate_set_impl(
    track_root: &Path,
    bytes: CertificateSetBytes<'_>,
    transaction_id: &str,
    #[cfg(test)] failure: Option<CertificatePublicationFailure>,
) -> Result<()> {
    #[cfg(test)]
    let stage_id = failure
        .map(CertificatePublicationFailure::stage_id)
        .unwrap_or_else(|| transaction_id.to_owned());
    #[cfg(not(test))]
    let stage_id = transaction_id.to_owned();
    let stage_relative = PathBuf::from(".archive")
        .join("certificate-staging")
        .join(stage_id);
    #[cfg(test)]
    inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::StagingDirectoryCreate,
    )?;
    let stage = ensure_contained_directory(track_root, &stage_relative)?;
    let paths = PublicationPaths {
        staged_certificate_dir: stage.join("certificate"),
        destination: contained_path(track_root, Path::new(CERTIFICATE_DIR), false)?,
        pdf_destinations: [
            contained_path(track_root, Path::new(PDF_FILE), false)?,
            contained_path(track_root, Path::new(PDF_FILE_DE), false)?,
        ],
        stage,
    };
    let mut state = PublicationState::default();
    let publish_result = publish_staged_set(
        track_root,
        &bytes,
        &paths,
        &mut state,
        #[cfg(test)]
        failure,
    );
    if let Err(cause) = publish_result {
        return rollback_publication(cause, &paths, &state);
    }
    Ok(())
}

fn publish_staged_set(
    track_root: &Path,
    bytes: &CertificateSetBytes<'_>,
    paths: &PublicationPaths,
    state: &mut PublicationState,
    #[cfg(test)] failure: Option<CertificatePublicationFailure>,
) -> Result<()> {
    fs::create_dir(&paths.staged_certificate_dir)
        .map_err(|error| AppError::io(&paths.staged_certificate_dir, error))?;
    let staged_manifest = paths.staged_certificate_dir.join("EVIDENCE_MANIFEST.json");
    let staged_certificate = paths
        .staged_certificate_dir
        .join("DOCUMENTATION_CERTIFICATE.md");
    let staged_hashes = paths.staged_certificate_dir.join("CERTIFICATE_SHA256.txt");
    let staged_pdf = paths.stage.join(PDF_FILE);
    #[cfg(test)]
    inject_certificate_publication_failure(failure, CertificatePublicationFailure::ManifestWrite)?;
    atomic_write_new(&staged_manifest, bytes.manifest)?;
    #[cfg(test)]
    inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::CertificateWrite,
    )?;
    atomic_write_new(&staged_certificate, bytes.certificate)?;
    #[cfg(test)]
    inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfWrite)?;
    atomic_write_new(&staged_pdf, bytes.pdf_en)?;
    atomic_write_new(&paths.stage.join(PDF_FILE_DE), bytes.pdf_de)?;
    #[cfg(test)]
    inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::CertificateHashWrite,
    )?;
    atomic_write_new(&staged_hashes, bytes.certificate_hashes)?;
    verify_staged_set(track_root, &paths.stage)?;
    ensure_publication_destinations_available(paths, state)?;
    #[cfg(test)]
    inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::CertificatePublishRename,
    )?;
    fs::rename(&paths.staged_certificate_dir, &paths.destination)
        .map_err(|error| AppError::io(&paths.destination, error))?;
    state.certificate_published = true;
    #[cfg(test)]
    inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfPublish)?;
    for (pdf_destination, staged_pdf) in paths
        .pdf_destinations
        .iter()
        .zip([paths.stage.join(PDF_FILE), paths.stage.join(PDF_FILE_DE)])
    {
        copy_new(&staged_pdf, pdf_destination)?;
        state.pdf_published.push(pdf_destination.clone());
    }
    #[cfg(test)]
    let post_publish_verification = inject_certificate_publication_failure(
        failure,
        CertificatePublicationFailure::PostPublishVerification,
    )
    .and_then(|()| verify(track_root));
    #[cfg(not(test))]
    let post_publish_verification = verify(track_root);
    post_publish_verification?;
    fs::remove_dir_all(&paths.stage).map_err(|error| AppError::io(&paths.stage, error))?;
    Ok(())
}

fn ensure_publication_destinations_available(
    paths: &PublicationPaths,
    state: &mut PublicationState,
) -> Result<()> {
    for pdf_destination in &paths.pdf_destinations {
        if pdf_destination.exists() {
            return Err(AppError::Collision(pdf_destination.display().to_string()));
        }
    }
    if !paths.destination.exists() {
        return Ok(());
    }
    if !paths.destination.is_dir() {
        return Err(AppError::Collision(paths.destination.display().to_string()));
    }
    if fs::read_dir(&paths.destination)
        .map_err(|error| AppError::io(&paths.destination, error))?
        .next()
        .is_some()
    {
        return Err(AppError::Collision(paths.destination.display().to_string()));
    }
    state.destination_started_empty = true;
    fs::remove_dir(&paths.destination).map_err(|error| AppError::io(&paths.destination, error))?;
    Ok(())
}

fn rollback_publication(
    cause: AppError,
    paths: &PublicationPaths,
    state: &PublicationState,
) -> Result<()> {
    let mut rollback_errors = Vec::new();
    for pdf_destination in state.pdf_published.iter().rev() {
        if let Err(error) = fs::remove_file(pdf_destination) {
            rollback_errors.push(format!("PDF cleanup failed: {error}"));
        }
    }
    if state.certificate_published && paths.destination.exists() {
        if let Err(error) = fs::rename(&paths.destination, &paths.staged_certificate_dir) {
            rollback_errors.push(format!("certificate rollback failed: {error}"));
        }
    }
    if rollback_errors.is_empty() && paths.stage.exists() {
        if let Err(error) = fs::remove_dir_all(&paths.stage) {
            rollback_errors.push(format!("staging cleanup failed: {error}"));
        }
    }
    if rollback_errors.is_empty() && state.destination_started_empty && !paths.destination.exists()
    {
        if let Err(error) = fs::create_dir(&paths.destination) {
            rollback_errors.push(format!(
                "empty certificate directory recovery failed: {error}"
            ));
        }
    }
    if rollback_errors.is_empty() {
        return Err(cause);
    }
    Err(AppError::Data(format!(
        "Certificate publication failed ({cause}); {}",
        rollback_errors.join("; ")
    )))
}

pub(super) fn verify_staged_set(track_root: &Path, stage: &Path) -> Result<()> {
    let certificate_stage = stage.join("certificate");
    let hashes_path = certificate_stage.join("CERTIFICATE_SHA256.txt");
    let content =
        fs::read_to_string(&hashes_path).map_err(|error| AppError::io(&hashes_path, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let verified_pdfs = verify_staged_hash_entries(track_root, stage, &certificate_stage, &hashes)?;
    verify_required_staged_pdfs(&certificate_stage, &hashes, &verified_pdfs)
}

fn verify_staged_hash_entries(
    track_root: &Path,
    stage: &Path,
    certificate_stage: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut verified_pdfs = BTreeMap::new();
    for (relative, expected) in hashes {
        let path = match relative.as_str() {
            HASH_FILE => contained_path(track_root, Path::new(HASH_FILE), true)?,
            MANIFEST_FILE => certificate_stage.join("EVIDENCE_MANIFEST.json"),
            CERTIFICATE_FILE => certificate_stage.join("DOCUMENTATION_CERTIFICATE.md"),
            PDF_FILE => stage.join(PDF_FILE),
            PDF_FILE_DE => stage.join(PDF_FILE_DE),
            _ => return Err(AppError::Data("Unexpected certificate hash entry.".into())),
        };
        let actual = if is_certificate_pdf_path(relative) {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdfs.insert(relative.clone(), bytes);
            digest
        } else {
            sha256_file(&path)?
        };
        if actual != *expected {
            return Err(AppError::Validation(format!(
                "Staged certificate integrity mismatch: {relative}"
            )));
        }
    }
    Ok(verified_pdfs)
}

fn verify_required_staged_pdfs(
    certificate_stage: &Path,
    hashes: &BTreeMap<String, String>,
    verified_pdfs: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let manifest_path = certificate_stage.join("EVIDENCE_MANIFEST.json");
    let requires_pdfa_2b = certificate_format_requires_pdfa_2b(&manifest_path)?;
    let required_pdfs = required_certificate_pdf_paths(&manifest_path, hashes)?;
    if required_pdfs.is_empty() {
        return Err(AppError::Validation(
            "A newly generated certificate must use the PDF certificate format.".into(),
        ));
    }
    for &pdf_path in required_pdfs {
        let bytes = verified_pdfs.get(pdf_path).ok_or_else(|| {
            AppError::Validation(format!(
                "Staged certificate PDF hash entry was not verified: {pdf_path}"
            ))
        })?;
        if requires_pdfa_2b {
            certificate_pdf::validate_pdfa_2b_bytes(bytes)?;
        } else {
            certificate_pdf::validate_pdf_bytes(bytes)?;
        }
    }
    Ok(())
}

pub(super) fn parse_certificate_hashes(content: &str) -> Result<BTreeMap<String, String>> {
    let legacy_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE];
    let single_pdf_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE, PDF_FILE];
    let dual_pdf_paths = [
        HASH_FILE,
        MANIFEST_FILE,
        CERTIFICATE_FILE,
        PDF_FILE,
        PDF_FILE_DE,
    ];
    let expected_paths = [
        HASH_FILE,
        MANIFEST_FILE,
        CERTIFICATE_FILE,
        PDF_FILE,
        PDF_FILE_DE,
    ];
    let mut result = BTreeMap::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.is_empty() {
            return Err(AppError::Data(format!(
                "Empty certificate hash line {}.",
                line_number + 1
            )));
        }
        let (digest, relative) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!(
                "Invalid certificate hash line {}.",
                line_number + 1
            ))
        })?;
        validate_digest(digest, line_number + 1)?;
        if !expected_paths.contains(&relative) {
            return Err(AppError::Data(format!(
                "Unexpected certificate hash path on line {}.",
                line_number + 1
            )));
        }
        if result
            .insert(relative.to_owned(), digest.to_ascii_lowercase())
            .is_some()
        {
            return Err(AppError::Data(format!(
                "Duplicate certificate hash path: {relative}"
            )));
        }
    }
    let is_legacy_set = result.len() == legacy_paths.len()
        && legacy_paths.iter().all(|path| result.contains_key(*path));
    let is_single_pdf_set = result.len() == single_pdf_paths.len()
        && single_pdf_paths
            .iter()
            .all(|path| result.contains_key(*path));
    let is_dual_pdf_set = result.len() == dual_pdf_paths.len()
        && dual_pdf_paths.iter().all(|path| result.contains_key(*path));
    if !is_legacy_set && !is_single_pdf_set && !is_dual_pdf_set {
        return Err(AppError::Validation(
            "Certificate hash set is incomplete.".into(),
        ));
    }
    Ok(result)
}
