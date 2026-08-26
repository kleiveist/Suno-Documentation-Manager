use super::*;

#[cfg(test)]
pub(super) const QUALIFIED_TIMESTAMP_SERVICE_TYPE: &str =
    "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST";
#[cfg(test)]
pub(super) const QUALIFIED_SERVICE_GRANTED_STATUS: &str =
    "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted";

#[derive(Debug, Clone)]
#[cfg(test)]
pub(crate) struct TrustedServiceStatusPeriod {
    pub valid_from: String,
    pub valid_until: String,
    pub status_uri: String,
}

#[derive(Debug, Clone)]
#[cfg(test)]
pub(crate) struct ValidatedTrustedService {
    pub certificate_sha256: String,
    pub policy_oid: String,
    pub provider_name: String,
    pub service_name: String,
    pub service_type: String,
    pub service_identifier: String,
    pub periods: Vec<TrustedServiceStatusPeriod>,
}

#[derive(Debug, Clone)]
#[cfg(test)]
pub(crate) struct ValidatedTrustedListSnapshot {
    pub evidence: TrustedListEvidence,
    pub services: Vec<ValidatedTrustedService>,
}

/// Match only the cryptographically verified signer certificate (and policy
/// when the Trusted List service specifies one). Provider labels and endpoint
/// URLs are deliberately not inputs to this classifier. The caller may only
/// supply a snapshot whose LOTL/TL validation completed successfully.
#[cfg(test)]
pub(crate) fn verify_provider_qualification(
    identity: &TimestampServiceIdentity,
    timestamp_value: &str,
    checked_at: &str,
    snapshot: &ValidatedTrustedListSnapshot,
) -> TimestampQualificationRecord {
    let checked_at_value = parse_qualification_time(checked_at);
    let timestamp = parse_qualification_time(timestamp_value);
    let base = TimestampQualificationRecord {
        provider_identity_status: TimestampQualificationStatus::ProviderIdentityVerified,
        checked_at: checked_at.into(),
        identity: identity.clone(),
        trusted_list: Some(snapshot.evidence.clone()),
        ..Default::default()
    };
    if snapshot.evidence.validation_status != TrustedListValidationStatus::Verified {
        return TimestampQualificationRecord {
            status: TimestampQualificationStatus::CheckFailed,
            trust_service_status: TimestampQualificationStatus::NotVerified,
            eidas_qualification_status: TimestampQualificationStatus::NotVerified,
            current_qualification_status: TimestampQualificationStatus::NotVerified,
            qualification_at_timestamp: TimestampQualificationStatus::NotVerified,
            message: "Qualification lookup failed because the Trusted List source was not cryptographically validated.".into(),
            ..base
        };
    }
    let service = snapshot.services.iter().find(|service| {
        service
            .certificate_sha256
            .eq_ignore_ascii_case(&identity.certificate_sha256)
            && (service.policy_oid.is_empty() || service.policy_oid == identity.policy_oid)
    });
    let Some(service) = service else {
        return TimestampQualificationRecord {
            status: TimestampQualificationStatus::NotVerified,
            trust_service_status: TimestampQualificationStatus::NotVerified,
            eidas_qualification_status: TimestampQualificationStatus::NotVerified,
            current_qualification_status: TimestampQualificationStatus::NotVerified,
            qualification_at_timestamp: TimestampQualificationStatus::NotVerified,
            message: "The verified timestamp signer identity could not be matched to a service in the validated Trusted List snapshot. This is not a finding that the provider is unsafe or unqualified.".into(),
            ..base
        };
    };
    let timestamp_period = timestamp
        .as_ref()
        .and_then(|value| qualification_period_at(&service.periods, value));
    let current_period = checked_at_value
        .as_ref()
        .and_then(|value| qualification_period_at(&service.periods, value));
    let qualified_service_type = service.service_type == QUALIFIED_TIMESTAMP_SERVICE_TYPE;
    let qualified_at_timestamp = qualified_service_type
        && timestamp_period
            .is_some_and(|period| period.status_uri == QUALIFIED_SERVICE_GRANTED_STATUS);
    let currently_qualified = qualified_service_type
        && current_period
            .is_some_and(|period| period.status_uri == QUALIFIED_SERVICE_GRANTED_STATUS);
    let qualification_at_timestamp = if qualified_at_timestamp {
        TimestampQualificationStatus::QualifiedServiceVerified
    } else {
        TimestampQualificationStatus::NotVerified
    };
    let current_qualification_status = if currently_qualified {
        TimestampQualificationStatus::QualifiedServiceVerified
    } else {
        TimestampQualificationStatus::NotVerified
    };
    TimestampQualificationRecord {
        status: if qualified_at_timestamp {
            TimestampQualificationStatus::QualifiedServiceVerified
        } else {
            TimestampQualificationStatus::TrustServiceVerified
        },
        trust_service_status: TimestampQualificationStatus::TrustServiceVerified,
        eidas_qualification_status: qualification_at_timestamp,
        current_qualification_status,
        qualification_at_timestamp,
        message: if qualified_at_timestamp {
            "The cryptographically verified timestamp signer was matched to a qualified electronic time-stamp service in a validated Trusted List for the timestamp time.".into()
        } else if currently_qualified {
            "A Trust Service entry was verified and is currently qualified, but a qualified electronic time-stamp service status for the timestamp time was not established.".into()
        } else {
            "A Trust Service entry was verified, but a qualified electronic time-stamp service status for the timestamp time was not established.".into()
        },
        trust_service_provider: service.provider_name.clone(),
        trust_service_name: service.service_name.clone(),
        service_type: service.service_type.clone(),
        service_status: timestamp_period
            .map(|value| value.status_uri.clone())
            .unwrap_or_default(),
        current_service_status: current_period
            .map(|value| value.status_uri.clone())
            .unwrap_or_default(),
        service_identifier: service.service_identifier.clone(),
        qualification_type: if qualified_service_type {
            QUALIFIED_TIMESTAMP_SERVICE_TYPE.into()
        } else {
            String::new()
        },
        status_valid_from: timestamp_period
            .map(|value| value.valid_from.clone())
            .unwrap_or_default(),
        status_valid_until: timestamp_period
            .map(|value| value.valid_until.clone())
            .unwrap_or_default(),
        current_status_valid_from: current_period
            .map(|value| value.valid_from.clone())
            .unwrap_or_default(),
        current_status_valid_until: current_period
            .map(|value| value.valid_until.clone())
            .unwrap_or_default(),
        ..base
    }
}

#[cfg(test)]
pub(super) fn parse_qualification_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
pub(super) fn qualification_period_at<'a>(
    periods: &'a [TrustedServiceStatusPeriod],
    value: &DateTime<Utc>,
) -> Option<&'a TrustedServiceStatusPeriod> {
    periods.iter().find(|period| {
        let Some(valid_from) = parse_qualification_time(&period.valid_from) else {
            return false;
        };
        if valid_from > *value {
            return false;
        }
        period.valid_until.is_empty()
            || parse_qualification_time(&period.valid_until)
                .is_some_and(|valid_until| *value < valid_until)
    })
}

pub(super) fn load_trust_anchors(
    path: &Path,
) -> std::result::Result<(Vec<CertificateDer<'static>>, Vec<String>), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "The configured TSA trust-anchor file cannot be read.".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("The configured TSA trust anchor must be a regular, non-symlink file.".into());
    }
    if metadata.len() == 0 || metadata.len() > MAX_TRUST_ANCHOR_BYTES {
        return Err(format!(
            "The configured TSA trust-anchor file must contain 1 to {MAX_TRUST_ANCHOR_BYTES} bytes."
        ));
    }
    let bytes = fs::read(path)
        .map_err(|_| "The configured TSA trust-anchor file cannot be read.".to_owned())?;
    let roots = if bytes.starts_with(b"-----BEGIN") {
        rustls_pemfile::certs(&mut Cursor::new(bytes.as_slice()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| "The configured TSA trust-anchor PEM is invalid.".to_owned())?
    } else {
        vec![CertificateDer::from(bytes)]
    };
    if roots.is_empty() {
        return Err("The configured TSA trust-anchor file contains no certificates.".into());
    }
    let fingerprints = roots
        .iter()
        .map(|certificate| sha256_bytes(certificate.as_ref()))
        .collect();
    Ok((roots, fingerprints))
}
