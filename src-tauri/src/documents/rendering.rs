mod artwork;
mod context;
mod pages;
mod sections;

use self::context::RenderContext;
use super::audio_screening::audio_screening_documentation_summary;
use super::marker;
use super::presentation::{render_suno_field_document, value_or_missing};
use crate::model::{EvidenceItem, Profile, StepState, TrackRecord};
use std::collections::BTreeMap;

pub(super) fn render(
    track: &TrackRecord,
    profile: &Profile,
    evidence: &[EvidenceItem],
    _steps: &[StepState],
) -> BTreeMap<String, String> {
    let context = RenderContext::new(track, profile, evidence);
    let source_declarations = sections::source_declarations(&context);
    let confirmed_work = sections::confirmed_work(&context);
    let audio_screening_summary =
        audio_screening_documentation_summary(&context.track.audio_screening);
    let timestamp_recommendation = sections::timestamp_recommendation(&context);
    let ai_artwork_usage = sections::ai_artwork_usage(&context);
    let lyrics_document = render_suno_field_document(&context.fields);
    let style_document = format!(
        "{}# Suno style prompt\n\n{}\n",
        marker(),
        value_or_missing(&context.fields.suno_style_prompt)
    );
    let image_generation_document = artwork::image_generation_document(&context);
    let artwork_document = artwork::artwork_document(&context);

    let mut values = BTreeMap::new();
    values.insert(
        "02_SUNO/suno_project.txt".into(),
        pages::suno_project(&context),
    );
    values.insert(
        "03_DOCUMENTATION/README.md".into(),
        pages::readme(
            &context,
            &source_declarations,
            &confirmed_work,
            &audio_screening_summary,
            timestamp_recommendation,
        ),
    );
    values.insert(
        "03_DOCUMENTATION/AI_USAGE.md".into(),
        pages::ai_usage(&context, &ai_artwork_usage),
    );
    values.insert("02_SUNO/Lyrics.md".into(), lyrics_document);
    values.insert("02_SUNO/Style.md".into(), style_document);
    values.insert(
        "04_LICENSES/suno_account_and_license.md".into(),
        pages::suno_account_and_license(&context),
    );
    values.insert(
        "04_LICENSES/openai_image_generation.md".into(),
        image_generation_document,
    );
    values.insert("05_ARTWORK/artwork_process.md".into(), artwork_document);
    values
}
