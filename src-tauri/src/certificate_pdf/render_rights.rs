use super::*;

pub(super) fn render_license_and_rights(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    layout.section_title("H. License and rights evidence");
    let terms = snapshot
        .evidence
        .iter()
        .copied()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .collect::<Vec<_>>();
    let rows = license_summary_rows(snapshot, &terms);
    layout.table_rows(&rows, 112.0);
    if terms.is_empty() {
        layout.write_localized(
            "No archived terms evidence recorded.",
            LEFT_MM,
            RIGHT_MM - LEFT_MM,
            7.2,
            BuiltinFont::Helvetica,
            1.3,
        );
        layout.write_localized("Factual archive and coverage status only. No rights ownership, license validity, legality, or non-infringement conclusion is made.", LEFT_MM, RIGHT_MM - LEFT_MM, 7.2, BuiltinFont::Helvetica, 1.3);
        return;
    }

    layout.section_title("Archived service-terms evidence");
    render_archived_terms_overview(layout, snapshot, &terms);
    let term_rows = archived_terms_rows(&terms);
    // Keep the longest provenance label on one extracted-text line. Values may
    // still wrap in the remaining column, which is preferable to interleaving a
    // value inside the label in accessibility/search text.
    layout.table_rows(&term_rows, 112.0);
    layout.write_localized("Factual archive and coverage status only. No rights ownership, license validity, legality, or non-infringement conclusion is made.", LEFT_MM, RIGHT_MM - LEFT_MM, 7.2, BuiltinFont::Helvetica, 1.3);
}

fn license_summary_rows(
    snapshot: &CertificatePdfSnapshot<'_>,
    terms: &[&EvidenceItem],
) -> Vec<TableRow> {
    let terms_ids = if terms.is_empty() {
        "N/A".to_owned()
    } else {
        terms
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut rows = vec![
        TableRow::documented_plain("Suno plan at generation [User-confirmed fact]", &snapshot.track.fields.suno_plan_at_generation),
        TableRow::system_plain("Assigned subscription evidence jointly covers the production period [System verification]", subscription_production_coverage(snapshot)),
        TableRow::system_plain("Final-generation date covered [System verification]", subscription_coverage(snapshot)),
        TableRow::system_plain("Terms evidence exists [System verification]", if terms.is_empty() { "NO" } else { "YES" }),
        if terms.is_empty() {
            TableRow::system_mono("Terms evidence IDs [System value]", "N/A")
        } else {
            TableRow::mono("Terms evidence IDs [System value]", terms_ids)
        },
        TableRow::system_plain("Terms evidence not available [User-confirmed fact]", yes_no(snapshot.track.fields.suno_terms_evidence_not_available)),
    ];
    for (index, item) in snapshot
        .evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SubscriptionPayment)
        .enumerate()
    {
        rows.push(TableRow::mono(
            format!("Subscription evidence {} path", index + 1),
            &item.relative_path,
        ));
        rows.push(TableRow::optional_plain(
            format!("Subscription evidence {} coverage start", index + 1),
            item.coverage_start.as_deref(),
            "NOT DOCUMENTED",
        ));
        rows.push(TableRow::optional_plain(
            format!("Subscription evidence {} coverage end", index + 1),
            item.coverage_end.as_deref(),
            "NOT DOCUMENTED",
        ));
        rows.push(TableRow::optional_mono(
            format!("Subscription evidence {} SHA-256", index + 1),
            item.sha256.as_deref(),
            "NOT RECORDED",
        ));
    }
    for role in [
        EvidenceRole::ExternalAudioLicense,
        EvidenceRole::ThirdPartySampleLicense,
    ] {
        for item in snapshot.evidence.iter().filter(|item| item.role == role) {
            rows.push(TableRow::mono(
                format!("{} evidence", role.as_str()),
                &item.relative_path,
            ));
        }
    }
    rows
}

fn render_archived_terms_overview(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
    terms: &[&EvidenceItem],
) {
    let production_end = parse_iso_date(&snapshot.track.fields.production_end_date);
    let mut applicable_terms = Vec::new();
    let mut future_terms = Vec::new();
    let mut other_terms = Vec::new();
    for item in terms {
        let future = production_end.is_some_and(|production_end| {
            parse_iso_date(&item.metadata.effective_date)
                .is_some_and(|effective| effective > production_end)
        });
        if future {
            future_terms.push(*item);
        } else if !item.metadata.applicable_production_period.trim().is_empty() {
            // The applicability statement is user-confirmed evidence metadata;
            // SunoDM adds no legal interpretation to it.
            applicable_terms.push(*item);
        } else {
            other_terms.push(*item);
        }
    }
    render_terms_overview_group(
        layout,
        "Terms applicable to documented production period",
        &applicable_terms,
        false,
    );
    render_terms_overview_group(layout, "Future archived Terms", &future_terms, true);
    render_terms_overview_group(layout, "Other archived Terms", &other_terms, false);
}

fn archived_terms_rows(terms: &[&EvidenceItem]) -> Vec<TableRow> {
    let mut term_rows = Vec::new();
    for (index, item) in terms.iter().enumerate() {
        term_rows.push(TableRow::mono(
            format!("Terms document {} evidence ID [System value]", index + 1),
            &item.id,
        ));
        term_rows.push(TableRow::documented_plain(
            format!("Terms document {} title [User-confirmed fact]", index + 1),
            &item.metadata.document_title,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} provider/source [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.provider,
        ));
        term_rows.push(TableRow::documented_mono(
            format!(
                "Terms document {} source URL [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.source_url,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} retrieval date [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.retrieval_date,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} effective date [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.effective_date,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} applicable production period [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.applicable_production_period,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} factual note [User-confirmed fact]",
                index + 1
            ),
            &item.metadata.factual_note,
        ));
        term_rows.push(TableRow::mono(
            format!("Terms document {} path [System value]", index + 1),
            &item.relative_path,
        ));
        term_rows.push(TableRow::documented_plain(
            format!(
                "Terms document {} original filename [Evidence-derived metadata]",
                index + 1
            ),
            &item.metadata.original_file_name,
        ));
        term_rows.push(TableRow::optional_mono(
            format!("Terms document {} SHA-256 [System verification]", index + 1),
            item.sha256.as_deref(),
            "NOT RECORDED",
        ));
        term_rows.push(TableRow::mono(
            format!("Terms document {} imported at [System value]", index + 1),
            &item.imported_at,
        ));
        term_rows.push(TableRow::plain(
            format!("Terms document {} provenance [System value]", index + 1),
            item.provenance.as_str(),
        ));
    }
    term_rows
}

pub(super) fn render_terms_overview_group(
    layout: &mut PdfLayout,
    heading: &str,
    terms: &[&EvidenceItem],
    future: bool,
) {
    if terms.is_empty() {
        return;
    }
    layout.write_localized(
        heading,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.2,
        BuiltinFont::HelveticaBold,
        1.3,
    );
    let mut rows = Vec::new();
    for (index, item) in terms.iter().enumerate() {
        rows.push(TableRow::documented_plain(
            format!("Terms version {}", index + 1),
            &item.metadata.document_title,
        ));
        rows.push(TableRow::documented_plain(
            format!("Terms version {} effective date", index + 1),
            &item.metadata.effective_date,
        ));
        rows.push(TableRow::documented_plain(
            format!("Terms version {} documented production period", index + 1),
            &item.metadata.applicable_production_period,
        ));
        if future && !item.metadata.effective_date.trim().is_empty() {
            rows.push(TableRow::plain(
                layout.label("Not applicable to production before"),
                &item.metadata.effective_date,
            ));
        }
    }
    layout.table_rows(&rows, 68.0);
}

pub(super) fn render_external_timestamp_base(
    layout: &mut PdfLayout,
    snapshot: &CertificatePdfSnapshot<'_>,
) {
    let timestamp = &snapshot.finalization_timestamp;
    let provider = if timestamp.provider.trim().is_empty() {
        "NOT DOCUMENTED"
    } else {
        timestamp.provider.as_str()
    };

    layout.section_title("I. External Timestamp Evidence");
    layout.write_localized(
        "A. Technical timestamp",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        8.0,
        BuiltinFont::HelveticaBold,
        1.3,
    );
    let mut technical_rows = vec![
        TableRow::system_plain(
            "Provider configuration",
            provider_configuration_status_label(timestamp.provider_configuration_status),
        ),
        TableRow::plain(
            "Provider configuration detail",
            &timestamp.provider_configuration_message,
        ),
        TableRow::plain("Provider", provider),
        TableRow::system_plain(
            "Automatic request for this finalization",
            if timestamp.automatic_request_enabled {
                "ENABLED"
            } else {
                "DISABLED"
            },
        ),
        TableRow::system_plain(
            "Concrete timestamp status",
            provider_verification_status_label(timestamp.technical_status),
        ),
        TableRow::plain("Concrete timestamp detail", &timestamp.technical_message),
        TableRow::plain("Referenced artifact", "EVIDENCE_MANIFEST.json"),
        TableRow::mono("Manifest anchor SHA-256", snapshot.evidence_manifest_sha256),
    ];
    append_technical_provider_rows(&mut technical_rows, timestamp);
    layout.table_rows(&technical_rows, 70.0);

    layout.write_localized(
        "B. Provider trust and qualification",
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        8.0,
        BuiltinFont::HelveticaBold,
        1.3,
    );
    let qualification_rows = finalization_timestamp_qualification_rows(timestamp);
    layout.table_rows(&qualification_rows, 78.0);
    layout.write_localized(
        EXTERNAL_TIMESTAMP_DISCLAIMER,
        LEFT_MM,
        RIGHT_MM - LEFT_MM,
        7.4,
        BuiltinFont::Helvetica,
        1.35,
    );
}

pub(super) fn finalization_timestamp_qualification_rows(
    timestamp: &FinalizationTimestampSnapshot,
) -> Vec<TableRow> {
    let qualification = timestamp
        .provider_metadata
        .as_ref()
        .and_then(|metadata| metadata.qualification.as_ref());
    let status =
        |field: fn(&crate::model::TimestampQualificationRecord) -> TimestampQualificationStatus| {
            qualification.map(field).unwrap_or_default()
        };
    let mut rows = Vec::new();
    for (label, value) in [
        (
            "Provider identity",
            status(|value| value.provider_identity_status),
        ),
        ("Trust Service", status(|value| value.trust_service_status)),
        (
            "eIDAS qualification",
            status(|value| value.eidas_qualification_status),
        ),
        (
            "Qualification at timestamp",
            status(|value| value.qualification_at_timestamp),
        ),
        (
            "Current qualification",
            status(|value| value.current_qualification_status),
        ),
    ] {
        rows.push(TableRow::system_plain(
            label,
            qualification_status_label(value),
        ));
    }

    let Some(qualification) = qualification else {
        rows.push(TableRow::system_plain(
            "Qualification detail",
            "No validated qualification source was checked. This does not mean that the provider is unsafe or not qualified.",
        ));
        return rows;
    };
    push_finalization_qualification_details(&mut rows, qualification);
    push_finalization_trusted_list_rows(&mut rows, qualification);
    rows
}

pub(super) fn push_documented_detail(rows: &mut Vec<TableRow>, label: &str, value: &str) {
    if !value.trim().is_empty() {
        rows.push(TableRow::plain(label, value));
    }
}

fn append_technical_provider_rows(
    rows: &mut Vec<TableRow>,
    timestamp: &FinalizationTimestampSnapshot,
) {
    if let Some(metadata) = timestamp.provider_metadata.as_ref() {
        for (label, value, mono) in [
            ("Timestamp protocol", metadata.protocol.as_str(), false),
            ("Hash algorithm", metadata.request_algorithm.as_str(), false),
            (
                "Provider response format",
                metadata.response_format.as_str(),
                false,
            ),
            ("Provider adapter", metadata.adapter.as_str(), false),
            ("Timestamp value", timestamp.timestamp_value.as_str(), true),
            (
                "Provider reference ID",
                timestamp.external_reference_id.as_str(),
                true,
            ),
            (
                "Provider endpoint",
                timestamp.provider_verification_url.as_str(),
                true,
            ),
            ("Policy OID", metadata.policy_oid.as_str(), true),
            (
                "Service identifier",
                metadata
                    .qualification
                    .as_ref()
                    .map(|value| value.identity.service_identifier.as_str())
                    .unwrap_or(""),
                true,
            ),
            (
                "Signer certificate subject",
                metadata.certificate_subject.as_str(),
                false,
            ),
            ("Signer certificate issuer", metadata.issuer.as_str(), false),
            (
                "Signer certificate serial",
                metadata.certificate_serial_number.as_str(),
                true,
            ),
            (
                "Signer certificate SHA-256",
                metadata.certificate_sha256.as_str(),
                true,
            ),
            (
                "Technical verification timestamp",
                metadata.verification_timestamp.as_str(),
                true,
            ),
        ] {
            push_documented_row(rows, label, value, mono);
        }
        for (label, value) in [
            (
                "Provider response structurally valid",
                yes_no(metadata.response_structure_valid),
            ),
            (
                "Manifest hash binding",
                match metadata.provider_digest_match {
                    Some(true) => "VERIFIED",
                    Some(false) => "NOT VERIFIED",
                    None => "NOT CHECKED",
                },
            ),
            (
                "Timestamp signature applicable",
                yes_no(metadata.signature_verification_applicable),
            ),
            (
                "Timestamp signature valid",
                yes_no(metadata.signature_verified),
            ),
            (
                "Certificate chain applicable",
                yes_no(metadata.trust_chain_verification_applicable),
            ),
            (
                "Certificate chain technically verified",
                yes_no(metadata.trust_chain_verified),
            ),
            (
                "Provider identity technically recognized",
                yes_no(metadata.provider_identity_verified),
            ),
            ("Policy match", yes_no(metadata.policy_match)),
        ] {
            rows.push(TableRow::system_plain(label, value));
        }
    } else {
        rows.push(TableRow::system_plain(
            "Timestamp protocol",
            "NOT DOCUMENTED",
        ));
        rows.push(TableRow::system_plain("Hash binding", "NOT VERIFIED"));
    }
}

fn push_finalization_qualification_details(
    rows: &mut Vec<TableRow>,
    qualification: &crate::model::TimestampQualificationRecord,
) {
    if qualification.eidas_qualification_status
        == TimestampQualificationStatus::QualifiedServiceVerified
    {
        rows.push(TableRow::system_plain(
            "Verified higher qualification",
            "eIDAS QUALIFIED TRUST SERVICE – VERIFIED",
        ));
    }
    for (label, value, mono) in [
        (
            "Qualification checked at",
            qualification.checked_at.as_str(),
            true,
        ),
        (
            "Qualification result detail",
            qualification.message.as_str(),
            false,
        ),
        (
            "Recognized Trust Service Provider",
            qualification.trust_service_provider.as_str(),
            false,
        ),
        (
            "Recognized Trust Service",
            qualification.trust_service_name.as_str(),
            false,
        ),
        (
            "Trust Service type",
            qualification.service_type.as_str(),
            false,
        ),
        (
            "Service status at timestamp",
            qualification.service_status.as_str(),
            false,
        ),
        (
            "Current service status",
            qualification.current_service_status.as_str(),
            false,
        ),
        (
            "Trust Service identifier",
            qualification.service_identifier.as_str(),
            true,
        ),
        (
            "Qualification type",
            qualification.qualification_type.as_str(),
            false,
        ),
        (
            "Timestamp-time status valid from",
            qualification.status_valid_from.as_str(),
            true,
        ),
        (
            "Timestamp-time status valid until",
            qualification.status_valid_until.as_str(),
            true,
        ),
        (
            "Current status valid from",
            qualification.current_status_valid_from.as_str(),
            true,
        ),
        (
            "Current status valid until",
            qualification.current_status_valid_until.as_str(),
            true,
        ),
        (
            "Identity certificate SHA-256",
            qualification.identity.certificate_sha256.as_str(),
            true,
        ),
        (
            "Identity certificate subject",
            qualification.identity.certificate_subject.as_str(),
            false,
        ),
        (
            "Identity certificate issuer",
            qualification.identity.certificate_issuer.as_str(),
            false,
        ),
        (
            "Identity certificate serial",
            qualification.identity.certificate_serial_number.as_str(),
            true,
        ),
        (
            "Identity policy OID",
            qualification.identity.policy_oid.as_str(),
            true,
        ),
    ] {
        push_documented_row(rows, label, value, mono);
    }
}

fn push_finalization_trusted_list_rows(
    rows: &mut Vec<TableRow>,
    qualification: &crate::model::TimestampQualificationRecord,
) {
    if let Some(source) = qualification.trusted_list.as_ref() {
        for (label, value, mono) in [
            ("Trusted List source", source.source.as_str(), true),
            ("Trusted List territory", source.territory.as_str(), false),
            ("Trusted List version", source.version.as_str(), false),
            (
                "Trusted List sequence",
                source.sequence_number.as_str(),
                false,
            ),
            ("Trusted List issued at", source.issued_at.as_str(), true),
            (
                "Trusted List next update",
                source.next_update.as_str(),
                true,
            ),
            ("Trusted List SHA-256", source.sha256.as_str(), true),
            (
                "Trusted List validated at",
                source.validated_at.as_str(),
                true,
            ),
        ] {
            push_documented_row(rows, label, value, mono);
        }
        rows.push(TableRow::system_plain(
            "Trusted List validation",
            trusted_list_validation_status_label(source.validation_status),
        ));
    }
}
