use super::*;

/// Immutable input for a post-finalization external-timestamp addendum.
///
/// This deliberately borrows only display strings plus the factual three-state hash comparison. It
/// has no reference to a mutable track or to phase-one evidence, so rendering an addendum cannot
/// silently re-evaluate or mutate the finalized certificate snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalTimestampPdfSnapshot<'a> {
    /// The human-facing ID of the original, immutable technical certificate.
    pub certificate_id: &'a str,
    pub provider: &'a str,
    pub timestamp_type: &'a str,
    pub timestamp_value: &'a str,
    pub referenced_artifact: &'a str,
    pub referenced_artifact_path: &'a str,
    pub referenced_sha256: &'a str,
    pub actual_sha256: &'a str,
    pub referenced_hash_match: Option<bool>,
    pub evidence_file_name: &'a str,
    pub evidence_sha256: &'a str,
    pub imported_at: &'a str,
    pub provenance: &'a str,
    pub external_reference_id: &'a str,
    pub provider_verification_url: &'a str,
    pub note: &'a str,
    /// Present only for an automatically obtained provider response. A missing
    /// value deliberately retains the legacy, manually recorded addendum path.
    pub provider_metadata: Option<&'a TimestampProviderMetadata>,
}

/// Render a deterministic, standalone PDF for evidence attached after technical finalization.
///
/// The original manifest, Markdown certificate, and certificate PDF remain byte-identical. The
/// caller is responsible for publishing and hashing this returned addendum as a separate phase-two
/// artifact.
pub fn generate_external_timestamp_addendum_pdf(
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
) -> Result<Vec<u8>> {
    validate_external_timestamp_snapshot(snapshot)?;

    let mut layout = PdfLayout::new(CertificateRenderOptions::default());
    layout.write_wrapped(
        "SunoDM – External Timestamp Evidence Addendum",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        14.0,
        BuiltinFont::HelveticaBold,
        1.28,
    );
    layout.write_wrapped(
        "Post-finalization technical timestamp and provider-qualification evidence",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.5,
        BuiltinFont::Helvetica,
        1.2,
    );
    layout.y_mm -= 1.0;
    layout.rule(LEFT_MM, RIGHT_MM, layout.y_mm, 0.6);
    layout.y_mm -= 3.0;

    layout.section_title("External Timestamp Evidence");
    let rows = external_timestamp_rows(snapshot);
    // Keep the complete provenance/verification labels searchable with the
    // bundled DejaVu metrics; long provider values may wrap in the value column.
    layout.table_rows(&rows, 112.0);
    layout.section_title("Provider Trust and Qualification");
    let qualification_rows = external_timestamp_qualification_rows(snapshot);
    layout.table_rows(&qualification_rows, 112.0);
    layout.write_wrapped(
        EXTERNAL_TIMESTAMP_DISCLAIMER,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        8.0,
        BuiltinFont::Helvetica,
        1.35,
    );
    layout.write_wrapped(
        if snapshot.provider_metadata.is_some() {
            "Technical timestamp verification and provider qualification answer different questions. A higher qualification is shown only when the archived independent trust record verifies it for the relevant timestamp time; no legal-effect or rights determination is made."
        } else {
            "Legacy manually recorded timestamp evidence: provider and timestamp values were supplied by the user and were not promoted to provider verification. No qualified timestamp, legal effect, or rights determination is asserted."
        },
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        8.0,
        BuiltinFont::Helvetica,
        1.35,
    );

    let deterministic_id = format!("{}:{}", snapshot.certificate_id, snapshot.evidence_sha256);
    let (pages, _) = layout.into_pages(snapshot.certificate_id);
    let mut document = PdfDocument::new("SunoDM – External Timestamp Evidence Addendum");
    document.metadata.info.creator = GENERATED_BY.to_owned();
    document.metadata.info.producer = GENERATED_BY.to_owned();
    document.metadata.info.subject =
        "Post-finalization external timestamp evidence addendum".to_owned();
    document.metadata.info.identifier = snapshot.certificate_id.to_owned();
    document.metadata.info.keywords = vec![
        "SunoDM".to_owned(),
        "external timestamp".to_owned(),
        "evidence".to_owned(),
        "SHA-256".to_owned(),
    ];
    document.with_pages(pages);
    serialize_pdfa_2b(document, &deterministic_id, snapshot.imported_at, "en-US")
}

pub(super) fn validate_external_timestamp_snapshot(
    snapshot: &ExternalTimestampPdfSnapshot<'_>,
) -> Result<()> {
    for (label, value) in [
        ("certificate ID", snapshot.certificate_id),
        ("referenced artifact", snapshot.referenced_artifact),
        (
            "referenced artifact path",
            snapshot.referenced_artifact_path,
        ),
        ("evidence filename", snapshot.evidence_file_name),
        ("import timestamp", snapshot.imported_at),
        ("provenance", snapshot.provenance),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::Data(format!(
                "External timestamp PDF snapshot has no {label}."
            )));
        }
    }
    for (label, digest) in [
        ("referenced artifact", snapshot.referenced_sha256),
        ("actual referenced artifact", snapshot.actual_sha256),
        ("timestamp evidence", snapshot.evidence_sha256),
    ] {
        validate_sha256(label, digest)?;
    }
    for (label, value) in [
        ("certificate ID", snapshot.certificate_id),
        ("provider", snapshot.provider),
        ("timestamp type", snapshot.timestamp_type),
        ("timestamp value", snapshot.timestamp_value),
        ("referenced artifact", snapshot.referenced_artifact),
        (
            "referenced artifact path",
            snapshot.referenced_artifact_path,
        ),
        ("evidence filename", snapshot.evidence_file_name),
        ("external reference ID", snapshot.external_reference_id),
        (
            "provider verification URL",
            snapshot.provider_verification_url,
        ),
        ("note", snapshot.note),
        ("import timestamp", snapshot.imported_at),
        ("provenance", snapshot.provenance),
    ] {
        validate_win_ansi(label, value)?;
    }
    if let Some(metadata) = snapshot.provider_metadata {
        if !metadata.provider_response_sha256.trim().is_empty() {
            validate_sha256(
                "timestamp provider response",
                metadata.provider_response_sha256.as_str(),
            )?;
        }
        for (label, value) in [
            ("timestamp provider adapter", metadata.adapter.as_str()),
            ("timestamp provider protocol", metadata.protocol.as_str()),
            (
                "timestamp provider request algorithm",
                metadata.request_algorithm.as_str(),
            ),
            (
                "timestamp provider response format",
                metadata.response_format.as_str(),
            ),
            (
                "timestamp provider endpoint identifier",
                metadata.provider_endpoint_identifier.as_str(),
            ),
            (
                "timestamp provider response archive",
                metadata.provider_response_file_name.as_str(),
            ),
            (
                "timestamp referenced revision ID",
                metadata.referenced_revision_id.as_str(),
            ),
            ("timestamp provider issuer", metadata.issuer.as_str()),
            (
                "timestamp provider certificate subject",
                metadata.certificate_subject.as_str(),
            ),
            (
                "timestamp provider certificate serial number",
                metadata.certificate_serial_number.as_str(),
            ),
            (
                "timestamp provider policy OID",
                metadata.policy_oid.as_str(),
            ),
            (
                "timestamp verification timestamp",
                metadata.verification_timestamp.as_str(),
            ),
            (
                "timestamp verification message",
                metadata.verification_message.as_str(),
            ),
        ] {
            validate_win_ansi(label, value)?;
        }
        validate_timestamp_qualification_metadata(metadata)?;
    }
    Ok(())
}

pub(super) fn validate_timestamp_qualification_metadata(
    metadata: &TimestampProviderMetadata,
) -> Result<()> {
    validate_timestamp_metadata_digests(metadata)?;
    let Some(qualification) = metadata.qualification.as_ref() else {
        return Ok(());
    };
    validate_qualification_identity(metadata, qualification)?;
    validate_qualification_text(qualification)?;
    validate_qualification_trusted_list(qualification)?;
    validate_qualification_states(qualification)?;
    validate_qualification_status_consistency(qualification)?;

    let at_timestamp_qualified = qualification.status
        == TimestampQualificationStatus::QualifiedServiceVerified
        || qualification.eidas_qualification_status
            == TimestampQualificationStatus::QualifiedServiceVerified
        || qualification.qualification_at_timestamp
            == TimestampQualificationStatus::QualifiedServiceVerified;
    let currently_qualified = qualification.current_qualification_status
        == TimestampQualificationStatus::QualifiedServiceVerified;
    let trust_service_verified =
        qualification.trust_service_status == TimestampQualificationStatus::TrustServiceVerified;
    validate_verified_qualification_evidence(
        metadata,
        qualification,
        at_timestamp_qualified,
        currently_qualified,
        trust_service_verified,
    )?;
    validate_qualification_service_status(
        qualification,
        at_timestamp_qualified,
        currently_qualified,
    )
}

fn validate_timestamp_metadata_digests(metadata: &TimestampProviderMetadata) -> Result<()> {
    if !metadata.certificate_sha256.trim().is_empty() {
        validate_sha256("timestamp signer certificate", &metadata.certificate_sha256)?;
    }
    if !metadata.provider_response_sha256.trim().is_empty() {
        validate_sha256(
            "timestamp provider response",
            &metadata.provider_response_sha256,
        )?;
    }
    for digest in &metadata.trust_anchor_sha256 {
        validate_sha256("timestamp trust anchor", digest)?;
    }
    Ok(())
}

fn validate_qualification_identity(
    metadata: &TimestampProviderMetadata,
    qualification: &crate::model::TimestampQualificationRecord,
) -> Result<()> {
    let identity = &qualification.identity;
    if !identity.certificate_sha256.trim().is_empty() {
        validate_sha256(
            "qualified timestamp signer certificate",
            &identity.certificate_sha256,
        )?;
    }
    if !metadata.certificate_sha256.trim().is_empty()
        && !identity.certificate_sha256.trim().is_empty()
        && !metadata
            .certificate_sha256
            .eq_ignore_ascii_case(&identity.certificate_sha256)
    {
        return Err(AppError::Data(
            "Certificate PDF timestamp qualification identity does not match the cryptographically verified signer certificate."
                .into(),
        ));
    }
    Ok(())
}

fn validate_qualification_text(
    qualification: &crate::model::TimestampQualificationRecord,
) -> Result<()> {
    let identity = &qualification.identity;
    for (label, value) in [
        (
            "qualification check timestamp",
            qualification.checked_at.as_str(),
        ),
        ("qualification detail", qualification.message.as_str()),
        (
            "qualification certificate subject",
            identity.certificate_subject.as_str(),
        ),
        (
            "qualification certificate issuer",
            identity.certificate_issuer.as_str(),
        ),
        (
            "qualification certificate serial number",
            identity.certificate_serial_number.as_str(),
        ),
        ("qualification policy OID", identity.policy_oid.as_str()),
        (
            "qualification identity service identifier",
            identity.service_identifier.as_str(),
        ),
        (
            "Trust Service Provider",
            qualification.trust_service_provider.as_str(),
        ),
        ("Trust Service", qualification.trust_service_name.as_str()),
        ("Trust Service type", qualification.service_type.as_str()),
        (
            "Trust Service status at timestamp",
            qualification.service_status.as_str(),
        ),
        (
            "current Trust Service status",
            qualification.current_service_status.as_str(),
        ),
        (
            "Trust Service identifier",
            qualification.service_identifier.as_str(),
        ),
        (
            "qualification type",
            qualification.qualification_type.as_str(),
        ),
        (
            "timestamp-time qualification status valid from",
            qualification.status_valid_from.as_str(),
        ),
        (
            "timestamp-time qualification status valid until",
            qualification.status_valid_until.as_str(),
        ),
        (
            "current qualification status valid from",
            qualification.current_status_valid_from.as_str(),
        ),
        (
            "current qualification status valid until",
            qualification.current_status_valid_until.as_str(),
        ),
    ] {
        validate_win_ansi(label, value)?;
    }
    Ok(())
}

fn validate_qualification_trusted_list(
    qualification: &crate::model::TimestampQualificationRecord,
) -> Result<()> {
    let Some(trusted_list) = qualification.trusted_list.as_ref() else {
        return Ok(());
    };
    for (label, value) in [
        ("Trusted List source", trusted_list.source.as_str()),
        ("Trusted List territory", trusted_list.territory.as_str()),
        ("Trusted List version", trusted_list.version.as_str()),
        (
            "Trusted List sequence number",
            trusted_list.sequence_number.as_str(),
        ),
        (
            "Trusted List issue timestamp",
            trusted_list.issued_at.as_str(),
        ),
        (
            "Trusted List next update",
            trusted_list.next_update.as_str(),
        ),
        (
            "Trusted List validation timestamp",
            trusted_list.validated_at.as_str(),
        ),
    ] {
        validate_win_ansi(label, value)?;
    }
    if !trusted_list.sha256.trim().is_empty() {
        validate_sha256("Trusted List", &trusted_list.sha256)?;
    }
    if trusted_list.validation_status == TrustedListValidationStatus::Verified
        && (trusted_list.source.trim().is_empty()
            || trusted_list.sha256.trim().is_empty()
            || trusted_list.validated_at.trim().is_empty())
    {
        return Err(AppError::Data(
            "Certificate PDF cannot report a verified Trusted List without its source, SHA-256, and validation timestamp."
                .into(),
        ));
    }
    Ok(())
}

fn validate_qualification_states(
    qualification: &crate::model::TimestampQualificationRecord,
) -> Result<()> {
    if !matches!(
        qualification.provider_identity_status,
        TimestampQualificationStatus::NotChecked
            | TimestampQualificationStatus::NotDocumented
            | TimestampQualificationStatus::NotVerified
            | TimestampQualificationStatus::ProviderIdentityVerified
            | TimestampQualificationStatus::CheckFailed
    ) {
        return Err(AppError::Data(
            "Certificate PDF contains an invalid provider-identity qualification state.".into(),
        ));
    }
    if !matches!(
        qualification.trust_service_status,
        TimestampQualificationStatus::NotChecked
            | TimestampQualificationStatus::NotDocumented
            | TimestampQualificationStatus::NotVerified
            | TimestampQualificationStatus::TrustServiceVerified
            | TimestampQualificationStatus::CheckFailed
    ) {
        return Err(AppError::Data(
            "Certificate PDF contains an invalid Trust Service qualification state.".into(),
        ));
    }
    for status in [
        qualification.eidas_qualification_status,
        qualification.current_qualification_status,
        qualification.qualification_at_timestamp,
    ] {
        if !matches!(
            status,
            TimestampQualificationStatus::NotChecked
                | TimestampQualificationStatus::NotDocumented
                | TimestampQualificationStatus::NotVerified
                | TimestampQualificationStatus::QualifiedServiceVerified
                | TimestampQualificationStatus::CheckFailed
        ) {
            return Err(AppError::Data(
                "Certificate PDF contains an invalid regulatory qualification state.".into(),
            ));
        }
    }
    Ok(())
}

fn validate_qualification_status_consistency(
    qualification: &crate::model::TimestampQualificationRecord,
) -> Result<()> {
    if qualification.status == TimestampQualificationStatus::ProviderIdentityVerified
        && qualification.provider_identity_status
            != TimestampQualificationStatus::ProviderIdentityVerified
    {
        return Err(AppError::Data(
            "Certificate PDF contains an inconsistent verified provider identity state.".into(),
        ));
    }
    if qualification.status == TimestampQualificationStatus::TrustServiceVerified
        && qualification.trust_service_status != TimestampQualificationStatus::TrustServiceVerified
    {
        return Err(AppError::Data(
            "Certificate PDF contains an inconsistent verified Trust Service state.".into(),
        ));
    }
    let at_timestamp_qualified = qualification.status
        == TimestampQualificationStatus::QualifiedServiceVerified
        || qualification.eidas_qualification_status
            == TimestampQualificationStatus::QualifiedServiceVerified
        || qualification.qualification_at_timestamp
            == TimestampQualificationStatus::QualifiedServiceVerified;
    if at_timestamp_qualified
        && !(qualification.status == TimestampQualificationStatus::QualifiedServiceVerified
            && qualification.eidas_qualification_status
                == TimestampQualificationStatus::QualifiedServiceVerified
            && qualification.qualification_at_timestamp
                == TimestampQualificationStatus::QualifiedServiceVerified)
    {
        return Err(AppError::Data(
            "Certificate PDF contains inconsistent qualified-service status for the timestamp time."
                .into(),
        ));
    }
    Ok(())
}

fn validate_verified_qualification_evidence(
    metadata: &TimestampProviderMetadata,
    qualification: &crate::model::TimestampQualificationRecord,
    at_timestamp_qualified: bool,
    currently_qualified: bool,
    trust_service_verified: bool,
) -> Result<()> {
    let identity = &qualification.identity;
    let identity_verified = qualification.provider_identity_status
        == TimestampQualificationStatus::ProviderIdentityVerified;
    if identity_verified
        && (metadata.provider_identity_verified != Some(true)
            || metadata.certificate_sha256.trim().is_empty()
            || identity.certificate_sha256.trim().is_empty()
            || !metadata
                .certificate_sha256
                .eq_ignore_ascii_case(&identity.certificate_sha256))
    {
        return Err(AppError::Data(
            "Certificate PDF cannot report a verified provider identity without the matching cryptographically verified signer certificate."
                .into(),
        ));
    }
    if !(at_timestamp_qualified || currently_qualified || trust_service_verified) {
        return Ok(());
    }
    let trusted_list = qualification.trusted_list.as_ref().ok_or_else(|| {
        AppError::Data(
            "Certificate PDF cannot report a verified Trust Service without Trusted List evidence."
                .into(),
        )
    })?;
    if trusted_list.validation_status != TrustedListValidationStatus::Verified {
        return Err(AppError::Data(
            "Certificate PDF cannot report a verified Trust Service from an unverified Trusted List."
                .into(),
        ));
    }
    if qualification.provider_identity_status
        != TimestampQualificationStatus::ProviderIdentityVerified
        || identity.certificate_sha256.trim().is_empty()
    {
        return Err(AppError::Data(
            "Certificate PDF cannot report a verified Trust Service without a cryptographically verified signer identity."
                .into(),
        ));
    }
    if qualification.checked_at.trim().is_empty()
        || qualification.trust_service_provider.trim().is_empty()
        || qualification.trust_service_name.trim().is_empty()
        || qualification.service_type.trim().is_empty()
    {
        return Err(AppError::Data(
            "Certificate PDF cannot report a verified Trust Service without its check time and matched service identity."
                .into(),
        ));
    }
    Ok(())
}

fn validate_qualification_service_status(
    qualification: &crate::model::TimestampQualificationRecord,
    at_timestamp_qualified: bool,
    currently_qualified: bool,
) -> Result<()> {
    const QUALIFIED_TIMESTAMP_SERVICE_TYPE_URI: &str =
        "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST";
    const QUALIFIED_SERVICE_GRANTED_STATUS_URI: &str =
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted";

    if (at_timestamp_qualified || currently_qualified)
        && (qualification.service_type != QUALIFIED_TIMESTAMP_SERVICE_TYPE_URI
            || qualification.qualification_type != QUALIFIED_TIMESTAMP_SERVICE_TYPE_URI)
    {
        return Err(AppError::Data(
            "Certificate PDF qualified-service status does not identify the official qualified electronic time-stamp service type."
                .into(),
        ));
    }
    if at_timestamp_qualified
        && (qualification.service_status != QUALIFIED_SERVICE_GRANTED_STATUS_URI
            || qualification.status_valid_from.trim().is_empty())
    {
        return Err(AppError::Data(
            "Certificate PDF qualified-service status for the timestamp time is not backed by the official granted status URI and its status period."
                .into(),
        ));
    }
    if currently_qualified
        && (qualification.current_service_status != QUALIFIED_SERVICE_GRANTED_STATUS_URI
            || qualification.current_status_valid_from.trim().is_empty())
    {
        return Err(AppError::Data(
            "Certificate PDF current qualified-service status is not backed by the official granted status URI and its current status period."
                .into(),
        ));
    }
    Ok(())
}
