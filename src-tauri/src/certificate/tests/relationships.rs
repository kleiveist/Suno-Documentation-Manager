use super::*;

#[test]
fn automatic_relationships_cover_only_unambiguous_adjacent_role_pairs() {
    let source = relationship_evidence("source", EvidenceRole::SourceCodeFile);
    let generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
    let artwork_original =
        relationship_evidence("artwork-original", EvidenceRole::AiArtworkOriginal);
    let artwork_ai_edited =
        relationship_evidence("artwork-ai-edited", EvidenceRole::AiArtworkEdited);
    let artwork_human_edited =
        relationship_evidence("artwork-human-edited", EvidenceRole::HumanEditedArtwork);
    let artwork_final = relationship_evidence("artwork-final", EvidenceRole::FinalArtwork);
    let mut global = relationship_evidence("terms-copy", EvidenceRole::SunoTermsRights);
    global.provenance = EvidenceProvenance::GlobalCopy;
    global.source_global_evidence_id = Some("global-terms".into());
    let evidence = [
        &source,
        &generated,
        &artwork_original,
        &artwork_ai_edited,
        &artwork_human_edited,
        &artwork_final,
        &global,
    ];

    let relationships = automatic_role_relationships(&evidence);

    assert_eq!(relationships.len(), 4);
    assert!(relationships.iter().any(|relationship| {
        relationship.kind == "source_to_generated_audio"
            && relationship.source_evidence_id == "source"
            && relationship.target_evidence_id == "generated"
    }));
    assert!(relationships.iter().any(|relationship| {
        relationship.kind == "artwork_stage"
            && relationship.source_evidence_id == "artwork-original"
            && relationship.target_evidence_id == "artwork-ai-edited"
    }));
    assert!(relationships.iter().any(|relationship| {
        relationship.kind == "artwork_stage"
            && relationship.source_evidence_id == "artwork-ai-edited"
            && relationship.target_evidence_id == "artwork-human-edited"
    }));
    assert!(relationships.iter().any(|relationship| {
        relationship.kind == "artwork_stage"
            && relationship.source_evidence_id == "artwork-human-edited"
            && relationship.target_evidence_id == "artwork-final"
    }));
    assert!(relationships
        .iter()
        .all(|relationship| relationship.kind != "global_copy"));

    let global_relationships = automatic_global_track_relationships("track-1", &evidence);
    assert_eq!(
        global_relationships,
        vec![AutomaticGlobalTrackRelationship {
            kind: "global_evidence_to_track",
            source_global_evidence_id: "global-terms".into(),
            materialized_evidence_id: "terms-copy".into(),
            role: EvidenceRole::SunoTermsRights.as_str(),
            target_track_id: "track-1".into(),
        }]
    );
}

#[test]
fn explicit_lineage_disambiguates_multiple_sources_without_cartesian_products() {
    let source_one = relationship_evidence("source-one", EvidenceRole::SourceCodeFile);
    let source_two = relationship_evidence("source-two", EvidenceRole::SourceCodeFile);
    let mut generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
    generated.derived_from_evidence_id = Some(source_two.id.clone());
    let artwork_one = relationship_evidence("artwork-one", EvidenceRole::AiArtworkOriginal);
    let artwork_two = relationship_evidence("artwork-two", EvidenceRole::AiArtworkOriginal);
    let mut artwork_edited = relationship_evidence("artwork-edited", EvidenceRole::AiArtworkEdited);
    artwork_edited.derived_from_evidence_id = Some(artwork_one.id.clone());
    let evidence = [
        &source_one,
        &source_two,
        &generated,
        &artwork_one,
        &artwork_two,
        &artwork_edited,
    ];

    let relationships = automatic_role_relationships(&evidence);

    assert_eq!(relationships.len(), 2);
    assert!(relationships.iter().any(|relationship| {
        relationship.source_evidence_id == "source-two"
            && relationship.target_evidence_id == "generated"
    }));
    assert!(relationships.iter().any(|relationship| {
        relationship.source_evidence_id == "artwork-one"
            && relationship.target_evidence_id == "artwork-edited"
    }));
    assert!(relationships.iter().all(|relationship| {
        relationship.source_evidence_id != "source-one"
            && relationship.source_evidence_id != "artwork-two"
    }));
}

#[test]
fn ambiguous_roles_without_explicit_lineage_emit_no_id_relationship() {
    let source_one = relationship_evidence("source-one", EvidenceRole::SourceCodeFile);
    let source_two = relationship_evidence("source-two", EvidenceRole::SourceCodeFile);
    let generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
    let artwork_one = relationship_evidence("artwork-one", EvidenceRole::AiArtworkOriginal);
    let artwork_two = relationship_evidence("artwork-two", EvidenceRole::AiArtworkOriginal);
    let artwork_edited = relationship_evidence("artwork-edited", EvidenceRole::AiArtworkEdited);
    let evidence = [
        &source_one,
        &source_two,
        &generated,
        &artwork_one,
        &artwork_two,
        &artwork_edited,
    ];

    assert!(automatic_role_relationships(&evidence).is_empty());
}

#[test]
fn artwork_relationships_never_skip_a_concrete_stage() {
    let original = relationship_evidence("original", EvidenceRole::AiArtworkOriginal);
    let human_edited = relationship_evidence("human-edited", EvidenceRole::HumanEditedArtwork);
    let final_artwork = relationship_evidence("final", EvidenceRole::FinalArtwork);
    let evidence = [&original, &human_edited, &final_artwork];

    let relationships = automatic_role_relationships(&evidence);

    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].source_evidence_id, "human-edited");
    assert_eq!(relationships[0].target_evidence_id, "final");
}

#[test]
fn invalid_or_non_adjacent_explicit_lineage_is_not_replaced_by_role_inference() {
    let source = relationship_evidence("source", EvidenceRole::SourceCodeFile);
    let mut generated = relationship_evidence("generated", EvidenceRole::CodeGeneratedAudioFile);
    generated.derived_from_evidence_id = Some("missing-source".into());
    let original = relationship_evidence("original", EvidenceRole::AiArtworkOriginal);
    let human_edited = relationship_evidence("human-edited", EvidenceRole::HumanEditedArtwork);
    let mut final_artwork = relationship_evidence("final", EvidenceRole::FinalArtwork);
    final_artwork.derived_from_evidence_id = Some(original.id.clone());
    let evidence = [
        &source,
        &generated,
        &original,
        &human_edited,
        &final_artwork,
    ];

    assert!(automatic_role_relationships(&evidence).is_empty());
}
