use super::*;

pub(super) fn external_timestamp_rows(
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
) -> Vec<TableRow> {
    let mut rows = vec![TableRow::mono(
        "Original Certificate ID [System value]",
        snapshot.certificate_id,
    )];

    if let Some(metadata) = snapshot.provider_metadata {
        push_automatic_timestamp_rows(&mut rows, snapshot, metadata);
    } else {
        push_legacy_timestamp_rows(&mut rows, snapshot);
    }

    rows.push(TableRow::mono(
        "Imported at [System value]",
        documented(snapshot.imported_at),
    ));
    rows.push(TableRow::plain(
        "Provenance [System value]",
        documented(snapshot.provenance),
    ));
    rows
}

fn push_automatic_timestamp_rows(
    rows: &mut Vec<TableRow>,
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
    metadata: &TimestampProviderMetadata,
) {
    let open_timestamps = is_open_timestamps_metadata(metadata);
    rows.push(TableRow::plain(
        "Record source [System verification]",
        "Automatic provider response",
    ));
    push_documented_row(
        rows,
        "Provider / issuer [Provider-derived metadata]",
        snapshot.provider,
        false,
    );
    push_documented_row(
        rows,
        "Timestamp type [Provider-derived metadata]",
        snapshot.timestamp_type,
        false,
    );
    if open_timestamps && snapshot.timestamp_value.trim().is_empty() {
        rows.push(TableRow::plain(
            "Confirmed timestamp [System verification]",
            "PENDING — OpenTimestamps proof verification / upgrade required",
        ));
    } else {
        push_documented_row(
            rows,
            "Timestamp value [Provider-derived metadata]",
            snapshot.timestamp_value,
            true,
        );
    }
    push_documented_row(
        rows,
        "Referenced artifact [System value]",
        snapshot.referenced_artifact,
        false,
    );
    rows.push(TableRow::mono(
        "Referenced artifact path [System value]",
        documented(snapshot.referenced_artifact_path),
    ));
    rows.push(TableRow::mono(
        "Requested SHA-256 [System verification]",
        snapshot.referenced_sha256,
    ));
    rows.push(TableRow::mono(
        "Actual artifact SHA-256 [System verification]",
        snapshot.actual_sha256,
    ));
    rows.push(TableRow::system_plain(
        "Referenced hash match [System verification]",
        yes_no(snapshot.referenced_hash_match),
    ));
    push_documented_row(
        rows,
        "Timestamp evidence filename [Evidence-derived metadata]",
        snapshot.evidence_file_name,
        false,
    );
    rows.push(TableRow::mono(
        "Timestamp evidence SHA-256 [System verification]",
        snapshot.evidence_sha256,
    ));
    push_documented_row(
        rows,
        "Provider external reference ID [Provider-derived metadata]",
        snapshot.external_reference_id,
        true,
    );
    let endpoint_label = if open_timestamps {
        "Calendar endpoint [Provider-derived metadata]"
    } else {
        "Provider verification URL [Provider-derived metadata]"
    };
    push_documented_row(
        rows,
        endpoint_label,
        snapshot.provider_verification_url,
        true,
    );
    push_documented_row(
        rows,
        "Provider response note [Provider-derived metadata]",
        snapshot.note,
        false,
    );
    provider_metadata_rows(rows, metadata);
}

fn push_legacy_timestamp_rows(
    rows: &mut Vec<TableRow>,
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
) {
    rows.push(TableRow::plain(
        "Record source [System value]",
        "Legacy manually recorded timestamp evidence",
    ));
    rows.push(TableRow::plain(
        "Provider verification result [System verification]",
        "ATTACHED – legacy manually recorded; no provider verification recorded",
    ));
    rows.push(TableRow::plain(
        "Provider / issuer [Legacy user-recorded fact]",
        documented(snapshot.provider),
    ));
    rows.push(TableRow::plain(
        "Timestamp type [Legacy user-recorded fact]",
        documented(snapshot.timestamp_type),
    ));
    rows.push(TableRow::mono(
        "Timestamp value [Legacy user-recorded fact]",
        documented(snapshot.timestamp_value),
    ));
    rows.push(TableRow::plain(
        "Referenced artifact [Legacy user-recorded fact]",
        documented(snapshot.referenced_artifact),
    ));
    rows.push(TableRow::mono(
        "Referenced artifact path [System value]",
        documented(snapshot.referenced_artifact_path),
    ));
    rows.push(TableRow::mono(
        "Referenced SHA-256 [Legacy user-recorded fact]",
        snapshot.referenced_sha256,
    ));
    rows.push(TableRow::mono(
        "Actual artifact SHA-256 [System verification]",
        snapshot.actual_sha256,
    ));
    rows.push(TableRow::system_plain(
        "Referenced hash match [System verification]",
        yes_no(snapshot.referenced_hash_match),
    ));
    rows.push(TableRow::plain(
        "Evidence filename [Evidence-derived metadata]",
        documented(snapshot.evidence_file_name),
    ));
    rows.push(TableRow::mono(
        "Timestamp evidence SHA-256 [System verification]",
        snapshot.evidence_sha256,
    ));
    rows.push(TableRow::mono(
        "External reference ID [Legacy user-recorded fact]",
        documented(snapshot.external_reference_id),
    ));
    rows.push(TableRow::mono(
        "Provider verification URL [Legacy user-recorded fact]",
        documented(snapshot.provider_verification_url),
    ));
    rows.push(TableRow::plain(
        "Note [Legacy user-recorded fact]",
        documented(snapshot.note),
    ));
}

pub(super) fn push_documented_row(rows: &mut Vec<TableRow>, label: &str, value: &str, mono: bool) {
    if value.trim().is_empty() {
        return;
    }
    rows.push(if mono {
        TableRow::mono(label, value)
    } else {
        TableRow::plain(label, value)
    });
}

pub(super) fn provider_metadata_rows(
    rows: &mut Vec<TableRow>,
    metadata: &TimestampProviderMetadata,
) {
    let open_timestamps = is_open_timestamps_metadata(metadata);
    push_provider_identity_rows(rows, metadata, open_timestamps);
    if metadata.protocol.contains("RFC 3161") {
        push_rfc3161_metadata_rows(rows, metadata);
    }
    push_provider_verification_rows(rows, metadata, open_timestamps);
}

fn push_provider_identity_rows(
    rows: &mut Vec<TableRow>,
    metadata: &TimestampProviderMetadata,
    open_timestamps: bool,
) {
    for (label, value, mono) in [
        (
            "Timestamp adapter [Provider-derived metadata]",
            metadata.adapter.as_str(),
            false,
        ),
        (
            "Timestamp protocol [Provider-derived metadata]",
            metadata.protocol.as_str(),
            false,
        ),
        (
            "Request algorithm [Provider-derived metadata]",
            metadata.request_algorithm.as_str(),
            false,
        ),
        (
            "Provider response format [Provider-derived metadata]",
            metadata.response_format.as_str(),
            false,
        ),
        (
            if open_timestamps {
                "Calendar endpoint identifier [Provider-derived metadata]"
            } else {
                "Provider endpoint identifier [Provider-derived metadata]"
            },
            metadata.provider_endpoint_identifier.as_str(),
            false,
        ),
        (
            "Provider response archive [System value]",
            metadata.provider_response_file_name.as_str(),
            false,
        ),
        (
            "Provider response SHA-256 [System verification]",
            metadata.provider_response_sha256.as_str(),
            true,
        ),
        (
            "Finalization snapshot / revision ID [System verification]",
            metadata.referenced_revision_id.as_str(),
            true,
        ),
        (
            "Timestamp issuer [Provider-derived metadata]",
            metadata.issuer.as_str(),
            false,
        ),
        (
            "Timestamp certificate subject [Provider-derived metadata]",
            metadata.certificate_subject.as_str(),
            false,
        ),
        (
            "Timestamp certificate serial number [Provider-derived metadata]",
            metadata.certificate_serial_number.as_str(),
            true,
        ),
        (
            "Verification timestamp [System verification]",
            metadata.verification_timestamp.as_str(),
            true,
        ),
    ] {
        push_documented_row(rows, label, value, mono);
    }
}

fn push_rfc3161_metadata_rows(rows: &mut Vec<TableRow>, metadata: &TimestampProviderMetadata) {
    let requested_policy = if metadata.requested_policy_oid.trim().is_empty() {
        "NONE (provider policy accepted)"
    } else {
        metadata.requested_policy_oid.as_str()
    };
    let trust_anchor_sha256 = metadata.trust_anchor_sha256.join(", ");
    for (label, value, mono) in [
        (
            "Timestamp policy OID [Provider-derived metadata]",
            metadata.policy_oid.as_str(),
            false,
        ),
        (
            "RFC 3161 request nonce [System value]",
            metadata.request_nonce.as_str(),
            true,
        ),
        (
            "RFC 3161 response nonce [Provider-derived metadata]",
            metadata.response_nonce.as_str(),
            true,
        ),
        (
            "Requested timestamp policy OID [System value]",
            requested_policy,
            false,
        ),
        (
            "Cryptographic verifier [System verification]",
            metadata.cryptographic_verifier.as_str(),
            false,
        ),
        (
            "Trust-anchor SHA-256 [System verification]",
            trust_anchor_sha256.as_str(),
            true,
        ),
    ] {
        push_documented_row(rows, label, value, mono);
    }
}

fn push_provider_verification_rows(
    rows: &mut Vec<TableRow>,
    metadata: &TimestampProviderMetadata,
    open_timestamps: bool,
) {
    rows.push(TableRow::system_plain(
        "Provider verification result [System verification]",
        provider_verification_status_label(metadata.verification_result),
    ));
    push_documented_row(
        rows,
        "Provider verification detail [System verification]",
        metadata.verification_message.as_str(),
        false,
    );
    if open_timestamps {
        rows.push(TableRow::system_plain(
            "OpenTimestamps proof verification [System verification]",
            "PENDING — upgrade/verification required",
        ));
        rows.push(TableRow::system_plain(
            "Local manifest / proof binding [System verification]",
            yes_no(metadata.provider_digest_match),
        ));
        rows.push(TableRow::system_plain(
            "CMS signature / trust chain [System verification]",
            "N/A — not RFC 3161",
        ));
    } else {
        for (label, value) in [
            (
                "Provider response structure valid [System verification]",
                metadata.response_structure_valid,
            ),
            (
                "Provider digest match [System verification]",
                metadata.provider_digest_match,
            ),
            (
                "RFC 3161 nonce match [System verification]",
                metadata.nonce_match,
            ),
            (
                "Requested policy match [System verification]",
                metadata.policy_match,
            ),
            (
                "Timestamp signature verified [System verification]",
                metadata.signature_verified,
            ),
            (
                "Timestamp trust chain verified [System verification]",
                metadata.trust_chain_verified,
            ),
        ] {
            if let Some(value) = value {
                rows.push(TableRow::system_plain(label, yes_no(Some(value))));
            }
        }
    }
}

pub(super) fn external_timestamp_qualification_rows(
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
) -> Vec<TableRow> {
    let mut rows = Vec::new();
    let Some(metadata) = snapshot.provider_metadata else {
        push_undocumented_qualification_rows(&mut rows);
        return rows;
    };
    let qualification = metadata.qualification.as_ref();
    push_qualification_status_rows(&mut rows, qualification);
    let Some(qualification) = qualification else {
        rows.push(TableRow::plain(
            "Qualification detail [Independent trust verification]",
            "No validated qualification source was checked. This does not mean that the provider is unsafe or not qualified.",
        ));
        return rows;
    };
    push_qualification_detail_rows(&mut rows, qualification);
    push_trusted_list_rows(&mut rows, qualification);
    rows
}

fn push_undocumented_qualification_rows(rows: &mut Vec<TableRow>) {
    for (label, status) in [
        (
            "Provider identity [Independent trust verification]",
            TimestampQualificationStatus::NotDocumented,
        ),
        (
            "Trust Service [Independent trust verification]",
            TimestampQualificationStatus::NotDocumented,
        ),
        (
            "eIDAS qualification [Independent trust verification]",
            TimestampQualificationStatus::NotDocumented,
        ),
        (
            "Qualification at timestamp [Independent trust verification]",
            TimestampQualificationStatus::NotDocumented,
        ),
    ] {
        rows.push(TableRow::system_plain(
            label,
            qualification_status_label(status),
        ));
    }
}

fn push_qualification_status_rows(
    rows: &mut Vec<TableRow>,
    qualification: Option<&crate::model::TimestampQualificationRecord>,
) {
    let status =
        |field: fn(&crate::model::TimestampQualificationRecord) -> TimestampQualificationStatus| {
            qualification.map(field).unwrap_or_default()
        };
    for (label, value) in [
        (
            "Provider identity [Independent trust verification]",
            status(|value| value.provider_identity_status),
        ),
        (
            "Trust Service [Independent trust verification]",
            status(|value| value.trust_service_status),
        ),
        (
            "eIDAS qualification [Independent trust verification]",
            status(|value| value.eidas_qualification_status),
        ),
        (
            "Qualification at timestamp [Independent trust verification]",
            status(|value| value.qualification_at_timestamp),
        ),
        (
            "Current qualification [Independent trust verification]",
            status(|value| value.current_qualification_status),
        ),
    ] {
        rows.push(TableRow::system_plain(
            label,
            qualification_status_label(value),
        ));
    }
}

fn push_qualification_detail_rows(
    rows: &mut Vec<TableRow>,
    qualification: &crate::model::TimestampQualificationRecord,
) {
    if qualification.eidas_qualification_status
        == TimestampQualificationStatus::QualifiedServiceVerified
    {
        rows.push(TableRow::system_plain(
            "Verified higher qualification [Independent trust verification]",
            "eIDAS QUALIFIED TRUST SERVICE – VERIFIED",
        ));
    }
    for (label, value, mono) in [
        (
            "Qualification checked at [System verification]",
            qualification.checked_at.as_str(),
            true,
        ),
        (
            "Recognized Trust Service Provider [Independent trust verification]",
            qualification.trust_service_provider.as_str(),
            false,
        ),
        (
            "Recognized Trust Service [Independent trust verification]",
            qualification.trust_service_name.as_str(),
            false,
        ),
        (
            "Trust Service type [Independent trust verification]",
            qualification.service_type.as_str(),
            false,
        ),
        (
            "Service status at timestamp [Independent trust verification]",
            qualification.service_status.as_str(),
            false,
        ),
        (
            "Current service status [Independent trust verification]",
            qualification.current_service_status.as_str(),
            false,
        ),
        (
            "Trust Service identifier [Independent trust verification]",
            qualification.service_identifier.as_str(),
            true,
        ),
        (
            "Qualification type [Independent trust verification]",
            qualification.qualification_type.as_str(),
            false,
        ),
        (
            "Timestamp-time status valid from [Independent trust verification]",
            qualification.status_valid_from.as_str(),
            true,
        ),
        (
            "Timestamp-time status valid until [Independent trust verification]",
            qualification.status_valid_until.as_str(),
            true,
        ),
        (
            "Current status valid from [Independent trust verification]",
            qualification.current_status_valid_from.as_str(),
            true,
        ),
        (
            "Current status valid until [Independent trust verification]",
            qualification.current_status_valid_until.as_str(),
            true,
        ),
        (
            "Signer certificate SHA-256 [System verification]",
            qualification.identity.certificate_sha256.as_str(),
            true,
        ),
        (
            "Qualification detail [Independent trust verification]",
            qualification.message.as_str(),
            false,
        ),
    ] {
        push_documented_row(rows, label, value, mono);
    }
}

fn push_trusted_list_rows(
    rows: &mut Vec<TableRow>,
    qualification: &crate::model::TimestampQualificationRecord,
) {
    if let Some(source) = qualification.trusted_list.as_ref() {
        for (label, value, mono) in [
            (
                "Trusted List source [Independent trust verification]",
                source.source.as_str(),
                false,
            ),
            (
                "Trusted List territory [Independent trust verification]",
                source.territory.as_str(),
                false,
            ),
            (
                "Trusted List version / sequence [Independent trust verification]",
                if source.sequence_number.is_empty() {
                    source.version.as_str()
                } else {
                    source.sequence_number.as_str()
                },
                false,
            ),
            (
                "Trusted List issued at [Independent trust verification]",
                source.issued_at.as_str(),
                true,
            ),
            (
                "Trusted List next update [Independent trust verification]",
                source.next_update.as_str(),
                true,
            ),
            (
                "Trusted List SHA-256 [System verification]",
                source.sha256.as_str(),
                true,
            ),
            (
                "Trusted List validated at [System verification]",
                source.validated_at.as_str(),
                true,
            ),
        ] {
            push_documented_row(rows, label, value, mono);
        }
        rows.push(TableRow::system_plain(
            "Trusted List validation [System verification]",
            trusted_list_validation_status_label(source.validation_status),
        ));
    }
}

pub(super) fn trusted_list_validation_status_label(
    status: TrustedListValidationStatus,
) -> &'static str {
    match status {
        TrustedListValidationStatus::NotChecked => "NOT CHECKED",
        TrustedListValidationStatus::Verified => "VERIFIED",
        TrustedListValidationStatus::Failed => "FAILED",
    }
}

pub(super) fn provider_configuration_status_label(
    status: TimestampProviderConfigurationStatus,
) -> &'static str {
    match status {
        TimestampProviderConfigurationStatus::Disabled => "DISABLED",
        TimestampProviderConfigurationStatus::NotConfigured => "NOT CONFIGURED",
        TimestampProviderConfigurationStatus::Ready => "READY",
        TimestampProviderConfigurationStatus::AuthenticationRequired => "AUTHENTICATION REQUIRED",
        TimestampProviderConfigurationStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        TimestampProviderConfigurationStatus::ConnectionFailed => "CONNECTION FAILED",
        TimestampProviderConfigurationStatus::VerificationConfigurationIncomplete => {
            "VERIFICATION CONFIGURATION INCOMPLETE"
        }
        TimestampProviderConfigurationStatus::ProviderError => "PROVIDER ERROR",
    }
}

pub(super) fn qualification_status_label(status: TimestampQualificationStatus) -> &'static str {
    match status {
        TimestampQualificationStatus::NotChecked => "NOT CHECKED",
        TimestampQualificationStatus::NotDocumented => "NOT DOCUMENTED",
        TimestampQualificationStatus::NotVerified => "NOT VERIFIED",
        TimestampQualificationStatus::ProviderIdentityVerified => "PROVIDER IDENTITY VERIFIED",
        TimestampQualificationStatus::TrustServiceVerified => "TRUST SERVICE VERIFIED",
        TimestampQualificationStatus::QualifiedServiceVerified => "QUALIFIED SERVICE VERIFIED",
        TimestampQualificationStatus::CheckFailed => "CHECK FAILED",
    }
}

pub(super) fn is_open_timestamps_metadata(metadata: &TimestampProviderMetadata) -> bool {
    metadata.adapter.eq_ignore_ascii_case("open_timestamps")
        || metadata
            .protocol
            .to_ascii_lowercase()
            .contains("opentimestamps")
}

pub(super) fn provider_verification_status_label(status: ExternalTimestampStatus) -> &'static str {
    match status {
        ExternalTimestampStatus::NotRecorded => "NOT RECORDED",
        ExternalTimestampStatus::Requesting => "REQUESTING",
        ExternalTimestampStatus::Attached => "ATTACHED",
        ExternalTimestampStatus::Verified => "VERIFIED",
        ExternalTimestampStatus::VerificationFailed => "VERIFICATION FAILED",
        ExternalTimestampStatus::ProviderUnavailable => "PROVIDER UNAVAILABLE",
        ExternalTimestampStatus::AuthenticationFailed => "AUTHENTICATION FAILED",
        ExternalTimestampStatus::AnchorMismatch => "ANCHOR MISMATCH",
        ExternalTimestampStatus::Disabled => "DISABLED",
        ExternalTimestampStatus::Ready => "READY",
        ExternalTimestampStatus::ConfigurationIncomplete => "CONFIGURATION INCOMPLETE",
        ExternalTimestampStatus::AuthenticationRequired => "AUTHENTICATION REQUIRED",
        ExternalTimestampStatus::ConnectionFailed => "CONNECTION FAILED",
        ExternalTimestampStatus::UnsupportedResponse => "UNSUPPORTED RESPONSE",
        ExternalTimestampStatus::VerificationConfigurationIncomplete => {
            "VERIFICATION CONFIGURATION INCOMPLETE"
        }
    }
}
