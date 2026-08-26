use super::evaluation::verified_role;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoverageStatus {
    Yes,
    No,
    NotVerified,
}

pub fn subscription_generation_coverage(
    track: &TrackRecord,
    evidence: &[EvidenceItem],
) -> CoverageStatus {
    let Some(generation_date) = parse_iso_day(&track.fields.suno_final_generation_date) else {
        return CoverageStatus::NotVerified;
    };
    let subscriptions = evidence
        .iter()
        .filter(|item| verified_role(item, EvidenceRole::SubscriptionPayment))
        .collect::<Vec<_>>();
    if subscriptions.is_empty()
        || subscriptions.iter().any(|item| {
            item.coverage_start.as_deref().is_none_or(str::is_empty)
                || item.coverage_end.as_deref().is_none_or(str::is_empty)
        })
    {
        return CoverageStatus::NotVerified;
    }
    let mut ranges = Vec::with_capacity(subscriptions.len());
    for item in subscriptions {
        let Some(start) = parse_iso_day(item.coverage_start.as_deref().unwrap_or_default()) else {
            return CoverageStatus::NotVerified;
        };
        let Some(end) = parse_iso_day(item.coverage_end.as_deref().unwrap_or_default()) else {
            return CoverageStatus::NotVerified;
        };
        if end < start {
            return CoverageStatus::NotVerified;
        }
        ranges.push((start, end));
    }
    if ranges
        .iter()
        .any(|(start, end)| *start <= generation_date && generation_date <= *end)
    {
        CoverageStatus::Yes
    } else {
        CoverageStatus::No
    }
}

pub fn subscription_production_coverage(
    track: &TrackRecord,
    evidence: &[EvidenceItem],
) -> CoverageStatus {
    let Some(production_start) = parse_iso_day(&track.fields.production_start_date) else {
        return CoverageStatus::NotVerified;
    };
    let Some(production_end) = parse_iso_day(&track.fields.production_end_date) else {
        return CoverageStatus::NotVerified;
    };
    if production_end < production_start {
        return CoverageStatus::NotVerified;
    }

    let subscriptions = evidence
        .iter()
        .filter(|item| {
            verified_role(item, EvidenceRole::SubscriptionPayment)
                && item.source_global_evidence_id.is_some()
        })
        .collect::<Vec<_>>();
    if subscriptions.is_empty()
        || subscriptions.iter().any(|item| {
            item.coverage_start.as_deref().is_none_or(str::is_empty)
                || item.coverage_end.as_deref().is_none_or(str::is_empty)
        })
    {
        return CoverageStatus::NotVerified;
    }

    let mut ranges = Vec::with_capacity(subscriptions.len());
    for item in subscriptions {
        let Some(start) = parse_iso_day(item.coverage_start.as_deref().unwrap_or_default()) else {
            return CoverageStatus::NotVerified;
        };
        let Some(end) = parse_iso_day(item.coverage_end.as_deref().unwrap_or_default()) else {
            return CoverageStatus::NotVerified;
        };
        if end < start {
            return CoverageStatus::NotVerified;
        }
        ranges.push((start, end));
    }
    ranges.sort_by_key(|range| range.0);

    let mut covered_until = production_start.pred_opt().unwrap_or(production_start);
    for (start, end) in ranges {
        if end < production_start || start > production_end {
            continue;
        }
        let next_uncovered = covered_until
            .checked_add_days(Days::new(1))
            .unwrap_or(covered_until);
        if start > next_uncovered {
            break;
        }
        if end > covered_until {
            covered_until = end;
        }
        if covered_until >= production_end {
            return CoverageStatus::Yes;
        }
    }
    CoverageStatus::No
}

fn parse_iso_day(value: &str) -> Option<NaiveDate> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return None;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}
