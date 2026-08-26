use super::*;

pub fn verify(track_root: &Path) -> Result<()> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|e| AppError::io(&sums, e))?;
    let hashes = parse_certificate_hashes(&content)?;
    let mut verified_pdfs = BTreeMap::new();
    for (relative, expected) in &hashes {
        let path = contained_path(track_root, Path::new(relative), true)?;
        let actual = if is_certificate_pdf_path(relative) {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdfs.insert(relative.as_str(), bytes);
            digest
        } else {
            sha256_file(&path)?
        };
        if actual != *expected {
            return Err(AppError::Validation(format!(
                "Certificate integrity mismatch: {relative}"
            )));
        }
    }
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    let requires_pdfa_2b = certificate_format_requires_pdfa_2b(&manifest)?;
    for pdf_path in required_certificate_pdf_paths(&manifest, &hashes)? {
        let bytes = verified_pdfs.get(pdf_path).ok_or_else(|| {
            AppError::Validation(format!(
                "Certificate PDF hash entry was not verified: {pdf_path}"
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

/// Returns whether the live certificate format requires the root-level PDF.
///
/// This intentionally performs only the format/hash-set inspection needed by
/// interrupted-revision recovery. Full integrity validation remains in
/// [`verify`]. Legacy certificates without a format version use the historical
/// three-entry set and do not trigger PDF recovery.
pub(crate) fn expects_pdf(track_root: &Path) -> Result<bool> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|error| AppError::io(&sums, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    certificate_format_requires_pdf(&manifest, &hashes)
}

pub(crate) fn required_pdf_files(track_root: &Path) -> Result<Vec<&'static str>> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|error| AppError::io(&sums, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let manifest = contained_path(track_root, Path::new(MANIFEST_FILE), true)?;
    Ok(required_certificate_pdf_paths(&manifest, &hashes)?.to_vec())
}
