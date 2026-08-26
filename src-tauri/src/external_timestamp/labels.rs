use super::*;

pub fn qualification_status_label(value: TimestampQualificationStatus) -> &'static str {
    match value {
        TimestampQualificationStatus::NotChecked => "NOT CHECKED",
        TimestampQualificationStatus::NotDocumented => "NOT DOCUMENTED",
        TimestampQualificationStatus::NotVerified => "NOT VERIFIED",
        TimestampQualificationStatus::ProviderIdentityVerified => "PROVIDER IDENTITY VERIFIED",
        TimestampQualificationStatus::TrustServiceVerified => "TRUST SERVICE VERIFIED",
        TimestampQualificationStatus::QualifiedServiceVerified => "QUALIFIED SERVICE VERIFIED",
        TimestampQualificationStatus::CheckFailed => "CHECK FAILED",
    }
}

pub fn timestamp_status_label(value: ExternalTimestampStatus) -> &'static str {
    match value {
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

pub fn timestamp_type_label(value: TimestampType) -> &'static str {
    match value {
        TimestampType::QualifiedElectronicTimestampUserDeclared => {
            "Qualified electronic timestamp — user declared"
        }
        TimestampType::ElectronicTimestamp => "Electronic timestamp",
        TimestampType::ExternalIntegrityTimestamp => "External integrity timestamp",
        TimestampType::Other => "Other",
        TimestampType::NotDocumented => "NOT DOCUMENTED",
    }
}

pub fn referenced_artifact_label(value: TimestampReferencedArtifact) -> &'static str {
    match value {
        TimestampReferencedArtifact::EvidenceManifest => "EVIDENCE_MANIFEST.json",
        TimestampReferencedArtifact::Sha256sums => "SHA256SUMS.txt",
        TimestampReferencedArtifact::DocumentationCertificateMarkdown => {
            "DOCUMENTATION_CERTIFICATE.md"
        }
        TimestampReferencedArtifact::CertificatePdf => "Certificate PDF",
        TimestampReferencedArtifact::FinalEvidencePackage => "Final Evidence Package",
        TimestampReferencedArtifact::Other => "Other",
    }
}
