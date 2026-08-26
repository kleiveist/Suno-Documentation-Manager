use super::*;

pub(super) fn render_pdf(record: &ExternalTimestampRecord) -> Result<Vec<u8>> {
    certificate_pdf::generate_external_timestamp_addendum_pdf(&ExternalTimestampPdfSnapshot {
        certificate_id: &record.certificate_id,
        provider: &record.provider,
        timestamp_type: timestamp_type_label(record.timestamp_type),
        timestamp_value: &record.timestamp_value,
        referenced_artifact: referenced_artifact_label(record.referenced_artifact),
        referenced_artifact_path: &record.referenced_artifact_path,
        referenced_sha256: &record.referenced_sha256,
        actual_sha256: &record.actual_sha256,
        referenced_hash_match: record.referenced_hash_match,
        evidence_file_name: &record.evidence_file_name,
        evidence_sha256: &record.evidence_sha256,
        imported_at: &record.imported_at,
        provenance: &record.provenance,
        external_reference_id: &record.external_reference_id,
        provider_verification_url: &record.provider_verification_url,
        note: &record.note,
        provider_metadata: record.provider_metadata.as_ref(),
    })
}

pub(super) fn render_markdown(record: &ExternalTimestampRecord) -> String {
    let automatic = record.provider_metadata.is_some();
    let provider_origin = if automatic {
        "Provider-derived metadata"
    } else {
        "Legacy user-recorded fact"
    };
    let open_timestamps = record
        .provider_metadata
        .as_ref()
        .is_some_and(is_open_timestamps_metadata);
    let timestamp_value = if open_timestamps && record.timestamp_value.trim().is_empty() {
        "PENDING — OpenTimestamps proof verification / upgrade required".into()
    } else {
        documented_md(&record.timestamp_value)
    };
    let provider_url_label = if open_timestamps {
        "Calendar endpoint"
    } else {
        "Provider verification URL"
    };
    let provider_metadata_md = provider_metadata_markdown(record);
    let qualification_md = qualification_markdown(record);
    format!(
        "# SunoDM External Timestamp Evidence Addendum\n\n> Post-finalization technical timestamp and provider-qualification evidence.\n\n## Certificate association\n\n- Certificate ID: `{}`\n- Timestamp record ID: `{}`\n- Imported at [System value]: {}\n\n## External Timestamp Evidence\n\n- Provider / issuer [{provider_origin}]: {}\n- Timestamp type [{provider_origin}]: {}\n- Timestamp value [{provider_origin}]: {}\n- Referenced artifact [System value]: {}\n- Referenced artifact path [System value]: `{}`\n- Referenced SHA-256 [System verification]: `{}`\n- Actual artifact SHA-256 [System verification]: `{}`\n- Referenced hash match [System verification]: **{}**\n- Timestamp evidence filename [Evidence-derived metadata]: {}\n- Timestamp evidence SHA-256 [System verification]: `{}`\n- External reference ID [{provider_origin}]: {}\n- {provider_url_label} [{provider_origin}]: {}\n- Note [{provider_origin}]: {}\n- Provenance [System value]: {}\n{provider_metadata_md}\n{qualification_md}\n{}\n",
        md(&record.certificate_id),
        md(&record.id),
        md(&record.imported_at),
        documented_md(&record.provider),
        timestamp_type_label(record.timestamp_type),
        timestamp_value,
        referenced_artifact_label(record.referenced_artifact),
        md(&record.referenced_artifact_path),
        record.referenced_sha256,
        record.actual_sha256,
        match record.referenced_hash_match {
            Some(true) => "YES",
            Some(false) => "NO",
            None => "NOT VERIFIED",
        },
        documented_md(&record.evidence_file_name),
        record.evidence_sha256,
        documented_md(&record.external_reference_id),
        documented_md(&record.provider_verification_url),
        documented_md(&record.note),
        documented_md(&record.provenance),
        DISCLAIMER,
    )
}

pub(super) fn provider_metadata_markdown(record: &ExternalTimestampRecord) -> String {
    let Some(metadata) = &record.provider_metadata else {
        return "\n- Record source [System value]: Legacy manually recorded timestamp evidence\n- Provider response verification [System verification]: NOT RECORDED (legacy manually recorded timestamp evidence)\n".into();
    };
    let is_rfc3161 = metadata.protocol.contains("RFC 3161");
    let open_timestamps = is_open_timestamps_metadata(metadata);
    let protocol_verification_context = if is_rfc3161 {
        let requested_policy = if metadata.requested_policy_oid.trim().is_empty() {
            "NONE (provider policy accepted)"
        } else {
            metadata.requested_policy_oid.as_str()
        };
        let policy_match = if metadata.requested_policy_oid.trim().is_empty() {
            "N/A"
        } else {
            optional_bool_label(metadata.policy_match)
        };
        format!(
            "- Request nonce [System value]: `{}`\n- Response nonce [Provider-derived metadata]: `{}`\n- Nonce match [System verification]: {}\n- Requested policy OID [System value]: {}\n- Returned policy OID [Provider-derived metadata]: {}\n- Policy match [System verification]: {}\n- Cryptographic verifier [System value]: {}\n- Trust-anchor SHA-256 [System verification]: {}\n- Provider response structure valid [System verification]: {}\n- Provider digest match [System verification]: {}\n- CMS signature verified [System verification]: {}\n- Trust chain verified [System verification]: {}\n",
            documented_md(&metadata.request_nonce),
            documented_md(&metadata.response_nonce),
            optional_bool_label(metadata.nonce_match),
            documented_md(requested_policy),
            documented_md(&metadata.policy_oid),
            policy_match,
            documented_md(&metadata.cryptographic_verifier),
            documented_md(&metadata.trust_anchor_sha256.join(", ")),
            optional_bool_label(metadata.response_structure_valid),
            optional_bool_label(metadata.provider_digest_match),
            optional_bool_label(metadata.signature_verified),
            optional_bool_label(metadata.trust_chain_verified),
        )
    } else if open_timestamps {
        format!(
            "- OpenTimestamps proof verification [System verification]: PENDING — upgrade/verification required\n- Local manifest / proof binding [System verification]: {}\n- CMS signature / trust chain [System verification]: N/A — not RFC 3161\n",
            optional_bool_label(metadata.provider_digest_match),
        )
    } else {
        format!(
            "- Provider response structure valid [System verification]: {}\n- Provider digest match [System verification]: {}\n- CMS signature verified [System verification]: {}\n- Trust chain verified [System verification]: {}\n",
            optional_bool_label(metadata.response_structure_valid),
            optional_bool_label(metadata.provider_digest_match),
            optional_bool_label(metadata.signature_verified),
            optional_bool_label(metadata.trust_chain_verified),
        )
    };
    let endpoint_label = if open_timestamps {
        "Calendar endpoint identifier"
    } else {
        "Provider endpoint identifier"
    };
    format!(
        "\n### Provider response metadata\n\n- Record source [System value]: Automatically attached provider response\n- Referenced finalization snapshot ID [System value]: `{}`\n- Provider adapter [Provider-derived metadata]: {}\n- Protocol [Provider-derived metadata]: {}\n- Request algorithm [System value]: {}\n{protocol_verification_context}- Response format [Provider-derived metadata]: {}\n- {endpoint_label} [Provider-derived metadata]: {}\n- Archived raw provider response [System value]: {}\n- Archived raw provider response SHA-256 [System verification]: `{}`\n- Provider verification result [System verification]: {}\n- Provider verification message [System verification]: {}\n- Provider verification timestamp [System verification]: {}\n- Timestamp issuer [Provider-derived metadata]: {}\n- Timestamp certificate subject [Provider-derived metadata]: {}\n- Timestamp certificate serial number [Provider-derived metadata]: {}\n",
        documented_md(&metadata.referenced_revision_id),
        documented_md(&metadata.adapter),
        documented_md(&metadata.protocol),
        documented_md(&metadata.request_algorithm),
        documented_md(&metadata.response_format),
        documented_md(&metadata.provider_endpoint_identifier),
        documented_md(&metadata.provider_response_file_name),
        documented_md(&metadata.provider_response_sha256),
        timestamp_status_label(metadata.verification_result),
        documented_md(&metadata.verification_message),
        documented_md(&metadata.verification_timestamp),
        documented_md(&metadata.issuer),
        documented_md(&metadata.certificate_subject),
        documented_md(&metadata.certificate_serial_number),
    )
}

pub(super) fn qualification_markdown(record: &ExternalTimestampRecord) -> String {
    let Some(metadata) = record.provider_metadata.as_ref() else {
        return "\n### Provider Trust and Qualification\n\n- Provider identity [Independent trust verification]: NOT DOCUMENTED\n- Trust Service [Independent trust verification]: NOT DOCUMENTED\n- eIDAS qualification [Independent trust verification]: NOT DOCUMENTED\n- Qualification at timestamp [Independent trust verification]: NOT DOCUMENTED\n".into();
    };
    let Some(qualification) = metadata.qualification.as_ref() else {
        return "\n### Provider Trust and Qualification\n\n- Provider identity [Independent trust verification]: NOT CHECKED\n- Trust Service [Independent trust verification]: NOT CHECKED\n- eIDAS qualification [Independent trust verification]: NOT CHECKED\n- Qualification at timestamp [Independent trust verification]: NOT CHECKED\n\nNo validated qualification source was checked. This does not mean that the provider is unsafe or not qualified.\n".into();
    };
    let trusted_list = qualification
        .trusted_list
        .as_ref()
        .map(|source| {
            format!(
                "- Trusted List source [Independent trust verification]: {}\n- Trusted List territory [Independent trust verification]: {}\n- Trusted List version / sequence [Independent trust verification]: {} / {}\n- Trusted List issued at [Independent trust verification]: {}\n- Trusted List next update [Independent trust verification]: {}\n- Trusted List SHA-256 [System verification]: {}\n- Trusted List validation [System verification]: {:?}\n- Trusted List validated at [System verification]: {}\n",
                documented_md(&source.source),
                documented_md(&source.territory),
                documented_md(&source.version),
                documented_md(&source.sequence_number),
                documented_md(&source.issued_at),
                documented_md(&source.next_update),
                documented_md(&source.sha256),
                source.validation_status,
                documented_md(&source.validated_at),
            )
        })
        .unwrap_or_default();
    let verified_badge = if qualification.eidas_qualification_status
        == TimestampQualificationStatus::QualifiedServiceVerified
    {
        "\n**eIDAS QUALIFIED TRUST SERVICE – VERIFIED**\n"
    } else {
        Default::default()
    };
    format!(
        "\n### Provider Trust and Qualification\n\n- Provider identity [Independent trust verification]: {}\n- Trust Service [Independent trust verification]: {}\n- eIDAS qualification [Independent trust verification]: {}\n- Qualification at timestamp [Independent trust verification]: {}\n- Current qualification [Independent trust verification]: {}\n- Qualification checked at [System verification]: {}\n- Recognized Trust Service Provider [Independent trust verification]: {}\n- Recognized Trust Service [Independent trust verification]: {}\n- Trust Service type [Independent trust verification]: {}\n- Service status at timestamp [Independent trust verification]: {}\n- Current service status [Independent trust verification]: {}\n- Trust Service identifier [Independent trust verification]: {}\n- Qualification type [Independent trust verification]: {}\n- Timestamp-time status valid from [Independent trust verification]: {}\n- Timestamp-time status valid until [Independent trust verification]: {}\n- Current status valid from [Independent trust verification]: {}\n- Current status valid until [Independent trust verification]: {}\n- Signer certificate SHA-256 [System verification]: {}\n{trusted_list}\n- Qualification detail [Independent trust verification]: {}\n{verified_badge}",
        qualification_status_label(qualification.provider_identity_status),
        qualification_status_label(qualification.trust_service_status),
        qualification_status_label(qualification.eidas_qualification_status),
        qualification_status_label(qualification.qualification_at_timestamp),
        qualification_status_label(qualification.current_qualification_status),
        documented_md(&qualification.checked_at),
        documented_md(&qualification.trust_service_provider),
        documented_md(&qualification.trust_service_name),
        documented_md(&qualification.service_type),
        documented_md(&qualification.service_status),
        documented_md(&qualification.current_service_status),
        documented_md(&qualification.service_identifier),
        documented_md(&qualification.qualification_type),
        documented_md(&qualification.status_valid_from),
        documented_md(&qualification.status_valid_until),
        documented_md(&qualification.current_status_valid_from),
        documented_md(&qualification.current_status_valid_until),
        documented_md(&qualification.identity.certificate_sha256),
        documented_md(&qualification.message),
    )
}

pub(super) fn is_open_timestamps_metadata(metadata: &TimestampProviderMetadata) -> bool {
    metadata.adapter.eq_ignore_ascii_case("open_timestamps")
        || metadata
            .protocol
            .to_ascii_lowercase()
            .contains("opentimestamps")
}

pub(super) fn optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT VERIFIED",
    }
}

pub(super) fn md(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('\r', "")
        .replace('\n', "<br>")
}

pub(super) fn documented_md(value: &str) -> String {
    if value.trim().is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        md(value)
    }
}
