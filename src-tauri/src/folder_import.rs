use crate::audio_metadata;
use crate::error::{AppError, Result};
use crate::evidence;
use crate::model::{EvidenceRole, TrackLibraryPlacement};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const TEXT_IMPORT_LIMIT_BYTES: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FolderImportKind {
    Single,
    Album,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportExecutionInput {
    pub source_path: String,
    pub expected_kind: FolderImportKind,
    #[serde(default)]
    pub single_track_title: Option<String>,
    #[serde(default)]
    pub single_track_library: Option<TrackLibraryPlacement>,
    #[serde(default)]
    pub production_start_date: String,
    #[serde(default)]
    pub commercial_use_intended: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportProposal {
    pub source_path: String,
    pub kind: FolderImportKind,
    pub album_title: Option<String>,
    pub tracks: Vec<FolderImportTrackProposal>,
    pub unassigned_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportTrackProposal {
    pub title: String,
    pub source_path: String,
    pub files: Vec<FolderImportFile>,
    pub ambiguities: Vec<String>,
    pub unassigned_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportFile {
    pub file_name: String,
    pub roles: Vec<String>,
    pub selected: bool,
}

#[derive(Debug, Clone)]
struct ClassifiedFile {
    source: PathBuf,
    roles: Vec<EvidenceRole>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportAssignment {
    pub role: EvidenceRole,
    pub source: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportTrackPlan {
    pub title: String,
    source: PathBuf,
    pub assignments: Vec<ImportAssignment>,
    pub lyrics: Option<String>,
    pub style: Option<String>,
    pub has_source_code: bool,
    files: Vec<ClassifiedFile>,
    has_track_media: bool,
}

pub(crate) fn plans(root: &Path) -> Result<(FolderImportProposal, Vec<ImportTrackPlan>)> {
    let root = source_directory(root)?;
    let child_plans = read_directories(&root)?
        .iter()
        .map(|directory| track_plan(directory))
        .collect::<Result<Vec<_>>>()?;

    // Only direct children with validated audio/video count as album tracks.
    // Documentation or artwork subfolders alone must not turn an arbitrary
    // folder hierarchy into an album import.
    let album_tracks = child_plans
        .into_iter()
        .filter(|plan| plan.has_track_media)
        .collect::<Vec<_>>();
    if album_tracks.len() >= 2 {
        let proposal_tracks = album_tracks.iter().map(proposal_for).collect();
        let unassigned_files = direct_regular_files(&root)?
            .into_iter()
            .filter_map(|path| file_name(&path))
            .collect();
        return Ok((
            FolderImportProposal {
                source_path: root.display().to_string(),
                kind: FolderImportKind::Album,
                album_title: file_name(&root),
                tracks: proposal_tracks,
                unassigned_files,
            },
            album_tracks,
        ));
    }

    let plan = track_plan(&root)?;
    let track_proposal = proposal_for(&plan);
    let proposal = FolderImportProposal {
        source_path: root.display().to_string(),
        kind: FolderImportKind::Single,
        album_title: None,
        unassigned_files: Vec::new(),
        tracks: vec![track_proposal],
    };
    Ok((proposal, vec![plan]))
}

fn source_directory(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AppError::Validation(
            "Der Importpfad muss ein normaler Ordner sein.".into(),
        ));
    }
    path.canonicalize()
        .map_err(|error| AppError::io(path, error))
}

fn read_directories(root: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| AppError::io(root, error))? {
        let entry = entry.map_err(|error| AppError::io(root, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::io(entry.path(), error))?;
        if file_type.is_dir() && !file_type.is_symlink() {
            directories.push(entry.path());
        }
    }
    directories.sort();
    Ok(directories)
}

fn direct_regular_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| AppError::io(root, error))? {
        let entry = entry.map_err(|error| AppError::io(root, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| AppError::io(entry.path(), error))?;
        if file_type.is_file() && !file_type.is_symlink() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

fn track_plan(root: &Path) -> Result<ImportTrackPlan> {
    let files = direct_regular_files(root)?
        .into_iter()
        .map(classify_file)
        .collect::<Vec<_>>();
    let has_track_media = files.iter().any(|file| {
        file.roles.iter().any(|role| {
            matches!(
                role,
                EvidenceRole::ReleaseMp3 | EvidenceRole::ReleaseMp4 | EvidenceRole::ReleaseWav
            ) && evidence::validate_type(role, &file.source).is_ok()
        })
    });
    let assignments = unique_assignments(&files);
    let lyrics = assigned_text(&assignments, EvidenceRole::Lyrics)?;
    let style = assigned_text(&assignments, EvidenceRole::Style)?;
    let has_source_code = assignments
        .iter()
        .any(|assignment| assignment.role == EvidenceRole::SourceCodeFile);

    Ok(ImportTrackPlan {
        title: file_name(root).unwrap_or_else(|| "Unbenannter Track".into()),
        source: root.to_path_buf(),
        assignments,
        lyrics,
        style,
        has_source_code,
        files,
        has_track_media,
    })
}

fn classify_file(source: PathBuf) -> ClassifiedFile {
    let name = file_stem(&source);
    let extension = extension(&source);
    let mut roles = Vec::new();

    if extension == "mp3" {
        roles.push(EvidenceRole::ReleaseMp3);
    }
    if matches!(extension.as_str(), "mp4" | "m4v") {
        roles.push(EvidenceRole::ReleaseMp4);
    }
    if extension == "wav" {
        roles.push(EvidenceRole::ReleaseWav);
        if audio_metadata::inspect_wav(&source)
            .ok()
            .flatten()
            .is_some_and(|metadata| metadata.suno_studio_detected)
        {
            roles.push(EvidenceRole::SunoFinalExport);
        }
    }
    if extension == "zip" && fuzzy_token(&name, "stems") {
        roles.push(EvidenceRole::SunoProjectZip);
    }

    let screenshot =
        is_screenshot(&name) && matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp");
    if screenshot {
        roles.push(EvidenceRole::SunoScreenshot);
    } else if is_image(&extension) {
        // Specific AI roles must win before the general human-edit marker.
        if has_ai_edited(&name) {
            roles.push(EvidenceRole::AiArtworkEdited);
        } else if has_ai_original(&name) {
            roles.push(EvidenceRole::AiArtworkOriginal);
        } else if has_human_edit(&name) {
            roles.push(EvidenceRole::HumanEditedArtwork);
        } else if matches!(extension.as_str(), "jpg" | "jpeg") {
            roles.push(EvidenceRole::ArtworkSunoOriginal);
        }
    }

    if matches!(extension.as_str(), "rb" | "py" | "js" | "ts") {
        roles.push(EvidenceRole::SourceCodeFile);
    }
    if matches!(extension.as_str(), "txt" | "md") {
        let lyrics_name = fuzzy_any(&name, &["lyrics", "lyric", "songtext"]);
        let style_name = fuzzy_any(&name, &["style", "stil", "prompt", "sunostyle"]);
        match (lyrics_name, style_name) {
            (true, false) => roles.push(EvidenceRole::Lyrics),
            (false, true) => roles.push(EvidenceRole::Style),
            (false, false) => {
                if let Some(role) = text_content_role(&source) {
                    roles.push(role);
                }
            }
            // A filename that claims both roles is not unambiguous.
            (true, true) => {}
        }
    }

    ClassifiedFile { source, roles }
}

fn unique_assignments(classified: &[ClassifiedFile]) -> Vec<ImportAssignment> {
    let roles = [
        EvidenceRole::ReleaseMp3,
        EvidenceRole::ReleaseMp4,
        EvidenceRole::ReleaseWav,
        EvidenceRole::SunoFinalExport,
        EvidenceRole::SunoProjectZip,
        EvidenceRole::SunoScreenshot,
        EvidenceRole::ArtworkSunoOriginal,
        EvidenceRole::AiArtworkOriginal,
        EvidenceRole::AiArtworkEdited,
        EvidenceRole::HumanEditedArtwork,
        EvidenceRole::SourceCodeFile,
        EvidenceRole::Lyrics,
        EvidenceRole::Style,
    ];
    let wav_count = classified
        .iter()
        .filter(|file| file.roles.contains(&EvidenceRole::ReleaseWav))
        .count();
    roles
        .into_iter()
        .filter_map(|role| {
            if matches!(
                role,
                EvidenceRole::ReleaseWav | EvidenceRole::SunoFinalExport
            ) && wav_count != 1
            {
                return None;
            }
            let matches = classified
                .iter()
                .filter(|file| file.roles.contains(&role))
                .collect::<Vec<_>>();
            (matches.len() == 1 && evidence::validate_type(&role, &matches[0].source).is_ok()).then(
                || ImportAssignment {
                    role,
                    source: matches[0].source.clone(),
                },
            )
        })
        .collect()
}

fn assigned_text(assignments: &[ImportAssignment], role: EvidenceRole) -> Result<Option<String>> {
    let Some(source) = assignments
        .iter()
        .find(|assignment| assignment.role == role)
        .map(|assignment| &assignment.source)
    else {
        return Ok(None);
    };
    let metadata = fs::metadata(source).map_err(|error| AppError::io(source, error))?;
    if metadata.len() > TEXT_IMPORT_LIMIT_BYTES {
        return Ok(None);
    }
    Ok(fs::read_to_string(source)
        .ok()
        .filter(|text| !text.trim().is_empty()))
}

fn proposal_for(plan: &ImportTrackPlan) -> FolderImportTrackProposal {
    let mut selected = BTreeMap::<PathBuf, Vec<String>>::new();
    for assignment in &plan.assignments {
        selected
            .entry(assignment.source.clone())
            .or_default()
            .push(assignment.role.as_str().to_owned());
    }

    let files = plan
        .files
        .iter()
        .map(|file| {
            let mut roles = file
                .roles
                .iter()
                .map(|role| role.as_str().to_owned())
                .collect::<Vec<_>>();
            roles.sort();
            FolderImportFile {
                file_name: file_name(&file.source).unwrap_or_default(),
                roles,
                selected: selected.contains_key(&file.source),
            }
        })
        .collect::<Vec<_>>();

    let candidate_count = |role: EvidenceRole| {
        plan.files
            .iter()
            .filter(|file| file.roles.contains(&role))
            .count()
    };
    let mut ambiguities = Vec::new();
    for role in [
        EvidenceRole::ReleaseMp3,
        EvidenceRole::ReleaseMp4,
        EvidenceRole::ReleaseWav,
        EvidenceRole::SunoProjectZip,
        EvidenceRole::ArtworkSunoOriginal,
        EvidenceRole::AiArtworkOriginal,
        EvidenceRole::AiArtworkEdited,
        EvidenceRole::HumanEditedArtwork,
        EvidenceRole::SunoScreenshot,
        EvidenceRole::SourceCodeFile,
        EvidenceRole::Lyrics,
        EvidenceRole::Style,
    ] {
        let count = candidate_count(role);
        if count > 1 {
            ambiguities.push(format!("{}: {count} Kandidaten", role.as_str()));
        }
    }
    let wav_count = candidate_count(EvidenceRole::ReleaseWav);
    if wav_count > 1 && candidate_count(EvidenceRole::SunoFinalExport) > 0 {
        ambiguities.push(format!("suno_final_export: {wav_count} WAV-Kandidaten"));
    }

    let unassigned_files = files
        .iter()
        .filter(|file| !file.selected)
        .map(|file| file.file_name.clone())
        .collect();
    FolderImportTrackProposal {
        title: plan.title.clone(),
        source_path: plan.source.display().to_string(),
        files,
        ambiguities,
        unassigned_files,
    }
}

fn text_content_role(path: &Path) -> Option<EvidenceRole> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > TEXT_IMPORT_LIMIT_BYTES {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let first = lines.first()?.trim_start_matches('#').trim().to_lowercase();
    if ["lyrics", "lyric", "songtext"]
        .iter()
        .any(|label| first == *label || first.starts_with(&format!("{label}:")))
        || (lines.len() >= 2
            && lines.iter().any(|line| {
                let line = line.to_ascii_lowercase();
                ["[verse", "[chorus", "[bridge", "[intro", "[outro"]
                    .iter()
                    .any(|marker| line.starts_with(marker))
            }))
    {
        return Some(EvidenceRole::Lyrics);
    }
    if ["style", "stil", "prompt", "suno style", "suno-style"]
        .iter()
        .any(|label| first == *label || first.starts_with(&format!("{label}:")))
    {
        return Some(EvidenceRole::Style);
    }
    None
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_owned)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn is_image(extension: &str) -> bool {
    matches!(extension, "png" | "jpg" | "jpeg")
}

fn fuzzy_token(name: &str, wanted: &str) -> bool {
    tokens(name)
        .iter()
        .any(|token| token == wanted || levenshtein(token, wanted) <= 1)
}

fn fuzzy_any(name: &str, wanted: &[&str]) -> bool {
    wanted.iter().any(|word| fuzzy_token(name, word))
}

fn tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn has_ai_marker(name: &str) -> bool {
    tokens(name).iter().any(|token| token == "ai")
}

fn has_ai_edited(name: &str) -> bool {
    has_ai_marker(name) && fuzzy_any(name, &["edited", "edit"])
}

fn has_ai_original(name: &str) -> bool {
    has_ai_marker(name) && fuzzy_token(name, "original")
}

fn has_human_edit(name: &str) -> bool {
    !has_ai_marker(name) && fuzzy_any(name, &["edited", "edit"])
}

fn is_screenshot(name: &str) -> bool {
    fuzzy_any(name, &["screenshot", "bildschirmfoto"])
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut previous = (0..=right.chars().count()).collect::<Vec<_>>();
    for (i, left_char) in left.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, right_char) in right.chars().enumerate() {
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + usize::from(left_char != right_char)),
            );
        }
        previous = current;
    }
    previous[right.chars().count()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, name: &str, bytes: &[u8]) {
        fs::write(root.join(name), bytes).expect("fixture file");
    }

    fn png() -> &'static [u8] {
        b"\x89PNG\r\n\x1a\nfixture"
    }

    fn jpeg() -> &'static [u8] {
        b"\xff\xd8\xfffixture"
    }

    fn wav() -> &'static [u8] {
        b"RIFF\x04\x00\x00\x00WAVE"
    }

    fn roles(plan: &ImportTrackPlan) -> Vec<EvidenceRole> {
        plan.assignments.iter().map(|item| item.role).collect()
    }

    #[test]
    fn classifies_required_artwork_variants_case_insensitively() {
        let directory = tempdir().expect("directory");
        write(directory.path(), "Track_AI_ORGINAL.PNG", png());
        write(directory.path(), "Track-ai-editd.png", png());
        write(directory.path(), "Track_HUMAN_EDIT.png", png());
        write(directory.path(), "Track.JpEg", jpeg());

        let (_, plans) = plans(directory.path()).expect("scan");
        let roles = roles(&plans[0]);
        assert!(roles.contains(&EvidenceRole::AiArtworkOriginal));
        assert!(roles.contains(&EvidenceRole::AiArtworkEdited));
        assert!(roles.contains(&EvidenceRole::HumanEditedArtwork));
        assert!(roles.contains(&EvidenceRole::ArtworkSunoOriginal));
    }

    #[test]
    fn ai_edited_always_wins_over_the_general_edited_marker() {
        let directory = tempdir().expect("directory");
        write(directory.path(), "Awakening_AI_EDITED.png", png());

        let (_, plans) = plans(directory.path()).expect("scan");
        let roles = roles(&plans[0]);
        assert!(roles.contains(&EvidenceRole::AiArtworkEdited));
        assert!(!roles.contains(&EvidenceRole::HumanEditedArtwork));
    }

    #[test]
    fn fuzzy_keywords_do_not_match_unrelated_substrings() {
        let directory = tempdir().expect("directory");
        write(directory.path(), "Credit.jpeg", jpeg());

        let (_, plans) = plans(directory.path()).expect("scan");
        let roles = roles(&plans[0]);
        assert!(roles.contains(&EvidenceRole::ArtworkSunoOriginal));
        assert!(!roles.contains(&EvidenceRole::HumanEditedArtwork));
    }

    #[test]
    fn screenshot_is_only_a_suno_project_hint_and_not_artwork() {
        let directory = tempdir().expect("directory");
        write(
            directory.path(),
            "Bildschirmfoto_20260817_141059.jpeg",
            jpeg(),
        );

        let (_, plans) = plans(directory.path()).expect("scan");
        assert_eq!(roles(&plans[0]), vec![EvidenceRole::SunoScreenshot]);
    }

    #[test]
    fn duplicate_candidates_for_every_singular_role_stay_unassigned() {
        let directory = tempdir().expect("directory");
        write(directory.path(), "Track.mp3", b"ID3first");
        write(directory.path(), "TrackV2.mp3", b"ID3second");
        write(directory.path(), "Track.wav", wav());
        write(directory.path(), "TrackV2.wav", wav());
        write(directory.path(), "Track_AI_EDITED.png", png());
        write(directory.path(), "TrackV2_AI_EDITED.png", png());
        write(directory.path(), "Screenshot_01.png", png());
        write(directory.path(), "Screenshot_02.png", png());
        write(directory.path(), "one.rb", b"play 60\n");
        write(directory.path(), "two.rb", b"play 61\n");

        let (proposal, plans) = plans(directory.path()).expect("scan");
        for role in [
            EvidenceRole::ReleaseMp3,
            EvidenceRole::ReleaseWav,
            EvidenceRole::AiArtworkEdited,
            EvidenceRole::SunoScreenshot,
            EvidenceRole::SourceCodeFile,
        ] {
            assert!(!roles(&plans[0]).contains(&role), "assigned {role:?}");
            assert!(proposal.tracks[0]
                .ambiguities
                .iter()
                .any(|item| item.contains(role.as_str())));
        }
        assert!(!plans[0].has_source_code);
    }

    #[test]
    fn imports_clear_named_and_content_identified_text_but_not_generic_prose() {
        let named = tempdir().expect("named directory");
        write(named.path(), "Lyrics.txt", b"First line\nSecond line\n");
        write(named.path(), "Prompt.md", b"dreamy synthwave\n");
        let (_, named_plans) = plans(named.path()).expect("named scan");
        assert!(roles(&named_plans[0]).contains(&EvidenceRole::Lyrics));
        assert!(roles(&named_plans[0]).contains(&EvidenceRole::Style));
        assert_eq!(
            named_plans[0].lyrics.as_deref(),
            Some("First line\nSecond line\n")
        );

        let generic = tempdir().expect("generic directory");
        write(
            generic.path(),
            "Textdatei.txt",
            b"Lyrics:\nA clear lyric line\n",
        );
        let (_, generic_plans) = plans(generic.path()).expect("generic scan");
        assert!(roles(&generic_plans[0]).contains(&EvidenceRole::Lyrics));

        let unknown = tempdir().expect("unknown directory");
        write(
            unknown.path(),
            "Textdatei.txt",
            b"A note without a reliable role.\n",
        );
        let (proposal, plans) = plans(unknown.path()).expect("unknown scan");
        assert!(plans[0].assignments.is_empty());
        assert_eq!(proposal.tracks[0].unassigned_files, vec!["Textdatei.txt"]);
    }

    #[test]
    fn finds_three_album_tracks_and_never_distributes_root_files() {
        let directory = tempdir().expect("directory");
        for title in ["Awakening", "Boot Sequence", "LastWarnung"] {
            let track = directory.path().join(title);
            fs::create_dir(&track).expect("track directory");
            write(&track, &format!("{title}.mp3"), b"ID3audio");
        }
        write(directory.path(), "Album_AI_EDITED.png", png());
        write(directory.path(), "signed_contract.pdf", b"%PDF-fixture");

        let (proposal, plans) = plans(directory.path()).expect("scan");
        assert_eq!(proposal.kind, FolderImportKind::Album);
        assert_eq!(plans.len(), 3);
        assert_eq!(
            proposal.unassigned_files,
            vec!["Album_AI_EDITED.png", "signed_contract.pdf"]
        );
    }

    #[test]
    fn non_media_subdirectories_do_not_create_a_false_album() {
        let directory = tempdir().expect("directory");
        for title in ["Artwork", "Notes"] {
            let child = directory.path().join(title);
            fs::create_dir(&child).expect("child directory");
            write(&child, "Lyrics.txt", b"lyrics\n");
        }
        write(directory.path(), "Track.mp3", b"ID3audio");

        let (proposal, _) = plans(directory.path()).expect("scan");
        assert_eq!(proposal.kind, FolderImportKind::Single);
    }

    #[test]
    fn source_folder_scan_is_read_only_and_source_code_is_unique() {
        let directory = tempdir().expect("directory");
        write(directory.path(), "SpaceWideToWide1.rb", b"play 60\n");
        let before = fs::read(directory.path().join("SpaceWideToWide1.rb")).expect("before");

        let (_, plans) = plans(directory.path()).expect("scan");

        assert!(plans[0].has_source_code);
        assert_eq!(
            fs::read(directory.path().join("SpaceWideToWide1.rb")).expect("after"),
            before
        );
    }
}
