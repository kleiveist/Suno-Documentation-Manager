use super::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManifestEvidence<'a> {
    pub(super) id: &'a str,
    pub(super) role: &'a str,
    pub(super) file_name: &'a str,
    pub(super) relative_path: &'a str,
    pub(super) sha256: Option<&'a str>,
    pub(super) size_bytes: u64,
    pub(super) imported_at: &'a str,
    pub(super) source_global_evidence_id: Option<&'a str>,
    pub(super) coverage_start: Option<&'a str>,
    pub(super) coverage_end: Option<&'a str>,
    pub(super) provenance: &'a EvidenceProvenance,
    pub(super) derived_from_evidence_id: Option<&'a str>,
    pub(super) generator_version: Option<&'a str>,
    pub(super) generated_disclosure_text: Option<&'a str>,
    pub(super) metadata: serde_json::Value,
}

/// Sanitized portable screening snapshot for a new certificate manifest.
///
/// The full Chromaprint fingerprint is retained only in the dedicated local
/// screening artifact; it is deliberately not copied into a certificate
/// manifest. Raw provider response bytes, request signatures, and credentials
/// are likewise excluded. This keeps the manifest reviewable while preserving
/// the source/artifact binding needed for an integrity audit.
pub(super) fn audio_screening_manifest(state: &AudioScreeningState) -> serde_json::Value {
    let local = &state.local;
    let external = &state.external;
    let matches = &external.matches;
    let samples = external
        .samples
        .iter()
        .map(|sample| {
            json!({
                "sequence": sample.sequence,
                "offsetMilliseconds": sample.offset_milliseconds,
                "endOffsetMilliseconds": sample.end_offset_milliseconds,
                "durationMilliseconds": sample.duration_milliseconds,
                "status": sample.status,
                "providerStatusCode": sample.provider_status_code,
                "providerStatusMessage": sample.provider_status_message,
                "providerApiVersion": sample.provider_api_version,
                "responseRelativePath": sample.response_relative_path,
                "responseSha256": sample.response_sha256,
                "matches": &sample.matches,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "local": {
            "schemaVersion": local.schema_version,
            "status": local.status,
            "engine": local.engine,
            "engineVersion": local.engine_version,
            "fingerprintAlgorithm": local.fingerprint_algorithm,
            "sourceEvidenceId": local.source_evidence_id,
            "sourceRelativePath": local.source_relative_path,
            "sourceSha256": local.source_sha256,
            "sourceSizeBytes": local.source_size_bytes,
            "durationMilliseconds": local.duration_milliseconds,
            "generatedAt": local.generated_at,
            "artifactRelativePath": local.artifact_relative_path,
            "artifactSha256": local.artifact_sha256,
        },
        "external": {
            "schemaVersion": external.schema_version,
            "provider": external.provider,
            "status": external.status,
            "sourceEvidenceId": external.source_evidence_id,
            "sourceRelativePath": external.source_relative_path,
            "sourceSha256": external.source_sha256,
            "sourceSizeBytes": external.source_size_bytes,
            "checkedAt": external.checked_at,
            "sampleOffsetMilliseconds": external.sample_offset_milliseconds,
            "sampleDurationMilliseconds": external.sample_duration_milliseconds,
            "sourceDurationMilliseconds": external.source_duration_milliseconds,
            "requestCount": external.request_count,
            "screeningMode": external.screening_mode,
            "requestedIntensityPercent": external.requested_intensity_percent,
            "dynamicByTrackDuration": external.dynamic_by_track_duration,
            "referenceDurationSeconds": external.reference_duration_seconds,
            "targetDurationMilliseconds": external.target_duration_milliseconds,
            "plannedRequestCount": external.planned_request_count,
            "executedRequestCount": external.executed_request_count,
            "uniqueSampleCount": external.unique_sample_count,
            "overlappingSampleCount": external.overlapping_sample_count,
            "duplicateSampleCount": external.duplicate_sample_count,
            "uniqueSampleDurationMilliseconds": external.unique_sample_duration_milliseconds,
            "trackCoveragePercent": external.track_coverage_percent,
            "providerStatus": external.provider_status,
            "samples": samples,
            "responseRelativePath": external.response_relative_path,
            "responseSha256": external.response_sha256,
            "configuredAtSnapshot": external.configured_at_snapshot,
            "matches": matches,
        },
        "statementScope": "technical comparison record only; no authorship, ownership, permission, infringement, legality, release-clearance, or legal conclusion",
    })
}
