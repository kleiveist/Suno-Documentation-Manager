use super::*;

pub fn finalization_anchors(track_root: &Path) -> Result<Vec<FinalizationAnchor>> {
    let definitions = [
        (
            TimestampReferencedArtifact::EvidenceManifest,
            "Evidence manifest (recommended timestamp anchor)",
            certificate::MANIFEST_FILE,
        ),
        (
            TimestampReferencedArtifact::Sha256sums,
            "Track SHA-256 manifest",
            integrity::HASH_FILE,
        ),
        (
            TimestampReferencedArtifact::DocumentationCertificateMarkdown,
            "Documentation certificate (Markdown)",
            certificate::CERTIFICATE_FILE,
        ),
        (
            TimestampReferencedArtifact::CertificatePdf,
            "Documentation certificate (English PDF)",
            certificate::PDF_FILE,
        ),
        (
            TimestampReferencedArtifact::FinalEvidencePackage,
            "Final evidence package certificate hash set",
            certificate::CERTIFICATE_HASH_FILE,
        ),
    ];
    definitions
        .into_iter()
        .map(|(artifact, label, relative)| {
            let path = contained_path(track_root, Path::new(relative), true)?;
            Ok(FinalizationAnchor {
                artifact,
                label: label.into(),
                relative_path: relative.into(),
                sha256: sha256_file(&path)?,
            })
        })
        .collect()
}

/// Resolve the one automatic timestamp anchor from the immutable phase-one
/// certificate hash set, then rehash the live manifest before any provider
/// request is made. This deliberately does not accept a UI-selected hash.
pub fn finalized_manifest_anchor(track_root: &Path) -> Result<FinalizationAnchor> {
    let certificate_hashes = contained_path(
        track_root,
        Path::new(certificate::CERTIFICATE_HASH_FILE),
        true,
    )?;
    let content = fs::read_to_string(&certificate_hashes)
        .map_err(|error| AppError::io(&certificate_hashes, error))?;
    let mut expected = None;
    for (index, line) in content.lines().enumerate() {
        let (digest, path) = line.split_once("  ").ok_or_else(|| {
            AppError::Data(format!(
                "Invalid finalized certificate hash entry on line {}.",
                index + 1
            ))
        })?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::Data(format!(
                "Invalid finalized certificate digest on line {}.",
                index + 1
            )));
        }
        if path == certificate::MANIFEST_FILE
            && expected.replace(digest.to_ascii_lowercase()).is_some()
        {
            return Err(AppError::Data(
                "Finalized certificate hash set contains the evidence manifest more than once."
                    .into(),
            ));
        }
    }
    let expected = expected.ok_or_else(|| {
        AppError::Data(
            "Finalized certificate hash set does not contain EVIDENCE_MANIFEST.json.".into(),
        )
    })?;
    let manifest = contained_path(track_root, Path::new(certificate::MANIFEST_FILE), true)?;
    let actual = sha256_file(&manifest)?;
    if actual != expected {
        return Err(AppError::Validation(
            "INTEGRITY CHECK FAILED: The selected timestamp anchor no longer matches the finalized snapshot."
                .into(),
        ));
    }
    Ok(FinalizationAnchor {
        artifact: TimestampReferencedArtifact::EvidenceManifest,
        label: "Evidence manifest (recommended timestamp anchor)".into(),
        relative_path: certificate::MANIFEST_FILE.into(),
        sha256: expected,
    })
}

pub(super) fn referenced_artifact_path(input: &ExternalTimestampInput) -> Result<PathBuf> {
    let fixed = match input.referenced_artifact {
        TimestampReferencedArtifact::EvidenceManifest => Some(certificate::MANIFEST_FILE),
        TimestampReferencedArtifact::Sha256sums => Some(integrity::HASH_FILE),
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            Some(certificate::CERTIFICATE_FILE)
        }
        TimestampReferencedArtifact::CertificatePdf => Some(certificate::PDF_FILE),
        TimestampReferencedArtifact::FinalEvidencePackage => {
            Some(certificate::CERTIFICATE_HASH_FILE)
        }
        TimestampReferencedArtifact::Other => None,
    };
    if let Some(relative) = fixed {
        return Ok(PathBuf::from(relative));
    }
    let relative = PathBuf::from(input.other_referenced_artifact.trim());
    validate_stable_artifact_relative(&relative)?;
    Ok(relative)
}

pub(super) fn validate_stable_artifact_relative(relative: &Path) -> Result<()> {
    validate_relative(relative)?;
    let portable = portable_relative(relative);
    if portable.contains('\\')
        || portable.chars().any(char::is_control)
        || portable == ".archive"
        || portable.starts_with(".archive/")
        || portable == EXTERNAL_TIMESTAMPS_DIR
        || portable.starts_with(&format!("{EXTERNAL_TIMESTAMPS_DIR}/"))
    {
        return Err(AppError::Validation(
            "Other timestamp artifacts must identify a stable phase-one track file.".into(),
        ));
    }
    Ok(())
}

pub(super) fn record_directory(record: &ExternalTimestampRecord) -> Result<PathBuf> {
    Uuid::parse_str(&record.id)
        .map_err(|_| AppError::Validation("Timestamp record ID is invalid.".into()))?;
    let directory = PathBuf::from(EXTERNAL_TIMESTAMPS_DIR).join(&record.id);
    validate_relative(&directory)?;
    for (actual, expected) in [
        (&record.record_relative_path, directory.join(RECORD_FILE)),
        (
            &record.markdown_relative_path,
            directory.join(MARKDOWN_FILE),
        ),
        (&record.pdf_relative_path, directory.join(PDF_FILE)),
        (
            &record.hash_list_relative_path,
            directory.join(HASH_LIST_FILE),
        ),
    ] {
        if actual != &portable_relative(&expected) {
            return Err(AppError::Validation(
                "Timestamp record contains an inconsistent managed path.".into(),
            ));
        }
    }
    Ok(directory)
}

pub(super) fn validate_input(input: &ExternalTimestampInput) -> Result<()> {
    for (name, value, max, required) in [
        (
            "Timestamp provider / issuer",
            input.provider.as_str(),
            1000,
            true,
        ),
        (
            "Timestamp value",
            input.timestamp_value.as_str(),
            500,
            false,
        ),
        (
            "Other referenced artifact",
            input.other_referenced_artifact.as_str(),
            4000,
            input.referenced_artifact == TimestampReferencedArtifact::Other,
        ),
        (
            "External reference ID",
            input.external_reference_id.as_str(),
            1000,
            false,
        ),
        (
            "Provider verification URL",
            input.provider_verification_url.as_str(),
            4000,
            false,
        ),
        ("Timestamp note", input.note.as_str(), 20_000, false),
    ] {
        if required && value.trim().is_empty() {
            return Err(AppError::Validation(format!("{name} is required.")));
        }
        if value.len() > max || value.chars().any(|character| character == '\0') {
            return Err(AppError::Validation(format!(
                "{name} is invalid or too long."
            )));
        }
    }
    let digest = input.referenced_sha256.trim();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Validation(
            "Referenced hash must be a SHA-256 value.".into(),
        ));
    }
    if !input.provider_verification_url.trim().is_empty() {
        let parsed = Url::parse(input.provider_verification_url.trim())
            .map_err(|_| AppError::Validation("Provider verification URL is invalid.".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(AppError::Validation(
                "Provider verification URL must be an HTTP(S) URL with a host.".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn verify_staged_hashes(
    directory: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<()> {
    for (name, expected) in hashes {
        let actual = sha256_file(&directory.join(name))?;
        if &actual != expected {
            return Err(AppError::Validation(format!(
                "External timestamp addendum integrity mismatch: {name}"
            )));
        }
    }
    Ok(())
}
