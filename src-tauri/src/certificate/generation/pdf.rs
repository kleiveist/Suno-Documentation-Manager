use super::manifest::GeneratedManifest;
use super::markdown_certificate::GeneratedMarkdown;
use super::*;

pub(super) struct GeneratedPdfs {
    pdf_en: Vec<u8>,
    pdf_de: Vec<u8>,
}

pub(super) fn generate_pdfs(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
    markdown: &GeneratedMarkdown,
    finalization_timestamp: &FinalizationTimestampSnapshot,
) -> Result<GeneratedPdfs> {
    let artwork_previews =
        certificate_pdf::prepare_artwork_previews(input.track_root, &manifest.evidence_values);
    let pdf_en = certificate_pdf::generate_pdf(&CertificatePdfSnapshot {
        track: input.track,
        automation: &manifest.automation,
        profile: input.profile,
        steps: input.steps,
        evidence: &manifest.evidence_values,
        deviations: input.deviations,
        revision_references: &manifest.archived_revisions,
        certificate_id: input.certificate_id,
        finalized_at: input.finalized_at,
        certificate_version: CERTIFICATE_FORMAT_VERSION,
        sha256sums_sha256: &manifest.hash_manifest_sha,
        evidence_manifest_sha256: &manifest.manifest_sha,
        markdown_certificate_sha256: &markdown.sha256,
        finalization_timestamp: finalization_timestamp.clone(),
        artwork_previews: &artwork_previews,
        render_options: CertificateRenderOptions {
            language: CertificateLanguage::En,
            bilingual: false,
        },
    })?;
    let pdf_de = certificate_pdf::generate_pdf(&CertificatePdfSnapshot {
        track: input.track,
        automation: &manifest.automation,
        profile: input.profile,
        steps: input.steps,
        evidence: &manifest.evidence_values,
        deviations: input.deviations,
        revision_references: &manifest.archived_revisions,
        certificate_id: input.certificate_id,
        finalized_at: input.finalized_at,
        certificate_version: CERTIFICATE_FORMAT_VERSION,
        sha256sums_sha256: &manifest.hash_manifest_sha,
        evidence_manifest_sha256: &manifest.manifest_sha,
        markdown_certificate_sha256: &markdown.sha256,
        finalization_timestamp: finalization_timestamp.clone(),
        artwork_previews: &artwork_previews,
        render_options: CertificateRenderOptions {
            language: CertificateLanguage::De,
            bilingual: false,
        },
    })?;
    Ok(GeneratedPdfs { pdf_en, pdf_de })
}

pub(super) fn publish_generated_certificate(
    input: &GenerationInput<'_>,
    manifest: &GeneratedManifest<'_>,
    markdown: &GeneratedMarkdown,
    pdfs: &GeneratedPdfs,
    #[cfg(test)] failure: Option<CertificateGenerationFailure>,
) -> Result<()> {
    let pdf_en_sha = sha256_bytes(&pdfs.pdf_en);
    let pdf_de_sha = sha256_bytes(&pdfs.pdf_de);
    let certificate_hashes = format!(
        "{}  {}\n{}  {}\n{}  {}\n{}  {}\n{}  {}\n",
        manifest.hash_manifest_sha,
        HASH_FILE,
        manifest.manifest_sha,
        MANIFEST_FILE,
        markdown.sha256,
        CERTIFICATE_FILE,
        pdf_en_sha,
        PDF_FILE,
        pdf_de_sha,
        PDF_FILE_DE,
    );
    publish_certificate_set_impl(
        input.track_root,
        CertificateSetBytes {
            manifest: &manifest.manifest_bytes,
            certificate: markdown.certificate.as_bytes(),
            pdf_en: &pdfs.pdf_en,
            pdf_de: &pdfs.pdf_de,
            certificate_hashes: certificate_hashes.as_bytes(),
        },
        input.transaction_id,
        #[cfg(test)]
        failure.and_then(CertificateGenerationFailure::publication_failure),
    )
}
