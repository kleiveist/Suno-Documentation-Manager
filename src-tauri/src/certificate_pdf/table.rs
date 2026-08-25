use super::*;

pub(super) struct TableRow {
    pub(super) label: String,
    pub(super) value: String,
    pub(super) value_font: BuiltinFont,
    pub(super) value_size_pt: f32,
    pub(super) localize_value: bool,
}

impl TableRow {
    pub(super) fn plain(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            value_font: BuiltinFont::Helvetica,
            value_size_pt: 7.1,
            localize_value: false,
        }
    }

    pub(super) fn mono(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            value_font: BuiltinFont::Courier,
            value_size_pt: 6.4,
            localize_value: false,
        }
    }

    /// Render a certificate-owned status/value in the selected language.
    /// User-, evidence-, and provider-derived strings must use `plain`/`mono`
    /// so a literal value such as "NO" can never be mistaken for a status.
    pub(super) fn system_plain(label: impl Into<String>, value: impl Into<String>) -> Self {
        let mut row = Self::plain(label, value);
        row.localize_value = true;
        row
    }

    pub(super) fn system_mono(label: impl Into<String>, value: impl Into<String>) -> Self {
        let mut row = Self::mono(label, value);
        row.localize_value = true;
        row
    }

    pub(super) fn documented_plain(label: impl Into<String>, value: &str) -> Self {
        if value.trim().is_empty() {
            Self::system_plain(label, "NOT DOCUMENTED")
        } else {
            Self::plain(label, value)
        }
    }

    pub(super) fn documented_mono(label: impl Into<String>, value: &str) -> Self {
        if value.trim().is_empty() {
            Self::system_mono(label, "NOT DOCUMENTED")
        } else {
            Self::mono(label, value)
        }
    }

    pub(super) fn documented_list_plain(label: impl Into<String>, values: &[String]) -> Self {
        let value = documented_list(values);
        if value == "NOT DOCUMENTED" {
            Self::system_plain(label, "NOT DOCUMENTED")
        } else {
            Self::plain(label, value)
        }
    }

    pub(super) fn conditional_plain(
        label: impl Into<String>,
        controlling: Option<bool>,
        value: &str,
    ) -> Self {
        match controlling {
            Some(true) => Self::documented_plain(label, value),
            Some(false) => Self::system_plain(label, "N/A"),
            None => Self::system_plain(label, "NOT DOCUMENTED"),
        }
    }

    pub(super) fn conditional_optional_plain(
        label: impl Into<String>,
        controlling: Option<bool>,
        value: Option<&str>,
        missing: &str,
    ) -> Self {
        match controlling {
            Some(true) => match value.filter(|value| !value.trim().is_empty()) {
                Some(value) => Self::plain(label, value),
                None => Self::system_plain(label, missing),
            },
            Some(false) => Self::system_plain(label, "N/A"),
            None => Self::system_plain(label, "NOT DOCUMENTED"),
        }
    }

    pub(super) fn optional_plain(
        label: impl Into<String>,
        value: Option<&str>,
        fallback: &str,
    ) -> Self {
        match value.filter(|value| !value.trim().is_empty()) {
            Some(value) => Self::plain(label, value),
            None => Self::system_plain(label, fallback),
        }
    }

    pub(super) fn optional_mono(
        label: impl Into<String>,
        value: Option<&str>,
        fallback: &str,
    ) -> Self {
        match value.filter(|value| !value.trim().is_empty()) {
            Some(value) => Self::mono(label, value),
            None => Self::system_mono(label, fallback),
        }
    }

    pub(super) fn optional_u64_plain(
        label: impl Into<String>,
        value: Option<u64>,
        fallback: &str,
    ) -> Self {
        match value {
            Some(value) => Self::plain(label, value.to_string()),
            None => Self::system_plain(label, fallback),
        }
    }

    pub(super) fn provider_summary(
        label: impl Into<String>,
        value: impl Into<String>,
        is_system: bool,
    ) -> Self {
        if is_system {
            Self::system_plain(label, value)
        } else {
            Self::plain(label, value)
        }
    }
}

pub(super) fn localized_table_value(options: CertificateRenderOptions, value: &str) -> String {
    if matches!(
        value,
        "DOCUMENTATION COMPLETE"
            | "configured documentation requirements completed"
            | "Configured documentation requirements for this step were satisfied."
            | "User-confirmed fact"
            | "Evidence-derived metadata"
            | "System verification"
            | "System value"
            | "Post-finalization addendum; phase-one snapshot remains unchanged"
            | "MULTIPLE — see Technical Appendix"
            | "FINGERPRINT GENERATED"
            | "NO MATCH DETECTED"
            | "MATCH DETECTED"
            | "NO MATCH"
            | "NOT RUN"
            | "NONE RECORDED"
            | "DYNAMIC BY TRACK DURATION"
            | "FIXED REFERENCE DURATION"
            | "eIDAS QUALIFIED TRUST SERVICE – VERIFIED"
            | "No validated qualification source was checked. This does not mean that the provider is unsafe or not qualified."
            | "VERIFICATION CONFIGURATION INCOMPLETE"
            | "PROVIDER IDENTITY VERIFIED"
            | "QUALIFIED SERVICE VERIFIED"
            | "TRUST SERVICE VERIFIED"
            | "AUTHENTICATION REQUIRED"
            | "CONFIGURATION INCOMPLETE"
            | "VERIFICATION FAILED"
            | "CONNECTION FAILED"
            | "UNSUPPORTED RESPONSE"
            | "ANCHOR MISMATCH"
            | "PROVIDER ERROR"
            | "CHECK FAILED"
            | "NOT CHECKED"
            | "REQUESTING"
            | "ATTACHED"
            | "ENABLED"
            | "VERIFIED"
            | "FAILED"
            | "READY"
            | "DISABLED"
            | "NOT CONFIGURED"
            | "AUTHENTICATION FAILED"
            | "PROVIDER UNAVAILABLE"
            | "CONFIGURATION INVALID"
            | "SKIPPED NOT CONFIGURED"
            | "ENGINE UNAVAILABLE"
            | "UNSUPPORTED FORMAT"
            | "PROCESSING FAILED"
            | "STALE"
            | "NOT_RUN"
            | "FAIL"
            | "BLOCKED"
            | "NOT_VERIFIED"
            | "INCOMPLETE – generative AI use NOT DOCUMENTED"
            | "Potential deepfake-related indicator recorded"
            | "Documented deepfake-related indicators: none recorded"
            | "Deepfake-related indicator documentation: INCOMPLETE"
            | "BYTE-IDENTICAL / SHA-256 MATCH"
            | "NO SHA-256 MATCH"
            | "VOCAL_LYRICS_ONLY"
            | "STRUCTURE_ONLY"
            | "MIXED"
            | "OTHER"
            | "EMPTY"
            | "VOCAL"
            | "INSTRUMENTAL"
            | "UNSPECIFIED"
            | "Full numbered list in Technical Appendix"
            | "MULTI-SAMPLE"
            | "YES"
            | "NO"
            | "NOT DOCUMENTED"
            | "NOT RECORDED"
            | "NOT VERIFIED"
            | "NOT AVAILABLE"
            | "NOT COVERED"
            | "DOCUMENTED"
            | "COMPLETE"
    ) {
        localized_certificate_label(options, value)
    } else {
        value.to_owned()
    }
}
