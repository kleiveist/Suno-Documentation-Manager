use super::*;

pub(super) fn finalization_timestamp_markdown(
    timestamp: &FinalizationTimestampSnapshot,
    manifest_sha256: &str,
    commercial_use_intended: bool,
) -> String {
    let provider = markdown_documented(&timestamp.provider);
    let mut output = format!(
        "### A. Technical timestamp\n\n- Provider configuration [System verification]: **{}**\n- Provider configuration detail [System verification]: {}\n- Provider [System value]: {}\n- Automatic request for this finalization [System value]: **{}**\n- Concrete timestamp status [System verification]: **{}**\n- Concrete timestamp detail [System verification]: {}\n- Referenced artifact [System value]: `EVIDENCE_MANIFEST.json`\n- Manifest anchor SHA-256 [System verification]: `{}`\n",
        timestamp_provider_configuration_label(timestamp.provider_configuration_status),
        markdown_documented(&timestamp.provider_configuration_message),
        provider,
        if timestamp.automatic_request_enabled { "ENABLED" } else { "DISABLED" },
        timestamp_technical_status_label(timestamp.technical_status),
        markdown_documented(&timestamp.technical_message),
        markdown_raw_value(manifest_sha256),
    );

    if let Some(metadata) = timestamp.provider_metadata.as_ref() {
        output.push_str(&format!(
            "- Timestamp protocol [Provider-derived metadata]: {}\n- Hash algorithm [Provider-derived metadata]: {}\n- Provider response structurally valid [System verification]: **{}**\n- Manifest hash binding [System verification]: **{}**\n- Timestamp signature applicable [System verification]: **{}**\n- Timestamp signature valid [System verification]: **{}**\n- Certificate chain applicable [System verification]: **{}**\n- Certificate chain technically verified [System verification]: **{}**\n- Provider identity technically recognized [System verification]: **{}**\n- Timestamp value [Provider-derived metadata]: {}\n- Provider reference ID [Provider-derived metadata]: {}\n- Signer certificate subject [System verification]: {}\n- Signer certificate issuer [System verification]: {}\n- Signer certificate serial [System verification]: {}\n- Signer certificate SHA-256 [System verification]: `{}`\n- Policy OID [Provider-derived metadata]: {}\n",
            markdown_documented(&metadata.protocol),
            markdown_documented(&metadata.request_algorithm),
            optional_verification_label(metadata.response_structure_valid),
            optional_verification_label(metadata.provider_digest_match),
            recorded_bool(metadata.signature_verification_applicable),
            optional_verification_label(metadata.signature_verified),
            recorded_bool(metadata.trust_chain_verification_applicable),
            optional_verification_label(metadata.trust_chain_verified),
            optional_verification_label(metadata.provider_identity_verified),
            markdown_documented(&timestamp.timestamp_value),
            markdown_documented(&timestamp.external_reference_id),
            markdown_documented(&metadata.certificate_subject),
            markdown_documented(&metadata.issuer),
            markdown_documented(&metadata.certificate_serial_number),
            markdown_raw_value(&metadata.certificate_sha256),
            markdown_documented(&metadata.policy_oid),
        ));
    } else {
        output.push_str(
            "- Timestamp protocol [System verification]: **NOT DOCUMENTED**\n- Manifest hash binding [System verification]: **NOT VERIFIED**\n",
        );
    }

    let qualification = timestamp
        .provider_metadata
        .as_ref()
        .and_then(|metadata| metadata.qualification.as_ref());
    let qualification_status =
        |field: fn(&crate::model::TimestampQualificationRecord) -> TimestampQualificationStatus| {
            qualification.map(field).unwrap_or_default()
        };
    output.push_str(&format!(
        "\n### B. Provider trust and qualification\n\n- Provider identity [Independent trust verification]: **{}**\n- Trust Service [Independent trust verification]: **{}**\n- eIDAS qualification [Independent trust verification]: **{}**\n- Qualification at timestamp [Independent trust verification]: **{}**\n- Current qualification [Independent trust verification]: **{}**\n",
        timestamp_qualification_status_label(qualification_status(|value| value.provider_identity_status)),
        timestamp_qualification_status_label(qualification_status(|value| value.trust_service_status)),
        timestamp_qualification_status_label(qualification_status(|value| value.eidas_qualification_status)),
        timestamp_qualification_status_label(qualification_status(|value| value.qualification_at_timestamp)),
        timestamp_qualification_status_label(qualification_status(|value| value.current_qualification_status)),
    ));

    if let Some(qualification) = qualification {
        if qualification.eidas_qualification_status
            == TimestampQualificationStatus::QualifiedServiceVerified
        {
            output.push_str("\n**eIDAS QUALIFIED TRUST SERVICE – VERIFIED**\n");
        }
        output.push_str(&format!(
            "- Qualification checked at [System verification]: {}\n- Qualification detail [Independent trust verification]: {}\n- Recognized Trust Service Provider [Independent trust verification]: {}\n- Recognized Trust Service [Independent trust verification]: {}\n- Trust Service type [Independent trust verification]: {}\n- Service status at timestamp [Independent trust verification]: {}\n- Current service status [Independent trust verification]: {}\n- Trust Service identifier [Independent trust verification]: {}\n- Qualification type [Independent trust verification]: {}\n- Timestamp-time status valid from [Independent trust verification]: {}\n- Timestamp-time status valid until [Independent trust verification]: {}\n- Current status valid from [Independent trust verification]: {}\n- Current status valid until [Independent trust verification]: {}\n- Identity certificate SHA-256 [System verification]: `{}`\n",
            markdown_documented(&qualification.checked_at),
            markdown_documented(&qualification.message),
            markdown_documented(&qualification.trust_service_provider),
            markdown_documented(&qualification.trust_service_name),
            markdown_documented(&qualification.service_type),
            markdown_documented(&qualification.service_status),
            markdown_documented(&qualification.current_service_status),
            markdown_documented(&qualification.service_identifier),
            markdown_documented(&qualification.qualification_type),
            markdown_documented(&qualification.status_valid_from),
            markdown_documented(&qualification.status_valid_until),
            markdown_documented(&qualification.current_status_valid_from),
            markdown_documented(&qualification.current_status_valid_until),
            markdown_raw_value(&qualification.identity.certificate_sha256),
        ));
        if let Some(source) = qualification.trusted_list.as_ref() {
            output.push_str(&format!(
                "- Trusted List source [Independent trust verification]: {}\n- Trusted List territory [Independent trust verification]: {}\n- Trusted List version [Independent trust verification]: {}\n- Trusted List sequence [Independent trust verification]: {}\n- Trusted List issued at [Independent trust verification]: {}\n- Trusted List next update [Independent trust verification]: {}\n- Trusted List SHA-256 [System verification]: `{}`\n- Trusted List validation [Independent trust verification]: **{}**\n- Trusted List validated at [System verification]: {}\n",
                markdown_documented(&source.source),
                markdown_documented(&source.territory),
                markdown_documented(&source.version),
                markdown_documented(&source.sequence_number),
                markdown_documented(&source.issued_at),
                markdown_documented(&source.next_update),
                markdown_raw_value(&source.sha256),
                timestamp_trusted_list_validation_label(source.validation_status),
                markdown_documented(&source.validated_at),
            ));
        }
    } else {
        output.push_str("- Qualification detail [System verification]: No validated qualification source was checked. This does not mean that the provider is unsafe or not qualified.\n");
    }

    output.push_str("\nTechnical timestamp verification and provider qualification answer different questions. No legal effect is inferred. A regulatory qualification is reported only when independently verified.\n");
    if commercial_use_intended && timestamp.technical_status == ExternalTimestampStatus::NotRecorded
    {
        output.push_str("\nFor long-term evidentiary preservation, an external timestamp can be requested in a later immutable addendum.\n");
    }
    output
}

pub(super) fn timestamp_provider_configuration_label(
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

pub(super) fn timestamp_technical_status_label(status: ExternalTimestampStatus) -> &'static str {
    match status {
        ExternalTimestampStatus::NotRecorded => "NOT RECORDED",
        ExternalTimestampStatus::Requesting => "REQUESTING",
        ExternalTimestampStatus::Attached => "ATTACHED",
        ExternalTimestampStatus::Verified => "VERIFIED",
        ExternalTimestampStatus::VerificationFailed => "FAILED",
        ExternalTimestampStatus::ProviderUnavailable => "FAILED",
        ExternalTimestampStatus::AuthenticationFailed => "FAILED",
        ExternalTimestampStatus::AnchorMismatch => "FAILED",
        ExternalTimestampStatus::Disabled => "NOT RECORDED",
        ExternalTimestampStatus::Ready => "NOT RECORDED",
        ExternalTimestampStatus::ConfigurationIncomplete => "NOT RECORDED",
        ExternalTimestampStatus::AuthenticationRequired => "NOT RECORDED",
        ExternalTimestampStatus::ConnectionFailed => "FAILED",
        ExternalTimestampStatus::UnsupportedResponse => "FAILED",
        ExternalTimestampStatus::VerificationConfigurationIncomplete => "FAILED",
    }
}

pub(super) fn timestamp_qualification_status_label(
    status: TimestampQualificationStatus,
) -> &'static str {
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

pub(super) fn timestamp_trusted_list_validation_label(
    status: TrustedListValidationStatus,
) -> &'static str {
    match status {
        TrustedListValidationStatus::NotChecked => "NOT CHECKED",
        TrustedListValidationStatus::Verified => "VERIFIED",
        TrustedListValidationStatus::Failed => "FAILED",
    }
}

pub(super) fn optional_verification_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "VERIFIED",
        Some(false) => "NOT VERIFIED",
        None => "NOT CHECKED",
    }
}

pub(super) fn archived_revision_references(track_root: &Path) -> Result<Vec<String>> {
    let relative_root = Path::new(".archive/revisions");
    let revisions_root = contained_path(track_root, relative_root, false)?;
    if !revisions_root.exists() {
        return Ok(Vec::new());
    }
    if !revisions_root.is_dir() {
        return Err(AppError::Data(
            "The managed revision archive path is not a directory.".into(),
        ));
    }

    let mut references = Vec::new();
    for entry in
        fs::read_dir(&revisions_root).map_err(|error| AppError::io(&revisions_root, error))?
    {
        let entry = entry.map_err(|error| AppError::io(&revisions_root, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::io(entry.path(), error))?;
        if file_type.is_symlink() {
            return Err(AppError::Symlink(entry.path().display().to_string()));
        }
        if !file_type.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| AppError::Data("A revision archive ID is not valid UTF-8.".into()))?;
        let relative = relative_root.join(&name);
        let metadata = contained_path(track_root, &relative.join("revision.json"), false)?;
        if metadata.is_file() {
            references.push(portable_relative(&relative));
        }
    }
    references.sort();
    Ok(references)
}
