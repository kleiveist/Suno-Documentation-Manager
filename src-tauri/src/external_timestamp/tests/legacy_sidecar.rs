use super::*;

#[test]
fn legacy_v0_sidecars_remain_self_consistently_verifiable_without_rendering() {
    let directory = tempdir().must("temporary track root");
    let track_root = directory.path();
    fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR)).must("certificate directory");
    let anchor = track_root.join(certificate::MANIFEST_FILE);
    fs::write(&anchor, b"legacy anchor").must("manifest anchor");
    let source = track_root.join("legacy-timestamp.json");
    fs::write(&source, b"legacy timestamp evidence").must("timestamp source");
    let staged = stage(
        track_root,
        "CERT-LEGACY-V0",
        &source,
        ExternalTimestampInput {
            provider: "Legacy Provider".into(),
            timestamp_type: TimestampType::ElectronicTimestamp,
            timestamp_value: String::new(),
            referenced_artifact: TimestampReferencedArtifact::EvidenceManifest,
            other_referenced_artifact: String::new(),
            referenced_sha256: sha256_file(&anchor).must("anchor hash"),
            external_reference_id: String::new(),
            provider_verification_url: String::new(),
            note: String::new(),
        },
    )
    .must("stage timestamp");
    let current = publish(track_root, &staged).must("publish timestamp");
    let record_directory = track_root
        .join(&current.record_relative_path)
        .parent()
        .must("record directory")
        .to_path_buf();

    let mut legacy_value = serde_json::to_value(&current).must("legacy record value");
    let legacy_object = legacy_value.as_object_mut().must("record object");
    legacy_object.remove("sidecarFormatVersion");
    legacy_object.remove("markdownSha256");
    legacy_object.remove("pdfSha256");
    legacy_object.remove("integrityVerifiedAtPublication");
    let legacy_bytes = serde_json::to_vec_pretty(&legacy_value).must("legacy bytes");
    fs::write(record_directory.join(RECORD_FILE), legacy_bytes).must("legacy record");
    let hashes =
        artifact_hashes(&record_directory, "TIMESTAMP_EVIDENCE.json").must("legacy hashes");
    fs::write(
        record_directory.join(HASH_LIST_FILE),
        render_hash_list(0, &hashes).must("legacy hash list"),
    )
    .must("legacy hash list fixture");
    let registered: ExternalTimestampRecord = serde_json::from_slice(
        &fs::read(record_directory.join(RECORD_FILE)).must("legacy record bytes"),
    )
    .must("deserialize legacy record");

    verify_published_record(track_root, &registered)
        .must("legacy sidecar self-consistency verifies without renderer equality");
}
