use crate::certificate_pdf::{self, CertificatePdfSnapshot};
use crate::error::{AppError, Result};
use crate::integrity::HASH_FILE;
use crate::model::{
    BlockingDeviation, EvidenceItem, EvidenceProvenance, Profile, StepState, StepStatus,
    TrackRecord,
};
use crate::security::{
    atomic_write_new, contained_path, copy_new, ensure_contained_directory, portable_relative,
    sha256_bytes, sha256_file,
};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const CERTIFICATE_DIR: &str = "06_CERTIFICATE";
pub const CERTIFICATE_FILE: &str = "06_CERTIFICATE/DOCUMENTATION_CERTIFICATE.md";
pub const MANIFEST_FILE: &str = "06_CERTIFICATE/EVIDENCE_MANIFEST.json";
pub const CERTIFICATE_HASH_FILE: &str = "06_CERTIFICATE/CERTIFICATE_SHA256.txt";
pub const PDF_FILE: &str = "SunoDM_DOCUMENTATION_CERTIFICATE.pdf";
pub const CERTIFICATE_FORMAT_VERSION: &str = "2.0";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEvidence<'a> {
    id: &'a str,
    role: &'a str,
    file_name: &'a str,
    relative_path: &'a str,
    sha256: Option<&'a str>,
    size_bytes: u64,
    imported_at: &'a str,
    source_global_evidence_id: Option<&'a str>,
    coverage_start: Option<&'a str>,
    coverage_end: Option<&'a str>,
    provenance: &'a EvidenceProvenance,
    derived_from_evidence_id: Option<&'a str>,
    generator_version: Option<&'a str>,
    generated_disclosure_text: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
pub fn generate(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
) -> Result<()> {
    generate_impl(
        track_root,
        track,
        profile,
        steps,
        evidence,
        deviations,
        certificate_id,
        finalized_at,
        transaction_id,
        #[cfg(test)]
        None,
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CertificateGenerationFailure {
    PdfGeneration,
    PdfStaging,
    PdfPublication,
    PostPublishVerification,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_with_failure(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
    failure: CertificateGenerationFailure,
) -> Result<()> {
    generate_impl(
        track_root,
        track,
        profile,
        steps,
        evidence,
        deviations,
        certificate_id,
        finalized_at,
        transaction_id,
        Some(failure),
    )
}

#[allow(clippy::too_many_arguments)]
fn generate_impl(
    track_root: &Path,
    track: &TrackRecord,
    profile: &Profile,
    steps: &[StepState],
    evidence: &[EvidenceItem],
    deviations: &[BlockingDeviation],
    certificate_id: &str,
    finalized_at: &str,
    transaction_id: &str,
    #[cfg(test)] failure: Option<CertificateGenerationFailure>,
) -> Result<()> {
    let hash_manifest = contained_path(track_root, Path::new(HASH_FILE), true)?;
    let hash_manifest_sha = sha256_file(&hash_manifest)?;
    let hashes = parse_hashes(&hash_manifest)?;
    let mut evidence_values = evidence
        .iter()
        .filter(|item| item.verified && item.sha256.is_some() && item.verification_error.is_none())
        .collect::<Vec<_>>();
    evidence_values.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let evidence_manifest = evidence_values
        .iter()
        .map(|item| ManifestEvidence {
            id: &item.id,
            role: item.role.as_str(),
            file_name: &item.file_name,
            relative_path: &item.relative_path,
            sha256: item.sha256.as_deref(),
            size_bytes: item.size_bytes,
            imported_at: &item.imported_at,
            source_global_evidence_id: item.source_global_evidence_id.as_deref(),
            coverage_start: item.coverage_start.as_deref(),
            coverage_end: item.coverage_end.as_deref(),
            provenance: &item.provenance,
            derived_from_evidence_id: item.derived_from_evidence_id.as_deref(),
            generator_version: item.generator_version.as_deref(),
            generated_disclosure_text: item.generated_disclosure_text.as_deref(),
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schema_version": 1,
        "track": {
            "id": track.id,
            "title": track.fields.title,
            "relative_path": ".",
            "production_start_date": track.fields.production_start_date,
            "production_end_date": track.fields.production_end_date,
            "final_export_date": track.fields.final_export_date,
        },
        "artist": {
            "name": profile.artist_name,
            "suno_profile_name": profile.suno_profile_name,
            "suno_handle": profile.suno_handle,
        },
        "workflow": {
            "id": track.workflow_id,
            "version": track.workflow_version,
            "application_version": env!("CARGO_PKG_VERSION"),
        },
        "finalization": {
            "timestamp": finalized_at,
            "result": "DOCUMENTATION COMPLETE",
        },
        "steps": steps,
        "evidence": evidence_manifest,
        "hashes": hashes,
        "certificate": {
            "id": certificate_id,
            "format_version": CERTIFICATE_FORMAT_VERSION,
            "status": "DOCUMENTATION COMPLETE",
            "sha256sums_sha256": hash_manifest_sha,
        },
        "deviations": deviations,
    });
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    manifest_bytes.push(b'\n');
    let manifest_sha = sha256_bytes(&manifest_bytes);

    let release_wav = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::ReleaseWav && item.verified)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("Not documented");
    let final_artwork = evidence
        .iter()
        .find(|item| item.role == crate::model::EvidenceRole::FinalArtwork && item.verified)
        .and_then(|item| item.sha256.as_deref())
        .unwrap_or("N/A");
    let na_steps = steps
        .iter()
        .filter(|step| step.status == StepStatus::NotApplicable)
        .map(|step| {
            format!(
                "- {} — {}\n",
                step.id,
                step.na_reason.as_deref().unwrap_or("No reason")
            )
        })
        .collect::<String>();
    let completed_steps = steps
        .iter()
        .filter(|step| matches!(step.status, StepStatus::Pass | StepStatus::NotApplicable))
        .map(|step| format!("- {}: {:?}\n", step.id, step.status))
        .collect::<String>();
    let open_blocking = deviations
        .iter()
        .filter(|d| d.blocking && !d.resolved)
        .count();
    let certificate = format!(
        "# Track Documentation Completion Certificate\n\n- Certificate ID: `{certificate_id}`\n- Certificate format version: `{CERTIFICATE_FORMAT_VERSION}`\n- Track: {}\n- Artist: {}\n- Workflow ID: `{}`\n- Workflow version: `{}`\n- Application version: `{}`\n- Finalization timestamp: `{finalized_at}`\n- Evidence file count: {}\n- Release WAV SHA-256: `{release_wav}`\n- Final artwork SHA-256: `{final_artwork}`\n- SHA256SUMS.txt SHA-256: `{hash_manifest_sha}`\n- Evidence manifest SHA-256: `{manifest_sha}`\n- Blocking deviations: {open_blocking}\n- Final result: **DOCUMENTATION COMPLETE**\n\n## Mandatory steps completed\n\n{completed_steps}\n## N/A steps with reasons\n\n{}\n## Scope and disclaimer\n\nThis certificate confirms completion of the configured\ndocumentation workflow and integrity checks.\n\nIt does not constitute governmental certification,\nlegal advice, or an independent determination of\ncopyright ownership or legal compliance.\n",
        track.fields.title,
        profile.artist_name,
        track.workflow_id,
        track.workflow_version,
        env!("CARGO_PKG_VERSION"),
        evidence_values.len(),
        if na_steps.is_empty() { "- None\n" } else { &na_steps }
    );
    let certificate_sha = sha256_bytes(certificate.as_bytes());

    #[cfg(test)]
    if failure == Some(CertificateGenerationFailure::PdfGeneration) {
        return Err(AppError::Data(
            "Injected technical PDF generation failure.".into(),
        ));
    }
    let pdf = certificate_pdf::generate_pdf(&CertificatePdfSnapshot {
        track,
        profile,
        steps,
        evidence: &evidence_values,
        deviations,
        certificate_id,
        finalized_at,
        certificate_version: CERTIFICATE_FORMAT_VERSION,
        sha256sums_sha256: &hash_manifest_sha,
        evidence_manifest_sha256: &manifest_sha,
        markdown_certificate_sha256: &certificate_sha,
    })?;
    let pdf_sha = sha256_bytes(&pdf);
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
        hash_manifest_sha,
        HASH_FILE,
        manifest_sha,
        MANIFEST_FILE,
        certificate_sha,
        CERTIFICATE_FILE,
        pdf_sha,
        PDF_FILE,
    );
    publish_certificate_set_impl(
        track_root,
        &manifest_bytes,
        certificate.as_bytes(),
        &pdf,
        certificate_hashes.as_bytes(),
        transaction_id,
        #[cfg(test)]
        failure.and_then(CertificateGenerationFailure::publication_failure),
    )?;
    Ok(())
}

pub fn verify(track_root: &Path) -> Result<()> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|e| AppError::io(&sums, e))?;
    let hashes = parse_certificate_hashes(&content)?;
    let mut verified_pdf = None;
    for (relative, expected) in &hashes {
        let path = contained_path(track_root, Path::new(relative), true)?;
        let actual = if relative == PDF_FILE {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdf = Some(bytes);
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
    if certificate_format_requires_pdf(
        &contained_path(track_root, Path::new(MANIFEST_FILE), true)?,
        &hashes,
    )? {
        certificate_pdf::validate_pdf_bytes(verified_pdf.as_deref().ok_or_else(|| {
            AppError::Validation("Certificate PDF hash entry was not verified.".into())
        })?)?;
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

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CertificatePublicationFailure {
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
    fn label(self) -> &'static str {
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

    fn stage_id(self) -> String {
        format!("failure-injection-{}", self.label())
    }
}

#[cfg(test)]
impl CertificateGenerationFailure {
    fn publication_failure(self) -> Option<CertificatePublicationFailure> {
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
fn inject_certificate_publication_failure(
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

fn publish_certificate_set_impl(
    track_root: &Path,
    manifest: &[u8],
    certificate: &[u8],
    pdf: &[u8],
    certificate_hashes: &[u8],
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
    let staged_certificate_dir = stage.join("certificate");
    let destination = contained_path(track_root, Path::new(CERTIFICATE_DIR), false)?;
    let pdf_destination = contained_path(track_root, Path::new(PDF_FILE), false)?;
    let mut destination_started_empty = false;
    let mut certificate_published = false;
    let mut pdf_published = false;
    let publish_result = (|| -> Result<()> {
        fs::create_dir(&staged_certificate_dir)
            .map_err(|error| AppError::io(&staged_certificate_dir, error))?;
        let staged_manifest = staged_certificate_dir.join("EVIDENCE_MANIFEST.json");
        let staged_certificate = staged_certificate_dir.join("DOCUMENTATION_CERTIFICATE.md");
        let staged_hashes = staged_certificate_dir.join("CERTIFICATE_SHA256.txt");
        let staged_pdf = stage.join(PDF_FILE);
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::ManifestWrite,
        )?;
        atomic_write_new(&staged_manifest, manifest)?;
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificateWrite,
        )?;
        atomic_write_new(&staged_certificate, certificate)?;
        #[cfg(test)]
        inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfWrite)?;
        atomic_write_new(&staged_pdf, pdf)?;
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificateHashWrite,
        )?;
        atomic_write_new(&staged_hashes, certificate_hashes)?;
        verify_staged_set(track_root, &stage)?;

        if pdf_destination.exists() {
            return Err(AppError::Collision(pdf_destination.display().to_string()));
        }
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificatePublishRename,
        )?;
        if destination.exists() {
            if !destination.is_dir() {
                return Err(AppError::Collision(destination.display().to_string()));
            }
            if fs::read_dir(&destination)
                .map_err(|error| AppError::io(&destination, error))?
                .next()
                .is_some()
            {
                return Err(AppError::Collision(destination.display().to_string()));
            }
            destination_started_empty = true;
            fs::remove_dir(&destination).map_err(|error| AppError::io(&destination, error))?;
        }
        fs::rename(&staged_certificate_dir, &destination)
            .map_err(|error| AppError::io(&destination, error))?;
        certificate_published = true;
        #[cfg(test)]
        inject_certificate_publication_failure(failure, CertificatePublicationFailure::PdfPublish)?;
        copy_new(&staged_pdf, &pdf_destination)?;
        pdf_published = true;
        #[cfg(test)]
        let post_publish_verification = inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::PostPublishVerification,
        )
        .and_then(|()| verify(track_root));
        #[cfg(not(test))]
        let post_publish_verification = verify(track_root);
        post_publish_verification?;
        fs::remove_dir_all(&stage).map_err(|error| AppError::io(&stage, error))?;
        Ok(())
    })();

    if let Err(cause) = publish_result {
        let mut rollback_errors = Vec::new();
        if pdf_published && pdf_destination.exists() {
            if let Err(error) = fs::remove_file(&pdf_destination) {
                rollback_errors.push(format!("PDF cleanup failed: {error}"));
            }
        }
        if certificate_published && destination.exists() {
            if let Err(error) = fs::rename(&destination, &staged_certificate_dir) {
                rollback_errors.push(format!("certificate rollback failed: {error}"));
            }
        }
        if rollback_errors.is_empty() && stage.exists() {
            if let Err(error) = fs::remove_dir_all(&stage) {
                rollback_errors.push(format!("staging cleanup failed: {error}"));
            }
        }
        if rollback_errors.is_empty() && destination_started_empty && !destination.exists() {
            if let Err(error) = fs::create_dir(&destination) {
                rollback_errors.push(format!(
                    "empty certificate directory recovery failed: {error}"
                ));
            }
        }
        if rollback_errors.is_empty() {
            return Err(cause);
        }
        return Err(AppError::Data(format!(
            "Certificate publication failed ({cause}); {}",
            rollback_errors.join("; ")
        )));
    }

    Ok(())
}

fn verify_staged_set(track_root: &Path, stage: &Path) -> Result<()> {
    let certificate_stage = stage.join("certificate");
    let hashes_path = certificate_stage.join("CERTIFICATE_SHA256.txt");
    let content =
        fs::read_to_string(&hashes_path).map_err(|error| AppError::io(&hashes_path, error))?;
    let hashes = parse_certificate_hashes(&content)?;
    let mut verified_pdf = None;
    for (relative, expected) in &hashes {
        let path = match relative.as_str() {
            HASH_FILE => contained_path(track_root, Path::new(HASH_FILE), true)?,
            MANIFEST_FILE => certificate_stage.join("EVIDENCE_MANIFEST.json"),
            CERTIFICATE_FILE => certificate_stage.join("DOCUMENTATION_CERTIFICATE.md"),
            PDF_FILE => stage.join(PDF_FILE),
            _ => return Err(AppError::Data("Unexpected certificate hash entry.".into())),
        };
        let actual = if relative == PDF_FILE {
            let bytes = fs::read(&path).map_err(|error| AppError::io(&path, error))?;
            let digest = sha256_bytes(&bytes);
            verified_pdf = Some(bytes);
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
    if !certificate_format_requires_pdf(&certificate_stage.join("EVIDENCE_MANIFEST.json"), &hashes)?
    {
        return Err(AppError::Validation(
            "A newly generated certificate must use the PDF certificate format.".into(),
        ));
    }
    certificate_pdf::validate_pdf_bytes(verified_pdf.as_deref().ok_or_else(|| {
        AppError::Validation("Staged certificate PDF hash entry was not verified.".into())
    })?)?;
    Ok(())
}

fn parse_certificate_hashes(content: &str) -> Result<BTreeMap<String, String>> {
    let legacy_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE];
    let expected_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE, PDF_FILE];
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
    let is_current_set = result.len() == expected_paths.len()
        && expected_paths.iter().all(|path| result.contains_key(*path));
    if !is_legacy_set && !is_current_set {
        return Err(AppError::Validation(
            "Certificate hash set is incomplete.".into(),
        ));
    }
    Ok(result)
}

fn certificate_format_requires_pdf(
    manifest_path: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<bool> {
    let bytes = fs::read(manifest_path).map_err(|error| AppError::io(manifest_path, error))?;
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)?;
    let format_version = manifest
        .get("certificate")
        .and_then(|certificate| certificate.get("format_version"))
        .and_then(serde_json::Value::as_str);
    match format_version {
        Some(CERTIFICATE_FORMAT_VERSION) => {
            if !hashes.contains_key(PDF_FILE) {
                return Err(AppError::Validation(
                    "Certificate format 2.0 requires the root-level technical PDF hash.".into(),
                ));
            }
            Ok(true)
        }
        Some(version) => Err(AppError::Validation(format!(
            "Unsupported certificate format version: {version}"
        ))),
        None => {
            if hashes.contains_key(PDF_FILE) {
                return Err(AppError::Validation(
                    "A legacy certificate cannot contain an unversioned PDF hash entry.".into(),
                ));
            }
            Ok(false)
        }
    }
}

fn parse_hashes(path: &Path) -> Result<BTreeMap<String, String>> {
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

fn validate_digest(digest: &str, line_number: usize) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Data(format!(
            "Invalid SHA-256 digest on line {line_number}."
        )));
    }
    Ok(())
}

fn hash_manifest_path_allowed(relative: &Path) -> bool {
    if relative == Path::new(HASH_FILE) || relative == Path::new(PDF_FILE) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn parse_main_hash_fixture(content: &str) -> Result<BTreeMap<String, String>> {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let sums = workspace.path().join("SHA256SUMS.txt");
        fs::write(&sums, content).expect("write SHA256SUMS fixture");
        parse_hashes(&sums)
    }

    fn publication_fixture(track_root: &Path) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        let main_hashes = b"fixture main hash manifest\n";
        let main_hash_path = track_root.join(HASH_FILE);
        fs::create_dir_all(
            main_hash_path
                .parent()
                .expect("main hash manifest parent directory"),
        )
        .expect("create documentation fixture directory");
        fs::write(&main_hash_path, main_hashes).expect("write main hash manifest fixture");
        let manifest = format!(
            "{{\"certificate\":{{\"format_version\":\"{CERTIFICATE_FORMAT_VERSION}\"}},\"fixture\":true}}\n"
        )
        .into_bytes();
        let certificate = b"# Fixture certificate\n".to_vec();
        let mut pdf_document = printpdf::PdfDocument::new("Fixture certificate");
        let pdf = pdf_document
            .with_pages(vec![printpdf::PdfPage::new(
                printpdf::Mm(210.0),
                printpdf::Mm(297.0),
                Vec::new(),
            )])
            .save(&printpdf::PdfSaveOptions::default(), &mut Vec::new());
        let certificate_hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
            sha256_bytes(main_hashes),
            HASH_FILE,
            sha256_bytes(&manifest),
            MANIFEST_FILE,
            sha256_bytes(&certificate),
            CERTIFICATE_FILE,
            sha256_bytes(&pdf),
            PDF_FILE,
        )
        .into_bytes();
        (manifest, certificate, pdf, certificate_hashes)
    }

    fn assert_injected_publication_failure(failure: CertificatePublicationFailure) {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let track_root = workspace.path();
        let (manifest, certificate, pdf, certificate_hashes) = publication_fixture(track_root);
        let live = track_root.join(CERTIFICATE_DIR);
        fs::create_dir(&live).expect("create empty live certificate directory");
        let correlated_stage = track_root
            .join(".archive")
            .join("certificate-staging")
            .join(failure.stage_id());

        let error = publish_certificate_set_impl(
            track_root,
            &manifest,
            &certificate,
            &pdf,
            &certificate_hashes,
            &failure.stage_id(),
            Some(failure),
        )
        .expect_err("injected publication failure");

        assert_eq!(
            error.to_string(),
            format!(
                "Invalid stored data: Injected certificate publication failure at {}.",
                failure.label()
            )
        );
        assert!(
            verify(track_root).is_err(),
            "incomplete live certificate unexpectedly verified after {} failure",
            failure.label()
        );
        assert!(
            live.is_dir(),
            "empty live certificate directory was removed"
        );
        assert!(
            fs::read_dir(&live)
                .expect("read restored certificate directory")
                .next()
                .is_none(),
            "live certificate directory is not empty after {} failure",
            failure.label()
        );
        assert!(
            !track_root.join(PDF_FILE).exists(),
            "live PDF remains after {} failure",
            failure.label()
        );
        assert!(
            !correlated_stage.exists(),
            "correlated staging directory was not cleaned after {} failure",
            failure.label()
        );
        let staging_parent = track_root.join(".archive/certificate-staging");
        assert!(
            !staging_parent.exists()
                || fs::read_dir(&staging_parent)
                    .expect("read certificate staging directory")
                    .next()
                    .is_none(),
            "certificate staging contains residue after {} failure",
            failure.label()
        );
    }

    #[test]
    fn staging_directory_creation_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::StagingDirectoryCreate);
    }

    #[test]
    fn manifest_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::ManifestWrite);
    }

    #[test]
    fn certificate_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::CertificateWrite);
    }

    #[test]
    fn pdf_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::PdfWrite);
    }

    #[test]
    fn certificate_hash_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::CertificateHashWrite);
    }

    #[test]
    fn certificate_publish_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(
            CertificatePublicationFailure::CertificatePublishRename,
        );
    }

    #[test]
    fn pdf_publish_failure_rolls_back_certificate_and_cleans_staging() {
        assert_injected_publication_failure(CertificatePublicationFailure::PdfPublish);
    }

    #[test]
    fn post_publish_verification_failure_rolls_back_and_cleans_staging() {
        assert_injected_publication_failure(CertificatePublicationFailure::PostPublishVerification);
    }

    #[test]
    fn certificate_hash_parser_requires_exact_complete_unique_set() {
        let legacy = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&legacy)
                .expect("valid legacy set")
                .len(),
            3
        );

        let valid = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&valid).expect("valid set").len(),
            4
        );

        let duplicate = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n{DIGEST}  {PDF_FILE}\n"
        );
        assert!(parse_certificate_hashes(&duplicate).is_err());

        let invalid_digest = format!(
            "short  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {PDF_FILE}\n"
        );
        assert!(parse_certificate_hashes(&invalid_digest).is_err());
    }

    #[test]
    fn legacy_certificate_without_pdf_remains_verifiable() {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let track_root = workspace.path();
        let main_hash_path = track_root.join(HASH_FILE);
        fs::create_dir_all(main_hash_path.parent().expect("main hash parent"))
            .expect("documentation directory");
        fs::write(&main_hash_path, b"legacy main hash list\n").expect("main hash fixture");
        let certificate_path = track_root.join(CERTIFICATE_FILE);
        fs::create_dir_all(certificate_path.parent().expect("certificate parent"))
            .expect("certificate directory");
        let manifest_path = track_root.join(MANIFEST_FILE);
        fs::write(&manifest_path, b"{\"certificate\":{}}\n").expect("legacy manifest");
        fs::write(&certificate_path, b"# Legacy certificate\n").expect("legacy certificate");
        let hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n",
            sha256_file(&main_hash_path).expect("main hash digest"),
            HASH_FILE,
            sha256_file(&manifest_path).expect("manifest digest"),
            MANIFEST_FILE,
            sha256_file(&certificate_path).expect("certificate digest"),
            CERTIFICATE_FILE,
        );
        fs::write(track_root.join(CERTIFICATE_HASH_FILE), hashes).expect("legacy hash set");

        verify(track_root).expect("legacy certificate remains valid");
        assert!(!expects_pdf(track_root).expect("legacy format detection"));
    }

    #[test]
    fn evidence_manifest_hash_parser_rejects_duplicates_and_exclusions() {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let sums = workspace.path().join("SHA256SUMS.txt");
        fs::write(
            &sums,
            format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
        )
        .expect("write duplicate sums");
        assert!(parse_hashes(&sums).is_err());

        fs::write(&sums, format!("{DIGEST}  06_CERTIFICATE/hidden.txt\n"))
            .expect("write excluded sums");
        assert!(parse_hashes(&sums).is_err());

        fs::write(&sums, format!("{DIGEST}  {PDF_FILE}\n")).expect("write excluded root PDF");
        assert!(parse_hashes(&sums).is_err());
    }

    #[test]
    fn certificate_hash_parser_rejects_empty_missing_extra_and_unsafe_entries() {
        let invalid_sets = [
            String::new(),
            format!("{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n"),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n{DIGEST}  {CERTIFICATE_HASH_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  ../EVIDENCE_MANIFEST.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  /absolute.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
            format!(
                "{DIGEST}  {HASH_FILE}\n{DIGEST}  06_CERTIFICATE/control\tmanifest.json\n{DIGEST}  {CERTIFICATE_FILE}\n"
            ),
        ];

        for content in invalid_sets {
            assert!(
                parse_certificate_hashes(&content).is_err(),
                "invalid certificate set was accepted: {content:?}"
            );
        }
    }

    #[test]
    fn evidence_manifest_hash_parser_covers_format_and_path_edge_cases() {
        let invalid_entries = [
            String::new(),
            "not-a-digest  01_RELEASE/song.wav\n".into(),
            format!("{DIGEST}  /absolute.wav\n"),
            format!("{DIGEST}  ../escape.wav\n"),
            format!("{DIGEST}  01_RELEASE\\windows.wav\n"),
            format!("{DIGEST}  01_RELEASE/control\tname.wav\n"),
            format!("{DIGEST}  {HASH_FILE}\n"),
            format!("{DIGEST}  .archive/hidden.wav\n"),
            format!("{DIGEST}  .summary/hidden.wav\n"),
            format!("{DIGEST}  .suno-doc/workspace.sqlite\n"),
            format!("{DIGEST}  {CERTIFICATE_FILE}\n"),
            format!("{DIGEST}  01_RELEASE/song.wav\n\n{DIGEST}  02_SUNO/song.wav\n"),
            format!("{DIGEST}  01_RELEASE/song.wav\n{DIGEST}  01_RELEASE/song.wav\n"),
        ];

        for content in invalid_entries {
            assert!(
                parse_main_hash_fixture(&content).is_err(),
                "invalid main hash entry was accepted: {content:?}"
            );
        }

        let uppercase = DIGEST.to_ascii_uppercase();
        let parsed = parse_main_hash_fixture(&format!(
            "{uppercase}  01_RELEASE/song.wav\n{DIGEST}  02_SUNO/source.wav\n"
        ))
        .expect("portable valid hash list");
        assert_eq!(parsed["01_RELEASE/song.wav"], DIGEST);
        assert_eq!(parsed.len(), 2);
    }
}
