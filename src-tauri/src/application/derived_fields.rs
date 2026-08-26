use super::*;

/// Treats a valid date from the current verified Suno export as authoritative
/// for the Suno and production facts. The last-editing date follows the WAV
/// only after the user confirms that no desktop editing took place.
pub(super) fn reconcile_evidence_derived_fields(
    track: &mut TrackRecord,
    evidence: &[EvidenceItem],
) -> bool {
    let suno = evidence.iter().find(|item| {
        item.role == EvidenceRole::SunoFinalExport
            && item.verified
            && item.verification_error.is_none()
            && item.sha256.is_some()
            && item.metadata.suno_studio_detected
            && !item.metadata.suno_created_timestamp.trim().is_empty()
            && !item.metadata.suno_created_date.trim().is_empty()
    });
    let derived = suno.map(|item| EvidenceDerivedField {
        value: item.metadata.suno_created_date.clone(),
        original_value: item.metadata.suno_created_timestamp.clone(),
        evidence_id: item.id.clone(),
        evidence_sha256: item.sha256.clone().unwrap_or_default(),
    });
    let derived_id = suno.and_then(|item| {
        let id = item.metadata.suno_id.trim();
        (!id.is_empty()).then(|| EvidenceDerivedField {
            value: id.to_owned(),
            // The structured UUID is already normalized by the strict WAV
            // parser. Keeping it as the original value lets us distinguish
            // this ID-specific origin from the timestamp-derived date facts.
            original_value: id.to_owned(),
            evidence_id: item.id.clone(),
            evidence_sha256: item.sha256.clone().unwrap_or_default(),
        })
    });

    let mut changed = reconcile_evidence_derived_id(
        &mut track.fields.suno_final_generation_id,
        &mut track.field_origins.suno_final_generation_id,
        derived_id.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.suno_final_generation_date,
        &mut track.field_origins.suno_final_generation_date,
        derived.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.production_end_date,
        &mut track.field_origins.production_end_date,
        derived.as_ref(),
    );
    changed |= reconcile_derived_value(
        &mut track.fields.suno_download_export_date,
        &mut track.field_origins.suno_download_export_date,
        derived.as_ref(),
    );
    let final_export_derived = (track.fields.post_export_editing_performed == Some(false))
        .then_some(derived.as_ref())
        .flatten();
    changed |= reconcile_derived_value(
        &mut track.fields.final_export_date,
        &mut track.field_origins.final_export_date,
        final_export_derived,
    );
    changed
}

pub(super) fn reconcile_derived_value(
    field: &mut String,
    origin: &mut Option<EvidenceDerivedField>,
    derived: Option<&EvidenceDerivedField>,
) -> bool {
    let previous_field = field.clone();
    let previous_origin = origin.clone();

    // A value that no longer equals the last automatic assignment has been
    // edited by the user and immediately ceases to be system-owned.
    if origin
        .as_ref()
        .is_some_and(|recorded| field != &recorded.value)
    {
        *origin = None;
    }

    if let Some(value) = derived {
        // Valid metadata wins over a submitted fallback value. The UI also
        // renders these fields read-only, while this assignment enforces the
        // same invariant for every native caller.
        *field = value.value.clone();
        *origin = Some(value.clone());
    } else {
        if origin
            .as_ref()
            .is_some_and(|recorded| field == &recorded.value)
        {
            field.clear();
        }
        *origin = None;
    }

    *field != previous_field || *origin != previous_origin
}

/// Unlike the date facts, a final-generation ID entered by the user is never
/// replaced by WAV metadata. An automatic ID is nevertheless kept tied to its
/// exact evidence so replacement and removal update or clear only that
/// system-owned value.
pub(super) fn reconcile_evidence_derived_id(
    field: &mut String,
    origin: &mut Option<EvidenceDerivedField>,
    derived: Option<&EvidenceDerivedField>,
) -> bool {
    let previous_field = field.clone();
    let previous_origin = origin.clone();

    // A changed automatic value is a user-confirmed override. The frontend
    // sends a full draft, so equality (rather than patch presence) is the only
    // reliable way to preserve automatic ownership across unrelated saves.
    if origin
        .as_ref()
        .is_some_and(|recorded| field != &recorded.value)
    {
        *origin = None;
    }

    match (origin.as_ref(), derived) {
        // Only values previously owned by this automation follow a replacement
        // or are cleared when the Suno evidence disappears.
        (Some(_), Some(value)) => {
            *field = value.value.clone();
            *origin = Some(value.clone());
        }
        (Some(recorded), None) => {
            if field == &recorded.value {
                field.clear();
            }
            *origin = None;
        }
        // A non-empty field without an automatic origin is a manual value and
        // must never be overwritten. A blank field is intentionally filled.
        (None, Some(value)) if field.trim().is_empty() => {
            *field = value.value.clone();
            *origin = Some(value.clone());
        }
        _ => {}
    }

    *field != previous_field || *origin != previous_origin
}
