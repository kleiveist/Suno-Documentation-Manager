use super::*;

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn relationship_evidence(id: &str, role: EvidenceRole) -> EvidenceItem {
    EvidenceItem {
        id: id.into(),
        role,
        file_name: format!("{id}.dat"),
        relative_path: format!("03_DOCUMENTATION/{id}.dat"),
        sha256: Some(DIGEST.into()),
        size_bytes: 1,
        imported_at: "2026-08-17T10:00:00Z".into(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: EvidenceProvenance::ManagedCopy,
        derived_from_evidence_id: None,
        generator_version: None,
        generated_disclosure_text: None,
        metadata: EvidenceMetadata::default(),
    }
}

fn parse_main_hash_fixture(content: &str) -> Result<BTreeMap<String, String>> {
    let workspace = tempfile::tempdir().expect("temporary directory");
    let sums = workspace.path().join("SHA256SUMS.txt");
    fs::write(&sums, content).expect("write SHA256SUMS fixture");
    parse_hashes(&sums)
}

mod generation;
mod localization;
mod markdown;
mod relationships;
mod verification;
