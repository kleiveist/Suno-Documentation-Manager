use super::*;

mod manifest;
mod markdown_certificate;
mod pdf;

use manifest::prepare_manifest;
use markdown_certificate::render_markdown_certificate;
use pdf::{generate_pdfs, publish_generated_certificate};

pub struct GenerationInput<'a> {
    pub(crate) track_root: &'a Path,
    pub(crate) track: &'a TrackRecord,
    pub(crate) profile: &'a Profile,
    pub(crate) steps: &'a [StepState],
    pub(crate) evidence: &'a [EvidenceItem],
    pub(crate) deviations: &'a [BlockingDeviation],
    pub(crate) certificate_id: &'a str,
    pub(crate) finalized_at: &'a str,
    pub(crate) transaction_id: &'a str,
    pub(crate) render_options: CertificateRenderOptions,
}

struct GenerationRequest<'a> {
    input: GenerationInput<'a>,
    timestamp_resolver: Option<&'a mut TimestampResolver<'a>>,
    #[cfg(test)]
    failure: Option<CertificateGenerationFailure>,
}

type TimestampResolver<'a> = dyn FnMut(&str, &[u8]) -> FinalizationTimestampSnapshot + 'a;

#[cfg(test)]
pub(crate) fn generate(input: GenerationInput<'_>) -> Result<()> {
    generate_impl(GenerationRequest {
        input,
        timestamp_resolver: None,
        #[cfg(test)]
        failure: None,
    })
}

/// Generate the immutable certificate after resolving the concrete timestamp
/// state for the already serialized manifest anchor. The resolver is called
/// exactly once and before either language PDF is rendered.
pub fn generate_with_finalization_timestamp(
    input: GenerationInput<'_>,
    timestamp_resolver: &mut dyn FnMut(&str, &[u8]) -> FinalizationTimestampSnapshot,
) -> Result<()> {
    generate_impl(GenerationRequest {
        input,
        timestamp_resolver: Some(timestamp_resolver),
        #[cfg(test)]
        failure: None,
    })
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CertificateGenerationFailure {
    PdfGeneration,
    PdfStaging,
    PdfPublication,
    PostPublishVerification,
}

#[cfg(test)]
pub(crate) fn generate_with_failure(
    input: GenerationInput<'_>,
    failure: CertificateGenerationFailure,
) -> Result<()> {
    generate_impl(GenerationRequest {
        input,
        timestamp_resolver: None,
        failure: Some(failure),
    })
}

fn generate_impl(mut request: GenerationRequest<'_>) -> Result<()> {
    let manifest = prepare_manifest(&request.input)?;
    let finalization_timestamp = request
        .timestamp_resolver
        .as_deref_mut()
        .map(|resolver| resolver(&manifest.manifest_sha, &manifest.manifest_bytes))
        .unwrap_or_default();
    let markdown = render_markdown_certificate(&request.input, &manifest, &finalization_timestamp);
    #[cfg(test)]
    if request.failure == Some(CertificateGenerationFailure::PdfGeneration) {
        return Err(AppError::Data(
            "Injected technical PDF generation failure.".into(),
        ));
    }
    let pdfs = generate_pdfs(
        &request.input,
        &manifest,
        &markdown,
        &finalization_timestamp,
    )?;
    publish_generated_certificate(
        &request.input,
        &manifest,
        &markdown,
        &pdfs,
        #[cfg(test)]
        request.failure,
    )
}
