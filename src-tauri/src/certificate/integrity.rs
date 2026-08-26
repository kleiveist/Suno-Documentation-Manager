use super::*;

pub(super) fn certificate_format_requires_pdf(
    manifest_path: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<bool> {
    Ok(!required_certificate_pdf_paths(manifest_path, hashes)?.is_empty())
}

pub(super) fn certificate_format_requires_pdfa_2b(manifest_path: &Path) -> Result<bool> {
    let bytes = fs::read(manifest_path).map_err(|error| AppError::io(manifest_path, error))?;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)?;
    let format_version = manifest
        .get("certificate")
        .and_then(|certificate| certificate.get("format_version"))
        .and_then(serde_json::Value::as_str);
    Ok(matches!(
        format_version,
        Some(version) if matches!(version, CERTIFICATE_FORMAT_VERSION | "6.1" | "6.0")
    ))
}

pub(super) fn required_certificate_pdf_paths(
    manifest_path: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<&'static [&'static str]> {
    let bytes = fs::read(manifest_path).map_err(|error| AppError::io(manifest_path, error))?;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)?;
    let format_version = manifest
        .get("certificate")
        .and_then(|certificate| certificate.get("format_version"))
        .and_then(serde_json::Value::as_str);
    match format_version {
        Some(CERTIFICATE_FORMAT_VERSION | "6.1" | "6.0") => {
            if !hashes.contains_key(PDF_FILE) || !hashes.contains_key(PDF_FILE_DE) {
                return Err(AppError::Validation(
                    format!(
                        "Certificate format {CERTIFICATE_FORMAT_VERSION} requires German and English PDF hash entries."
                    ),
                ));
            }
            Ok(&[PDF_FILE, PDF_FILE_DE])
        }
        Some("5.2") => {
            if !hashes.contains_key(PDF_FILE) || !hashes.contains_key(PDF_FILE_DE) {
                return Err(AppError::Validation(
                    "Certificate format 5.2 requires German and English PDF hash entries.".into(),
                ));
            }
            Ok(&[PDF_FILE, PDF_FILE_DE])
        }
        Some("5.1" | "5.0" | "4.1" | "4.0" | "3.0" | "2.0") => {
            if !hashes.contains_key(PDF_FILE) {
                return Err(AppError::Validation(
                    "This certificate format requires the root-level technical PDF hash.".into(),
                ));
            }
            Ok(&[PDF_FILE])
        }
        Some(version) => Err(AppError::Validation(format!(
            "Unsupported certificate format version: {version}"
        ))),
        None => {
            if is_certificate_pdf_path_in_hashes(hashes) {
                return Err(AppError::Validation(
                    "A legacy certificate cannot contain an unversioned PDF hash entry.".into(),
                ));
            }
            Ok(&[])
        }
    }
}

pub(super) fn is_certificate_pdf_path(relative: &str) -> bool {
    matches!(relative, PDF_FILE | PDF_FILE_DE)
}

pub(super) fn is_certificate_pdf_path_in_hashes(hashes: &BTreeMap<String, String>) -> bool {
    hashes.keys().any(|path| is_certificate_pdf_path(path))
}

pub(super) fn parse_hashes(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;
    let mut result = BTreeMap::new();
    for (line_number, line) in content.lines().enumerate() {
        if line.is_empty() {
            return Err(AppError::Data(format!(
                "Empty SHA256SUMS line {}.",
                line_number + 1
            )));
        }
        let (hash, relative) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!("Invalid SHA256SUMS line {}.", line_number + 1))
        })?;
        validate_digest(hash, line_number + 1)?;
        let relative_path = Path::new(relative);
        crate::security::validate_relative(relative_path)?;
        if relative.contains('\\') || relative.chars().any(char::is_control) {
            return Err(AppError::Data(format!(
                "Invalid SHA256SUMS path on line {}.",
                line_number + 1
            )));
        }
        let portable = portable_relative(relative_path);
        if !hash_manifest_path_allowed(relative_path) {
            return Err(AppError::Data(format!(
                "Excluded SHA256SUMS path on line {}.",
                line_number + 1
            )));
        }
        if result
            .insert(portable.clone(), hash.to_ascii_lowercase())
            .is_some()
        {
            return Err(AppError::Data(format!(
                "Duplicate SHA256SUMS path: {portable}"
            )));
        }
    }
    if result.is_empty() {
        return Err(AppError::Validation("SHA256SUMS is empty.".into()));
    }
    Ok(result)
}

pub(super) fn validate_digest(digest: &str, line_number: usize) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Data(format!(
            "Invalid SHA-256 digest on line {line_number}."
        )));
    }
    Ok(())
}

pub(super) fn hash_manifest_path_allowed(relative: &Path) -> bool {
    if relative == Path::new(HASH_FILE)
        || relative == Path::new(PDF_FILE)
        || relative == Path::new(PDF_FILE_DE)
    {
        return false;
    }
    !matches!(
        relative.components().next(),
        Some(std::path::Component::Normal(value))
            if value == ".archive"
                || value == ".summary"
                || value == ".suno-doc"
                || value == CERTIFICATE_DIR
    )
}
