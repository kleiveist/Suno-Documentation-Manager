use crate::error::{AppError, Result};
use crate::integrity::HASH_FILE;
use crate::model::{
    BlockingDeviation, EvidenceItem, EvidenceProvenance, Profile, StepState, StepStatus,
    TrackRecord,
};
use crate::security::{
    atomic_write_new, contained_path, ensure_contained_directory, portable_relative, sha256_bytes,
    sha256_file,
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
        "# Track Documentation Completion Certificate\n\n- Certificate ID: `{certificate_id}`\n- Track: {}\n- Artist: {}\n- Workflow ID: `{}`\n- Workflow version: `{}`\n- Application version: `{}`\n- Finalization timestamp: `{finalized_at}`\n- Evidence file count: {}\n- Release WAV SHA-256: `{release_wav}`\n- Final artwork SHA-256: `{final_artwork}`\n- SHA256SUMS.txt SHA-256: `{hash_manifest_sha}`\n- Evidence manifest SHA-256: `{manifest_sha}`\n- Blocking deviations: {open_blocking}\n- Final result: **DOCUMENTATION COMPLETE**\n\n## Mandatory steps completed\n\n{completed_steps}\n## N/A steps with reasons\n\n{}\n## Scope and disclaimer\n\nThis certificate confirms completion of the configured\ndocumentation workflow and integrity checks.\n\nIt does not constitute governmental certification,\nlegal advice, or an independent determination of\ncopyright ownership or legal compliance.\n",
        track.fields.title,
        profile.artist_name,
        track.workflow_id,
        track.workflow_version,
        env!("CARGO_PKG_VERSION"),
        evidence_values.len(),
        if na_steps.is_empty() { "- None\n" } else { &na_steps }
    );
    let certificate_sha = sha256_bytes(certificate.as_bytes());
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n",
        hash_manifest_sha,
        HASH_FILE,
        manifest_sha,
        MANIFEST_FILE,
        certificate_sha,
        CERTIFICATE_FILE
    );
    publish_certificate_set(
        track_root,
        &manifest_bytes,
        certificate.as_bytes(),
        certificate_hashes.as_bytes(),
        transaction_id,
    )?;
    Ok(())
}

pub fn verify(track_root: &Path) -> Result<()> {
    let sums = contained_path(track_root, Path::new(CERTIFICATE_HASH_FILE), true)?;
    let content = fs::read_to_string(&sums).map_err(|e| AppError::io(&sums, e))?;
    for (relative, expected) in parse_certificate_hashes(&content)? {
        let path = contained_path(track_root, Path::new(&relative), true)?;
        if sha256_file(&path)? != expected {
            return Err(AppError::Validation(format!(
                "Certificate integrity mismatch: {relative}"
            )));
        }
    }
    Ok(())
}

fn publish_certificate_set(
    track_root: &Path,
    manifest: &[u8],
    certificate: &[u8],
    certificate_hashes: &[u8],
    transaction_id: &str,
) -> Result<()> {
    publish_certificate_set_impl(
        track_root,
        manifest,
        certificate,
        certificate_hashes,
        transaction_id,
        #[cfg(test)]
        None,
    )
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CertificatePublicationFailure {
    StagingDirectoryCreate,
    ManifestWrite,
    CertificateWrite,
    CertificateHashWrite,
    PublishRename,
    PostPublishVerification,
}

#[cfg(test)]
impl CertificatePublicationFailure {
    fn label(self) -> &'static str {
        match self {
            Self::StagingDirectoryCreate => "staging-directory-create",
            Self::ManifestWrite => "manifest-write",
            Self::CertificateWrite => "certificate-write",
            Self::CertificateHashWrite => "certificate-hash-write",
            Self::PublishRename => "publish-rename",
            Self::PostPublishVerification => "post-publish-verification",
        }
    }

    fn stage_id(self) -> String {
        format!("failure-injection-{}", self.label())
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
    let publish_result = (|| -> Result<()> {
        let staged_manifest = stage.join("EVIDENCE_MANIFEST.json");
        let staged_certificate = stage.join("DOCUMENTATION_CERTIFICATE.md");
        let staged_hashes = stage.join("CERTIFICATE_SHA256.txt");
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
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::CertificateHashWrite,
        )?;
        atomic_write_new(&staged_hashes, certificate_hashes)?;
        verify_staged_set(track_root, &stage)?;

        let destination = contained_path(track_root, Path::new(CERTIFICATE_DIR), false)?;
        #[cfg(test)]
        inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::PublishRename,
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
            fs::remove_dir(&destination).map_err(|error| AppError::io(&destination, error))?;
        }
        fs::rename(&stage, &destination).map_err(|error| AppError::io(&destination, error))?;
        #[cfg(test)]
        let post_publish_verification = inject_certificate_publication_failure(
            failure,
            CertificatePublicationFailure::PostPublishVerification,
        )
        .and_then(|()| verify(track_root));
        #[cfg(not(test))]
        let post_publish_verification = verify(track_root);
        if let Err(verification_error) = post_publish_verification {
            let rollback = fs::rename(&destination, &stage);
            return match rollback {
                Ok(()) => Err(verification_error),
                Err(rollback_error) => Err(AppError::Data(format!(
                    "Certificate publication verification failed ({verification_error}); rollback failed: {rollback_error}"
                ))),
            };
        }
        Ok(())
    })();
    if stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    publish_result
}

fn verify_staged_set(track_root: &Path, stage: &Path) -> Result<()> {
    let hashes_path = stage.join("CERTIFICATE_SHA256.txt");
    let content =
        fs::read_to_string(&hashes_path).map_err(|error| AppError::io(&hashes_path, error))?;
    for (relative, expected) in parse_certificate_hashes(&content)? {
        let path = match relative.as_str() {
            HASH_FILE => contained_path(track_root, Path::new(HASH_FILE), true)?,
            MANIFEST_FILE => stage.join("EVIDENCE_MANIFEST.json"),
            CERTIFICATE_FILE => stage.join("DOCUMENTATION_CERTIFICATE.md"),
            _ => return Err(AppError::Data("Unexpected certificate hash entry.".into())),
        };
        if sha256_file(&path)? != expected {
            return Err(AppError::Validation(format!(
                "Staged certificate integrity mismatch: {relative}"
            )));
        }
    }
    Ok(())
}

fn parse_certificate_hashes(content: &str) -> Result<BTreeMap<String, String>> {
    let expected_paths = [HASH_FILE, MANIFEST_FILE, CERTIFICATE_FILE];
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
    if result.len() != expected_paths.len() {
        return Err(AppError::Validation(
            "Certificate hash set is incomplete.".into(),
        ));
    }
    Ok(result)
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
    if relative == Path::new(HASH_FILE) {
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

    fn publication_fixture(track_root: &Path) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let main_hashes = b"fixture main hash manifest\n";
        let main_hash_path = track_root.join(HASH_FILE);
        fs::create_dir_all(
            main_hash_path
                .parent()
                .expect("main hash manifest parent directory"),
        )
        .expect("create documentation fixture directory");
        fs::write(&main_hash_path, main_hashes).expect("write main hash manifest fixture");
        let manifest = b"{\"fixture\":true}\n".to_vec();
        let certificate = b"# Fixture certificate\n".to_vec();
        let certificate_hashes = format!(
            "{}  {}\n{}  {}\n{}  {}\n",
            sha256_bytes(main_hashes),
            HASH_FILE,
            sha256_bytes(&manifest),
            MANIFEST_FILE,
            sha256_bytes(&certificate),
            CERTIFICATE_FILE
        )
        .into_bytes();
        (manifest, certificate, certificate_hashes)
    }

    fn assert_injected_publication_failure(failure: CertificatePublicationFailure) {
        let workspace = tempfile::tempdir().expect("temporary directory");
        let track_root = workspace.path();
        let (manifest, certificate, certificate_hashes) = publication_fixture(track_root);
        let live = track_root.join(CERTIFICATE_DIR);
        let live_started_empty = failure != CertificatePublicationFailure::PostPublishVerification;
        if live_started_empty {
            fs::create_dir(&live).expect("create empty live certificate directory");
        }
        let correlated_stage = track_root
            .join(".archive")
            .join("certificate-staging")
            .join(failure.stage_id());

        let error = publish_certificate_set_impl(
            track_root,
            &manifest,
            &certificate,
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
        if live_started_empty {
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
        } else {
            assert!(
                !live.exists(),
                "absent live certificate directory was not restored after {} failure",
                failure.label()
            );
        }
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
    fn certificate_hash_write_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::CertificateHashWrite);
    }

    #[test]
    fn publish_rename_failure_is_controlled_and_cleaned() {
        assert_injected_publication_failure(CertificatePublicationFailure::PublishRename);
    }

    #[test]
    fn post_publish_verification_failure_rolls_back_and_cleans_staging() {
        assert_injected_publication_failure(CertificatePublicationFailure::PostPublishVerification);
    }

    #[test]
    fn certificate_hash_parser_requires_exact_complete_unique_set() {
        let valid = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
        );
        assert_eq!(
            parse_certificate_hashes(&valid).expect("valid set").len(),
            3
        );

        let duplicate = format!(
            "{DIGEST}  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {MANIFEST_FILE}\n"
        );
        assert!(parse_certificate_hashes(&duplicate).is_err());

        let invalid_digest = format!(
            "short  {HASH_FILE}\n{DIGEST}  {MANIFEST_FILE}\n{DIGEST}  {CERTIFICATE_FILE}\n"
        );
        assert!(parse_certificate_hashes(&invalid_digest).is_err());
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
