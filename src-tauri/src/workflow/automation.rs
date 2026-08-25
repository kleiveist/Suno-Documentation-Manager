use super::evaluation::present;
use super::evaluation::verified_role;
use super::*;

pub fn original_evidence_file_name(evidence: &[EvidenceItem], role: EvidenceRole) -> Option<&str> {
    evidence
        .iter()
        .find(|item| verified_role(item, role))
        .and_then(|item| {
            let value = item.metadata.original_file_name.trim();
            (!value.is_empty()).then_some(value)
        })
}

pub fn automation_summary(track: &TrackRecord, evidence: &[EvidenceItem]) -> TrackAutomation {
    let suno = relevant_suno_export(evidence);
    let final_generation_id_origin = suno_id_fact_origin(
        &track.fields.suno_final_generation_id,
        track.field_origins.suno_final_generation_id.as_ref(),
        suno,
    );
    let final_generation_origin = fact_origin(
        &track.fields.suno_final_generation_date,
        track.field_origins.suno_final_generation_date.as_ref(),
        suno,
    );
    let production_end_origin = fact_origin(
        &track.fields.production_end_date,
        track.field_origins.production_end_date.as_ref(),
        suno,
    );
    let download_export_origin = fact_origin(
        &track.fields.suno_download_export_date,
        track.field_origins.suno_download_export_date.as_ref(),
        suno,
    );
    let final_export_origin = fact_origin(
        &track.fields.final_export_date,
        track.field_origins.final_export_date.as_ref(),
        suno,
    );
    let byte_identical_pairs = byte_identical_pairs(evidence);
    let release_identical_to_suno_export = byte_identical_pairs.iter().any(|pair| {
        matches!(
            (pair.left_role, pair.right_role),
            (EvidenceRole::SunoFinalExport, EvidenceRole::ReleaseWav)
                | (EvidenceRole::ReleaseWav, EvidenceRole::SunoFinalExport)
        )
    });
    TrackAutomation {
        final_generation_id_origin,
        final_generation_origin,
        production_end_origin,
        download_export_origin,
        final_export_origin,
        suno_metadata_detected: suno.is_some_and(|item| item.metadata.suno_studio_detected),
        suno_created_timestamp: suno
            .map(|item| item.metadata.suno_created_timestamp.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        suno_id: suno
            .map(|item| item.metadata.suno_id.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        release_identical_to_suno_export,
        byte_identical_pairs,
        consistency_issues: consistency_issues(track, evidence),
    }
}

pub fn byte_identical_pairs(evidence: &[EvidenceItem]) -> Vec<ByteIdenticalPair> {
    let mut verified = evidence
        .iter()
        .filter(|item| {
            item.verified
                && item.verification_error.is_none()
                && item.sha256.as_deref().is_some_and(|hash| !hash.is_empty())
        })
        .collect::<Vec<_>>();
    verified.sort_by(|left, right| left.id.cmp(&right.id));
    let mut pairs = Vec::new();
    for (index, left) in verified.iter().enumerate() {
        for right in verified.iter().skip(index + 1) {
            if left.sha256 != right.sha256 {
                continue;
            }
            pairs.push(ByteIdenticalPair {
                left_evidence_id: left.id.clone(),
                left_role: left.role,
                right_evidence_id: right.id.clone(),
                right_role: right.role,
                sha256: left.sha256.clone().unwrap_or_default(),
            });
        }
    }
    pairs
}

/// Compare verified human-edited artwork with the single verified final
/// artwork. A mismatch is an informational technical fact only; it must never
/// become a workflow blocker or consistency issue.
pub fn human_edited_final_artwork_sha256_match(evidence: &[EvidenceItem]) -> Option<bool> {
    let human_edited = evidence
        .iter()
        .filter(|item| verified_role(item, EvidenceRole::HumanEditedArtwork))
        .collect::<Vec<_>>();
    let final_artwork = evidence
        .iter()
        .filter(|item| verified_role(item, EvidenceRole::FinalArtwork))
        .collect::<Vec<_>>();
    match final_artwork.as_slice() {
        [final_artwork] if !human_edited.is_empty() => Some(
            human_edited
                .iter()
                .any(|item| item.sha256 == final_artwork.sha256),
        ),
        _ => None,
    }
}

pub fn human_edited_final_artwork_status(evidence: &[EvidenceItem]) -> &'static str {
    match human_edited_final_artwork_sha256_match(evidence) {
        Some(true) => "BYTE-IDENTICAL / SHA-256 MATCH",
        Some(false) => "NO SHA-256 MATCH",
        None => "NOT VERIFIED",
    }
}

pub fn consistency_issues(track: &TrackRecord, evidence: &[EvidenceItem]) -> Vec<ConsistencyIssue> {
    let mut issues = Vec::new();
    let suno = relevant_suno_export(evidence);
    if let Some(item) = suno {
        let metadata = &item.metadata;
        let embedded_suno_values = metadata
            .embedded_metadata
            .iter()
            .map(|entry| entry.value.as_str())
            .filter(|value| has_suno_metadata_marker(value))
            .collect::<Vec<_>>();
        let distinct_embedded_suno_values =
            embedded_suno_values.iter().copied().collect::<HashSet<_>>();
        let has_any_suno_state = metadata.suno_studio_detected
            || !metadata.suno_raw_metadata.is_empty()
            || !metadata.suno_created_timestamp.is_empty()
            || !metadata.suno_created_date.is_empty()
            || !metadata.suno_id.is_empty()
            || !embedded_suno_values.is_empty();
        if distinct_embedded_suno_values.len() > 1 {
            issues.push(issue(
                "suno_metadata_ambiguous",
                "Mehrere widersprüchliche Suno-Metadatensätze wurden erkannt; kein Datum wurde automatisch ausgewählt.",
                "suno",
            ));
        } else if has_any_suno_state
            && !stored_suno_metadata_matches(metadata, &embedded_suno_values)
        {
            issues.push(issue(
                "suno_stored_metadata_mismatch",
                "Gespeicherte Suno-Metadaten stimmen nicht mit dem erhaltenen eingebetteten Wert überein.",
                "suno",
            ));
        }
    }

    if track
        .field_origins
        .suno_final_generation_id
        .as_ref()
        .is_some_and(|origin| !derived_suno_id_origin_matches(origin, suno))
    {
        issues.push(issue(
            "suno_generation_id_origin_stale",
            "Die gespeicherte Evidence-Herkunft verweist nicht mehr auf den aktuellen Suno-Export.",
            "suno",
        ));
    }

    for (code, origin) in [
        (
            "suno_generation_origin_stale",
            track.field_origins.suno_final_generation_date.as_ref(),
        ),
        (
            "production_end_origin_stale",
            track.field_origins.production_end_date.as_ref(),
        ),
        (
            "download_export_origin_stale",
            track.field_origins.suno_download_export_date.as_ref(),
        ),
        (
            "final_export_origin_stale",
            track.field_origins.final_export_date.as_ref(),
        ),
    ] {
        if origin.is_some_and(|origin| !derived_origin_matches(origin, suno)) {
            issues.push(issue(
                code,
                "Die gespeicherte Evidence-Herkunft verweist nicht mehr auf den aktuellen Suno-Export.",
                "suno",
            ));
        }
    }

    if evidence.iter().any(|item| {
        verified_role(item, EvidenceRole::HumanEditedArtwork)
            && !human_artwork_editing_documented(track)
    }) {
        issues.push(issue(
            "human_artwork_editing_undocumented",
            "Menschlich bearbeitetes Artwork ist vorhanden, aber die Bearbeitung ist nicht dokumentiert.",
            "artwork",
        ));
    }

    if track.fields.suno_terms_evidence_not_available == Some(true)
        && evidence
            .iter()
            .any(|item| verified_role(item, EvidenceRole::SunoTermsRights))
    {
        issues.push(issue(
            "terms_evidence_availability_conflict",
            "Verifizierte Terms-Evidence ist vorhanden, widerspricht aber der Angabe, dass sie nicht verfügbar sei.",
            "evidence_licenses",
        ));
    }

    let ids = evidence
        .iter()
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    if evidence.iter().any(|item| {
        item.derived_from_evidence_id
            .as_deref()
            .is_some_and(|source_id| source_id == item.id || !ids.contains(source_id))
    }) {
        issues.push(issue(
            "referenced_evidence_missing",
            "Eine automatisch referenzierte Evidence-Datei fehlt.",
            "evidence_licenses",
        ));
    }

    issues.sort_by(|left, right| left.code.cmp(&right.code));
    issues.dedup_by(|left, right| left.code == right.code);
    issues
}

fn relevant_suno_export(evidence: &[EvidenceItem]) -> Option<&EvidenceItem> {
    evidence
        .iter()
        .find(|item| verified_role(item, EvidenceRole::SunoFinalExport))
}

fn stored_suno_metadata_matches(
    metadata: &crate::model::EvidenceMetadata,
    embedded_suno_values: &[&str],
) -> bool {
    if !metadata.suno_studio_detected
        || embedded_suno_values.is_empty()
        || embedded_suno_values
            .iter()
            .any(|value| *value != metadata.suno_raw_metadata)
    {
        return false;
    }
    let Some(parsed) = parse_suno_metadata(&metadata.suno_raw_metadata) else {
        return false;
    };
    metadata.suno_created_timestamp == parsed.created_timestamp
        && metadata.suno_created_date == parsed.created_date
        && metadata.suno_id == parsed.id
}

fn fact_origin(
    value: &str,
    origin: Option<&EvidenceDerivedField>,
    suno: Option<&EvidenceItem>,
) -> FactOrigin {
    if value.trim().is_empty() {
        FactOrigin::NotDocumented
    } else if origin
        .is_some_and(|origin| origin.value == value && derived_origin_matches(origin, suno))
    {
        FactOrigin::EvidenceDerivedMetadata
    } else {
        FactOrigin::UserConfirmedFact
    }
}

fn suno_id_fact_origin(
    value: &str,
    origin: Option<&EvidenceDerivedField>,
    suno: Option<&EvidenceItem>,
) -> FactOrigin {
    if value.trim().is_empty() {
        FactOrigin::NotDocumented
    } else if origin
        .is_some_and(|origin| origin.value == value && derived_suno_id_origin_matches(origin, suno))
    {
        FactOrigin::EvidenceDerivedMetadata
    } else {
        FactOrigin::UserConfirmedFact
    }
}

fn derived_origin_matches(origin: &EvidenceDerivedField, suno: Option<&EvidenceItem>) -> bool {
    suno.is_some_and(|item| {
        origin.evidence_id == item.id
            && origin.evidence_sha256 == item.sha256.clone().unwrap_or_default()
            && origin.original_value == item.metadata.suno_created_timestamp
            && origin.value == item.metadata.suno_created_date
    })
}

fn derived_suno_id_origin_matches(
    origin: &EvidenceDerivedField,
    suno: Option<&EvidenceItem>,
) -> bool {
    suno.is_some_and(|item| {
        origin.evidence_id == item.id
            && origin.evidence_sha256 == item.sha256.clone().unwrap_or_default()
            && origin.original_value == item.metadata.suno_id
            && origin.value == item.metadata.suno_id
            && item.metadata.suno_studio_detected
            && parse_suno_metadata(&item.metadata.suno_raw_metadata)
                .is_some_and(|parsed| parsed.id == item.metadata.suno_id)
    })
}

fn human_artwork_editing_documented(track: &TrackRecord) -> bool {
    let fields = &track.fields;
    match fields.artwork_origin.as_str() {
        "human" => {
            fields
                .human_artwork_process_operations
                .iter()
                .any(|value| present(value))
                || present(&fields.human_artwork_process_notes)
        }
        "ai_assisted" => {
            fields
                .human_artwork_modifications
                .iter()
                .any(|value| present(value))
                || present(&fields.custom_artwork_change)
        }
        _ => false,
    }
}

fn issue(code: &str, message: &str, step_id: &str) -> ConsistencyIssue {
    ConsistencyIssue {
        code: code.into(),
        message: message.into(),
        step_id: step_id.into(),
        blocking: true,
    }
}
