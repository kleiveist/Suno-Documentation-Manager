use crate::model::{
    EvidenceItem, EvidenceRole, Profile, TrackAutomation, TrackFields, TrackRecord,
};

pub(super) struct RenderContext<'a> {
    pub(super) track: &'a TrackRecord,
    pub(super) profile: &'a Profile,
    pub(super) evidence: Vec<EvidenceItem>,
    pub(super) fields: TrackFields,
    pub(super) automation: TrackAutomation,
    pub(super) artwork_present: bool,
    pub(super) ai_artwork: bool,
    pub(super) artwork_hash_status: &'static str,
}

impl<'a> RenderContext<'a> {
    pub(super) fn new(
        track: &'a TrackRecord,
        profile: &'a Profile,
        evidence: &[EvidenceItem],
    ) -> Self {
        let evidence = evidence
            .iter()
            .filter(|item| item.role != EvidenceRole::ExternalTimestamp)
            .cloned()
            .collect::<Vec<_>>();
        let automation = crate::workflow::automation_summary(track, &evidence);
        // Rendering is defensive as well as patch normalization: older workspace records may
        // still contain hidden answers from a formerly active conditional branch.
        let fields = track.fields.normalized_conditionals();
        let artwork_present = !matches!(fields.artwork_origin.as_str(), "" | "none");
        let ai_artwork = matches!(
            fields.artwork_origin.as_str(),
            "ai_generated" | "ai_assisted"
        );
        let artwork_hash_status = crate::workflow::human_edited_final_artwork_status(&evidence);
        Self {
            track,
            profile,
            evidence,
            fields,
            automation,
            artwork_present,
            ai_artwork,
            artwork_hash_status,
        }
    }

    pub(super) fn evidence_path(&self, role: EvidenceRole) -> &str {
        super::super::evidence::evidence_path(&self.evidence, role)
    }

    pub(super) fn ai_original(&self) -> &str {
        self.evidence
            .iter()
            .find(|item| item.role == EvidenceRole::AiArtworkOriginal)
            .map(|item| item.relative_path.as_str())
            .unwrap_or("NOT DOCUMENTED")
    }

    pub(super) fn final_artwork(&self) -> &str {
        self.evidence
            .iter()
            .find(|item| item.role == EvidenceRole::FinalArtwork)
            .map(|item| item.relative_path.as_str())
            .unwrap_or("NOT DOCUMENTED")
    }
}
