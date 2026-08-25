use super::presentation::value_or_missing;
use crate::model::{EvidenceItem, EvidenceRole};

pub(super) fn evidence_path(evidence: &[EvidenceItem], role: crate::model::EvidenceRole) -> &str {
    evidence
        .iter()
        .find(|item| item.role == role)
        .map(|item| item.relative_path.as_str())
        .unwrap_or("NOT DOCUMENTED")
}

pub(super) fn technically_verified(item: &EvidenceItem) -> bool {
    item.verified
        && item.verification_error.is_none()
        && item.sha256.as_deref().is_some_and(|hash| !hash.is_empty())
}

pub(super) fn release_identity_status(evidence: &[EvidenceItem]) -> &'static str {
    let release = evidence
        .iter()
        .find(|item| item.role == EvidenceRole::ReleaseWav && technically_verified(item));
    let suno = evidence
        .iter()
        .find(|item| item.role == EvidenceRole::SunoFinalExport && technically_verified(item));
    match (release, suno) {
        (Some(release), Some(suno)) if release.sha256 == suno.sha256 => "YES",
        (Some(_), Some(_)) => "NO",
        _ => "NOT VERIFIED",
    }
}

pub(super) fn terms_evidence_status(evidence: &[EvidenceItem]) -> &'static str {
    let terms = evidence
        .iter()
        .filter(|item| item.role == EvidenceRole::SunoTermsRights)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        "NO"
    } else if terms.iter().any(|item| technically_verified(item)) {
        "YES"
    } else {
        "NOT VERIFIED"
    }
}

pub(super) fn evidence_list(evidence: &[EvidenceItem]) -> String {
    if evidence.is_empty() {
        return "- No evidence files registered\n".into();
    }
    let mut values = evidence.to_vec();
    values.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    values
        .iter()
        .map(|item| {
            format!(
                "- Evidence ID `{}` — path `{}` — role `{}` — original `{}` — {} bytes — SHA-256 `{}` — imported `{}` — provenance `{}` — document `{}` — provider `{}` — source URL `{}` — retrieval `{}` — effective `{}` — applicable production period `{}` — note `{}` — source global `{}` — derived from `{}` — generator `{}` — generated disclosure `{}`\n",
                item.id,
                item.relative_path,
                item.role.as_str(),
                value_or_missing(&item.metadata.original_file_name),
                item.size_bytes,
                item.sha256.as_deref().unwrap_or("NOT DOCUMENTED"),
                item.imported_at,
                item.provenance.as_str(),
                value_or_missing(&item.metadata.document_title),
                value_or_missing(&item.metadata.provider),
                value_or_missing(&item.metadata.source_url),
                value_or_missing(&item.metadata.retrieval_date),
                value_or_missing(&item.metadata.effective_date),
                value_or_missing(&item.metadata.applicable_production_period),
                value_or_missing(&item.metadata.factual_note),
                item.source_global_evidence_id.as_deref().unwrap_or("N/A"),
                item.derived_from_evidence_id.as_deref().unwrap_or("N/A"),
                item.generator_version.as_deref().unwrap_or("N/A"),
                item.generated_disclosure_text.as_deref().unwrap_or("N/A"),
            )
        })
        .collect()
}
