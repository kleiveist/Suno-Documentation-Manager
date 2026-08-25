use super::*;

#[test]
fn tampered_finalized_manifest_prevents_provider_request() {
    let directory = tempdir().must("temporary track root");
    let track_root = directory.path();
    fs::create_dir_all(track_root.join(certificate::CERTIFICATE_DIR)).must("certificate directory");
    let manifest = track_root.join(certificate::MANIFEST_FILE);
    fs::write(&manifest, b"{\"finalized\":true}\n").must("manifest");
    let expected = sha256_file(&manifest).must("finalized hash");
    fs::write(
        track_root.join(certificate::CERTIFICATE_HASH_FILE),
        format!("{expected}  {}\n", certificate::MANIFEST_FILE),
    )
    .must("certificate hash set");
    fs::write(&manifest, b"{\"tampered\":true}\n").must("tampered manifest");
    let mock = MockTransport::successful(rfc3161_response_for_digest(&"00".repeat(32)));

    if let Ok(anchor) = finalized_manifest_anchor(track_root) {
        let _ = request_timestamp_with_transport(
            &free_tsa_settings(),
            None,
            &anchor.sha256,
            None,
            &mock,
        );
    }
    assert_eq!(mock.calls.get(), 0, "no digest request may leave the app");
    assert!(finalized_manifest_anchor(track_root)
        .must_err("tampered anchor is rejected")
        .to_string()
        .contains("INTEGRITY CHECK FAILED"));
}
