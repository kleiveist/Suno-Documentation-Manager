use super::*;

pub(super) fn phase_one_metadata(metadata: &EvidenceMetadata) -> Result<serde_json::Value> {
    let mut sanitized = serde_json::to_value(metadata)?;
    if let Some(object) = sanitized.as_object_mut() {
        for field in [
            "timestampType",
            "externalTimestamp",
            "referencedHash",
            "referencedArtifact",
            "externalReferenceId",
            "providerVerificationUrl",
        ] {
            object.remove(field);
        }
    }
    Ok(sanitized)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct AutomaticEvidenceRelationship {
    pub(super) kind: &'static str,
    pub(super) source_evidence_id: String,
    pub(super) source_role: &'static str,
    pub(super) target_evidence_id: String,
    pub(super) target_role: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct AutomaticGlobalTrackRelationship {
    pub(super) kind: &'static str,
    pub(super) source_global_evidence_id: String,
    pub(super) materialized_evidence_id: String,
    pub(super) role: &'static str,
    pub(super) target_track_id: String,
}

pub(super) fn automatic_role_relationships(
    evidence: &[&EvidenceItem],
) -> Vec<AutomaticEvidenceRelationship> {
    let mut relationships = Vec::new();

    append_role_relationships(
        evidence,
        EvidenceRole::SourceCodeFile,
        EvidenceRole::CodeGeneratedAudioFile,
        "source_to_generated_audio",
        &mut relationships,
    );
    for (source_role, target_role) in [
        (
            EvidenceRole::AiArtworkOriginal,
            EvidenceRole::AiArtworkEdited,
        ),
        (
            EvidenceRole::AiArtworkEdited,
            EvidenceRole::HumanEditedArtwork,
        ),
        (EvidenceRole::HumanEditedArtwork, EvidenceRole::FinalArtwork),
    ] {
        append_role_relationships(
            evidence,
            source_role,
            target_role,
            "artwork_stage",
            &mut relationships,
        );
    }

    relationships.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.source_evidence_id.cmp(&right.source_evidence_id))
            .then_with(|| left.target_evidence_id.cmp(&right.target_evidence_id))
    });
    relationships
}

pub(super) fn automatic_global_track_relationships(
    track_id: &str,
    evidence: &[&EvidenceItem],
) -> Vec<AutomaticGlobalTrackRelationship> {
    let mut relationships = evidence
        .iter()
        .copied()
        .filter(|item| item.provenance == EvidenceProvenance::GlobalCopy)
        .filter_map(|item| {
            item.source_global_evidence_id
                .as_deref()
                .filter(|source_id| !source_id.trim().is_empty())
                .map(|source_id| AutomaticGlobalTrackRelationship {
                    kind: "global_evidence_to_track",
                    source_global_evidence_id: source_id.to_owned(),
                    materialized_evidence_id: item.id.clone(),
                    role: item.role.as_str(),
                    target_track_id: track_id.to_owned(),
                })
        })
        .collect::<Vec<_>>();
    relationships.sort_by(|left, right| {
        left.source_global_evidence_id
            .cmp(&right.source_global_evidence_id)
            .then_with(|| {
                left.materialized_evidence_id
                    .cmp(&right.materialized_evidence_id)
            })
    });
    relationships
}

pub(super) fn append_role_relationships(
    evidence: &[&EvidenceItem],
    source_role: EvidenceRole,
    target_role: EvidenceRole,
    kind: &'static str,
    relationships: &mut Vec<AutomaticEvidenceRelationship>,
) {
    let sources = evidence
        .iter()
        .copied()
        .filter(|item| item.role == source_role)
        .collect::<Vec<_>>();
    let targets = evidence
        .iter()
        .copied()
        .filter(|item| item.role == target_role)
        .collect::<Vec<_>>();

    for target in &targets {
        let Some(source_id) = target
            .derived_from_evidence_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        if let Some(source) = sources
            .iter()
            .copied()
            .find(|source| source.id == source_id)
        {
            relationships.push(evidence_relationship(kind, source, target));
        }
    }

    // Role inference is only a safe ID-level statement when both concrete
    // stages are singletons and the target has no explicit lineage claim.
    if sources.len() == 1 && targets.len() == 1 && targets[0].derived_from_evidence_id.is_none() {
        relationships.push(evidence_relationship(kind, sources[0], targets[0]));
    }
}

pub(super) fn evidence_relationship(
    kind: &'static str,
    source: &EvidenceItem,
    target: &EvidenceItem,
) -> AutomaticEvidenceRelationship {
    AutomaticEvidenceRelationship {
        kind,
        source_evidence_id: source.id.clone(),
        source_role: source.role.as_str(),
        target_evidence_id: target.id.clone(),
        target_role: target.role.as_str(),
    }
}
