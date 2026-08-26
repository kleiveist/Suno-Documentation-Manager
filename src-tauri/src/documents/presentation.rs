use super::marker;
use crate::model::{
    DocumentationAnswer, SunoContentClassification, SunoLyricsContentSource, TrackFields,
};

pub(super) fn yes_no(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "YES",
        Some(false) => "NO",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn deliberate_non_application(value: Option<bool>) -> &'static str {
    match value {
        Some(false) => "YES",
        Some(true) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn value_or_missing(value: &str) -> &str {
    if value.trim().is_empty() {
        "NOT DOCUMENTED"
    } else {
        value
    }
}

pub(super) fn documented_list(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        values.join(", ")
    }
}

pub(super) fn documentation_answer(value: Option<DocumentationAnswer>) -> &'static str {
    match value {
        Some(DocumentationAnswer::Yes) => "YES",
        Some(DocumentationAnswer::No) => "NO",
        Some(DocumentationAnswer::NotDocumented) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn conditional_documentation_answer(
    controlling: Option<bool>,
    value: Option<DocumentationAnswer>,
) -> &'static str {
    match controlling {
        Some(true) => documentation_answer(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn conditional_value(controlling: Option<bool>, value: &str) -> &str {
    match controlling {
        Some(true) => value_or_missing(value),
        Some(false) => "N/A",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn content_classification(fields: &TrackFields) -> &'static str {
    fields
        .suno_content_classification
        .map(SunoContentClassification::as_str)
        .unwrap_or("NOT DOCUMENTED")
}

pub(super) fn vocal_intent(fields: &TrackFields) -> &'static str {
    fields
        .vocal_intent
        .map(crate::model::VocalIntent::as_str)
        .unwrap_or("NOT DOCUMENTED")
}

pub(super) fn generation_text_field_used(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => "NO",
        Some(_) => "YES",
        None => "NOT DOCUMENTED",
    }
}

pub(super) fn generation_field_vocal_lyrics_present(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::VocalLyricsOnly | SunoContentClassification::Mixed) => {
            "YES"
        }
        Some(SunoContentClassification::StructureOnly) => "NO",
        Some(SunoContentClassification::Empty) => "N/A",
        Some(SunoContentClassification::Other) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn structure_instructions_present(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::StructureOnly | SunoContentClassification::Mixed) => "YES",
        Some(SunoContentClassification::VocalLyricsOnly) => "NO",
        Some(SunoContentClassification::Empty) => "N/A",
        Some(SunoContentClassification::Other) | None => "NOT DOCUMENTED",
    }
}

pub(super) fn suno_content_source(fields: &TrackFields) -> &'static str {
    match fields.suno_content_classification {
        Some(SunoContentClassification::Empty) => "N/A",
        None => "NOT DOCUMENTED",
        Some(_) => match fields.suno_lyrics_content_source {
            Some(SunoLyricsContentSource::Human) => "human",
            Some(SunoLyricsContentSource::Ai) => "AI",
            Some(SunoLyricsContentSource::Mixed) => "mixed",
            None => "NOT DOCUMENTED",
        },
    }
}

pub(super) fn render_suno_field_document(fields: &TrackFields) -> String {
    let mut output = format!(
        "{}# Suno Generation Text Field\n\n- Suno Instrumental Mode Selected [User-confirmed fact]: {}\n- Content Classification [User-confirmed fact]: {}\n- Vocal Intent [User-confirmed fact]: {}\n- Final Audio Contains Vocals [User-confirmed fact]: {}\n- Generation Text Field Used [User-confirmed fact]: {}\n- Vocal Lyrics Present [Classification-derived presentation]: {}\n- Structure Instructions Present [Classification-derived presentation]: {}\n- Content Source [User-confirmed fact]: {}\n- Other Content Label [User-confirmed fact]: {}\n\n## Exact Generation Text Field Content\n\n{}\n",
        marker(),
        yes_no(fields.instrumental_track),
        content_classification(fields),
        vocal_intent(fields),
        yes_no(fields.vocal_lyrics_present),
        generation_text_field_used(fields),
        generation_field_vocal_lyrics_present(fields),
        structure_instructions_present(fields),
        suno_content_source(fields),
        if fields.suno_content_classification == Some(SunoContentClassification::Other) {
            value_or_missing(&fields.suno_lyrics_other_content_type)
        } else {
            "N/A"
        },
        match fields.suno_content_classification {
            Some(SunoContentClassification::Empty) => "N/A",
            Some(_) => value_or_missing(&fields.suno_lyrics_field_text),
            None => "NOT DOCUMENTED",
        },
    );
    if !fields.lyrics_source.trim().is_empty() || !fields.lyrics_text.trim().is_empty() {
        output
            .push_str("\n## Unclassified legacy lyrics data\n\n- Classification: NOT DOCUMENTED\n");
        output.push_str(&format!(
            "- Legacy source value: {}\n- Legacy text: {}\n\nThis retained historical data is unclassified legacy data and is not a Vocal Lyrics claim.\n",
            value_or_missing(&fields.lyrics_source),
            value_or_missing(&fields.lyrics_text),
        ));
    }
    output
}

pub(super) fn deepfake_indicator_summary(fields: &TrackFields) -> &'static str {
    if fields.generative_ai_used == Some(false) {
        return "N/A";
    }
    if fields.generative_ai_used != Some(true) {
        return "INCOMPLETE – generative AI use NOT DOCUMENTED";
    }
    let answers = [
        fields.real_person_voice_intentionally_imitated,
        fields.real_person_identity_intentionally_represented,
        fields.real_event_represented_as_authentic_recording,
        fields.real_location_institution_event_presented_as_authentic_ai_recording,
    ];
    if answers.contains(&Some(DocumentationAnswer::Yes)) {
        "Potential deepfake-related indicator recorded"
    } else if answers
        .iter()
        .all(|answer| *answer == Some(DocumentationAnswer::No))
    {
        "Documented deepfake-related indicators: none recorded"
    } else {
        "Deepfake-related indicator documentation: INCOMPLETE"
    }
}

pub(super) fn render_ai_audio(fields: &TrackFields) -> String {
    let active = fields.generative_ai_used;
    format!(
        "- Generative AI used [User-confirmed fact]: {}\n- AI system [User-confirmed fact]: {}\n- AI-assisted audio elements [User-confirmed fact]: {}\n- AI-generated audio elements [User-confirmed fact]: {}\n- Real person voice intentionally imitated [User-confirmed fact]: {}\n- Real person's identity intentionally represented [User-confirmed fact]: {}\n- Real event represented as authentic recording [User-confirmed fact]: {}\n- Real location / institution / event presented as authentic AI recording [User-confirmed fact]: {}\n- Disclosure applied [User-confirmed fact]: {}\n- Disclosure locations [User-confirmed fact]: {}\n- Disclosure text [User-confirmed fact]: {}\n- Disclosure reason / note [User-confirmed fact]: {}\n- Deepfake-related indicator summary: {}\n",
        yes_no(active),
        conditional_value(active, &fields.audio_ai_system),
        conditional_documentation_answer(active, fields.ai_assisted_audio_elements),
        conditional_documentation_answer(active, fields.ai_generated_audio_elements),
        conditional_documentation_answer(active, fields.real_person_voice_intentionally_imitated,),
        conditional_documentation_answer(
            active,
            fields.real_person_identity_intentionally_represented,
        ),
        conditional_documentation_answer(
            active,
            fields.real_event_represented_as_authentic_recording,
        ),
        conditional_documentation_answer(
            active,
            fields.real_location_institution_event_presented_as_authentic_ai_recording,
        ),
        conditional_documentation_answer(active, fields.audio_disclosure_applied),
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                documented_list(&fields.audio_disclosure_locations)
            }
            (Some(true), _) | (Some(false), _) => "N/A".into(),
            (None, _) => "NOT DOCUMENTED".into(),
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::Yes)) => {
                value_or_missing(&fields.audio_disclosure_text)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        match (active, fields.audio_disclosure_applied) {
            (Some(true), Some(DocumentationAnswer::No)) => {
                value_or_missing(&fields.audio_disclosure_reason)
            }
            (Some(true), _) | (Some(false), _) => "N/A",
            (None, _) => "NOT DOCUMENTED",
        },
        deepfake_indicator_summary(fields),
    )
}

pub(super) fn fact_origin_label(origin: crate::model::FactOrigin) -> &'static str {
    match origin {
        crate::model::FactOrigin::UserConfirmedFact => "User-confirmed fact",
        crate::model::FactOrigin::EvidenceDerivedMetadata => "Evidence-derived metadata",
        crate::model::FactOrigin::NotDocumented => "NOT DOCUMENTED",
    }
}

pub(super) const LEGACY_SELECTION_NOTICE: &str =
    "Legacy value retained in the track record; select a defined category in the app.";

pub(super) const SOURCE_CHOICES: &[(&str, &[&str])] = &[
    (
        "Audio from a licensed sample library",
        &["Lizenzierte Sample-Bibliothek"],
    ),
    (
        "Licensed beat or instrumental",
        &["Lizenzierter Beat oder Instrumentaltrack"],
    ),
    (
        "Audio supplied by a collaborator",
        &["Von Mitwirkenden bereitgestelltes Audio"],
    ),
    ("Commissioned recording", &["Beauftragte Aufnahme"]),
    ("Public-domain recording", &["Gemeinfreie Aufnahme"]),
    (
        "Creative Commons recording",
        &["Aufnahme unter Creative-Commons-Lizenz"],
    ),
    ("Original vocal recording", &["Eigene Gesangsaufnahme"]),
    (
        "Original instrument recording",
        &["Eigene Instrumentalaufnahme"],
    ),
    (
        "Original field recording",
        &["Eigene Feldaufnahme", "Eigene Aufnahme"],
    ),
    (
        "Original MIDI or software render",
        &["Eigener MIDI- oder Software-Render"],
    ),
    ("Original sound design", &["Eigenes Sounddesign"]),
    (
        "Commercial sample library",
        &["Kommerzielle Sample-Bibliothek"],
    ),
    ("Royalty-free sample pack", &["Royalty-free Sample-Pack"]),
    (
        "Directly licensed from the sample creator",
        &["Direkt vom Sample-Urheber lizenziert"],
    ),
    ("Public-domain archive", &["Gemeinfreies Archiv"]),
    ("Creative Commons source", &["Creative-Commons-Quelle"]),
];

pub(super) const RIGHTS_CHOICES: &[(&str, &[&str])] = &[
    (
        "Commercial-use license",
        &["Lizenz für kommerzielle Nutzung"],
    ),
    (
        "Direct permission from the rights holder",
        &["Direkte Erlaubnis des Rechteinhabers"],
    ),
    ("Joint rights agreement", &["Gemeinsame Rechtevereinbarung"]),
    ("Public domain", &["Gemeinfreiheit"]),
    ("Creative Commons license", &["Creative-Commons-Lizenz"]),
    (
        "Solely owned by the artist",
        &["Ausschließlich eigene Rechte", "Eigene Produktion"],
    ),
    (
        "Jointly owned with collaborators",
        &["Gemeinsame Rechte mit Mitwirkenden"],
    ),
    (
        "Participant permissions documented",
        &["Einwilligungen der Beteiligten dokumentiert"],
    ),
    ("Commercial sample license", &["Kommerzielle Sample-Lizenz"]),
    ("Royalty-free license", &["Royalty-free Lizenz"]),
];

pub(super) const HUMAN_WORK_CHOICES: &[(&str, &[&str])] = &[
    ("Arrangement", &[]),
    ("Lyrics", &[]),
    ("Timing and cuts", &["Timing und Cuts"]),
    ("Sound design", &["Sounddesign"]),
    ("EQ", &[]),
    ("Mixing", &[]),
    ("Mastering", &[]),
    ("Loudness adjustment", &["Lautheitsanpassung"]),
];

pub(super) const POST_EXPORT_CHOICES: &[(&str, &[&str])] = &[
    ("Editing and cuts", &["Schnitt"]),
    ("Arrangement", &[]),
    ("Timing correction", &["Timing-Korrektur"]),
    ("Sound design", &["Sounddesign"]),
    ("EQ", &[]),
    ("Mixing", &[]),
    ("Mastering", &[]),
    ("Loudness adjustment", &["Lautheitsanpassung"]),
    ("Noise reduction", &["Rauschreduzierung"]),
    ("Dynamics processing", &["Dynamikbearbeitung"]),
];

pub(super) const RELEASE_CHOICES: &[(&str, &[&str])] = &[
    ("Original Suno version", &["Originale Suno-Fassung"]),
    ("Streaming master", &["Streaming-Master"]),
    ("Radio edit", &["Radio Edit"]),
    ("Extended mix", &["Extended Mix"]),
    ("Instrumental version", &["Instrumental"]),
    ("Clean version", &["Clean Version"]),
    ("Explicit version", &["Explicit Version"]),
    ("Social-media version", &["Social-Media-Version"]),
];

pub(super) fn choice_key(value: &str) -> String {
    value.trim().to_lowercase()
}

pub(super) fn english_guided_value(value: &str, choices: &[(&str, &[&str])]) -> String {
    if value.trim().is_empty() {
        return "NOT DOCUMENTED".into();
    }
    let key = choice_key(value);
    choices
        .iter()
        .find(|(english, aliases)| {
            choice_key(english) == key || aliases.iter().any(|alias| choice_key(alias) == key)
        })
        .map(|(english, _)| (*english).to_owned())
        .unwrap_or_else(|| LEGACY_SELECTION_NOTICE.into())
}

pub(super) fn english_guided_list(value: &str, choices: &[(&str, &[&str])]) -> String {
    if value.trim().is_empty() {
        return "NOT DOCUMENTED".into();
    }
    let mut normalized = Vec::new();
    for item in value
        .split('|')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let mapped = english_guided_value(item, choices);
        if !normalized.contains(&mapped) {
            normalized.push(mapped);
        }
    }
    if normalized.is_empty() {
        "NOT DOCUMENTED".into()
    } else {
        normalized.join(", ")
    }
}
