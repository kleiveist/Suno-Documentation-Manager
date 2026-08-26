use super::*;
use crate::model::{
    CertificateState, DocumentState, EvidenceProvenance, IntegrityState, TrackFields,
    TrackLibraryPlacement, TrackStatus,
};

const CERTIFICATE_ID: &str = "SUNODM-CERT-2026-0001";
const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

struct Fixture {
    track: TrackRecord,
    profile: Profile,
    steps: Vec<StepState>,
    evidence: Vec<EvidenceItem>,
    deviations: Vec<BlockingDeviation>,
    revision_references: Vec<String>,
    artwork_previews: Vec<CertificateArtworkPreview>,
    finalization_timestamp: FinalizationTimestampSnapshot,
    render_options: CertificateRenderOptions,
}

impl Fixture {
    fn new(evidence_count: usize) -> Self {
        let workflow_config = workflow::config().expect("workflow fixture");
        let profile = Profile {
            artist_name: "Künstlerin Änne Öster".into(),
            suno_profile_name: "Studio Überklang".into(),
            suno_handle: "@ueberklang".into(),
            suno_plan: "Pro".into(),
            subscription_start_date: "2025-01-01".into(),
            ..Profile::default()
        };
        let fields = TrackFields {
            title: "Größe & Präzision".into(),
            production_start_date: "2026-01-02".into(),
            production_end_date: "2026-01-03".into(),
            final_export_date: "2026-01-04".into(),
            suno_model: "v4.5".into(),
            suno_project_url: "https://suno.example/project/fixture".into(),
            suno_project_version_id: "project-version-fixture".into(),
            suno_final_generation_id: "generation-fixture".into(),
            suno_final_generation_date: "2026-01-03".into(),
            suno_download_export_date: "2026-01-04".into(),
            suno_plan_at_generation: "Pro".into(),
            instrumental_track: Some(false),
            vocal_lyrics_present: Some(true),
            vocal_intent: Some(VocalIntent::Vocal),
            suno_content_classification: Some(SunoContentClassification::VocalLyricsOnly),
            suno_lyrics_content_source: Some(SunoLyricsContentSource::Mixed),
            suno_lyrics_field_text: "A documented vocal line".into(),
            external_audio_uploaded: Some(false),
            own_audio_uploaded: Some(false),
            code_based_generation: Some(true),
            third_party_samples_uploaded: Some(false),
            human_editing_performed: Some(true),
            human_editing_details: "Timing and cuts; EQ; Mixing".into(),
            post_export_editing_performed: Some(false),
            artwork_origin: "human".into(),
            generative_ai_used: Some(false),
            ..TrackFields::default()
        };
        let track = TrackRecord {
            id: "track-fixture".into(),
            relative_path: ".".into(),
            status: TrackStatus::Ready,
            workflow_id: workflow_config.id.clone(),
            workflow_version: workflow_config.version.clone(),
            profile_snapshot: profile.clone(),
            library: TrackLibraryPlacement::default(),
            field_origins: Default::default(),
            fields,
            audio_screening: Default::default(),
            documents: DocumentState::default(),
            integrity: IntegrityState::default(),
            certificate: CertificateState::default(),
            created_at: "2026-01-02T00:00:00Z".into(),
            updated_at: "2026-01-04T00:00:00Z".into(),
            legacy: false,
        };
        let steps = workflow_config
            .steps
            .into_iter()
            .map(|step| StepState {
                id: step.id.clone(),
                status: if step.id == "ai_transparency" {
                    StepStatus::NotApplicable
                } else {
                    StepStatus::Pass
                },
                na_reason: (step.id == "ai_transparency")
                    .then(|| "Kein AI-Artwork dokumentiert".into()),
                updated_at: Some("2026-01-04T00:00:00Z".into()),
            })
            .collect();
        let evidence = (0..evidence_count).map(evidence_item).collect();
        Self {
            track,
            profile,
            steps,
            evidence,
            deviations: Vec::new(),
            revision_references: Vec::new(),
            artwork_previews: Vec::new(),
            finalization_timestamp: FinalizationTimestampSnapshot::default(),
            render_options: CertificateRenderOptions::default(),
        }
    }

    fn generate(&self) -> Vec<u8> {
        self.generate_result()
            .expect("generate fixture certificate PDF")
    }

    fn generate_result(&self) -> Result<Vec<u8>> {
        let evidence_refs = self.evidence.iter().collect::<Vec<_>>();
        let automation = workflow::automation_summary(&self.track, &self.evidence);
        generate_pdf(&CertificatePdfSnapshot {
            track: &self.track,
            automation: &automation,
            profile: &self.profile,
            steps: &self.steps,
            evidence: &evidence_refs,
            deviations: &self.deviations,
            revision_references: &self.revision_references,
            certificate_id: CERTIFICATE_ID,
            finalized_at: "2026-01-04T12:34:56Z",
            certificate_version: crate::certificate::CERTIFICATE_FORMAT_VERSION,
            sha256sums_sha256: DIGEST_A,
            evidence_manifest_sha256: DIGEST_B,
            markdown_certificate_sha256: DIGEST_C,
            finalization_timestamp: self.finalization_timestamp.clone(),
            artwork_previews: &self.artwork_previews,
            render_options: self.render_options,
        })
    }
}

fn evidence_item(index: usize) -> EvidenceItem {
    let role = match index % 3 {
        0 => EvidenceRole::ReleaseWav,
        1 => EvidenceRole::SourceCodeFile,
        _ => EvidenceRole::SunoScreenshot,
    };
    EvidenceItem {
        id: format!("evidence-{index:04}"),
        role,
        file_name: format!("evidence-{index:04}.dat"),
        relative_path: format!("02_SUNO/evidence-{index:04}.dat"),
        sha256: Some(format!("{value:064x}", value = index + 1)),
        size_bytes: 1_024 + index as u64,
        imported_at: "2026-01-04T10:00:00Z".into(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: index
            .is_multiple_of(4)
            .then(|| format!("global-{index:04}")),
        coverage_start: index.is_multiple_of(5).then(|| "2026-01-01".into()),
        coverage_end: index.is_multiple_of(5).then(|| "2026-01-31".into()),
        provenance: if index.is_multiple_of(4) {
            EvidenceProvenance::GlobalCopy
        } else {
            EvidenceProvenance::ManagedCopy
        },
        derived_from_evidence_id: index
            .is_multiple_of(6)
            .then(|| format!("source-{index:04}")),
        generator_version: index.is_multiple_of(7).then(|| "generator-1.0".into()),
        generated_disclosure_text: None,
        metadata: Default::default(),
    }
}

fn parse_text(bytes: &[u8]) -> (PdfDocument, String) {
    let mut warnings = Vec::new();
    let document = PdfDocument::parse(bytes, &PdfParseOptions::default(), &mut warnings)
        .expect("parse generated certificate PDF");
    assert!(
        warnings
            .iter()
            .all(|warning| warning.severity != PdfParseErrorSeverity::Error),
        "parser errors: {warnings:?}"
    );
    let text = document
        .extract_text()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    (document, text)
}

fn normalized_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn write_review_pdf(file_name: &str, bytes: &[u8]) {
    let Some(review_directory) = std::env::var_os("SUNODM_PDF_REVIEW_DIR") else {
        return;
    };
    let review_directory = std::path::PathBuf::from(review_directory);
    std::fs::create_dir_all(&review_directory).expect("manual PDF review directory");
    std::fs::write(review_directory.join(file_name), bytes)
        .expect("manual certificate PDF review fixture");
}

mod archive;
mod content;
mod rendering;
mod timestamps;
