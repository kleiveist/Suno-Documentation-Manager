use super::*;

/// A bounded, CMYK render derivative of one verified artwork evidence item.
///
/// The bytes are never written beside the evidence and never replace its
/// registered path or SHA-256. They exist only long enough to serialize the PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateArtworkPreview {
    pub(super) role: EvidenceRole,
    pub(super) evidence_id: String,
    pub(super) file_name: String,
    pub(super) relative_path: String,
    pub(super) sha256: String,
    pub(super) width_pixels: u32,
    pub(super) height_pixels: u32,
    pub(super) cmyk_pixels: Arc<Vec<u8>>,
    pub(super) resource_id: String,
}

impl CertificateArtworkPreview {
    pub(super) fn xobject_id(&self) -> XObjectId {
        XObjectId(self.resource_id.clone())
    }
}

/// Build bounded artwork render assets from verified evidence without writing
/// to the track tree. A missing, undecodable, path-mismatched, or hash-mismatched
/// optional image is skipped so certificate generation retains its textual
/// fallback and never weakens the registered evidence identity.
pub fn prepare_artwork_previews(
    track_root: &Path,
    evidence: &[&EvidenceItem],
) -> Vec<CertificateArtworkPreview> {
    const ARTWORK_ROLES: [EvidenceRole; 6] = [
        // High-resolution summary roles come first so byte-identical process
        // evidence reuses the 640 px derivative, never the 384 px derivative.
        EvidenceRole::FinalArtwork,
        EvidenceRole::ReleaseArtwork,
        EvidenceRole::ArtworkSunoOriginal,
        EvidenceRole::AiArtworkOriginal,
        EvidenceRole::AiArtworkEdited,
        EvidenceRole::HumanEditedArtwork,
    ];

    let mut previews: Vec<CertificateArtworkPreview> = Vec::new();
    for role in ARTWORK_ROLES {
        let Some(item) = evidence.iter().copied().find(|item| item.role == role) else {
            continue;
        };
        let Some(expected_sha256) = item.sha256.as_deref() else {
            continue;
        };
        let Some(source_path) = resolve_preview_source(track_root, item, expected_sha256) else {
            continue;
        };
        let Ok(source_metadata) = std::fs::symlink_metadata(&source_path) else {
            continue;
        };
        if !source_metadata.file_type().is_file()
            || source_metadata.file_type().is_symlink()
            || source_metadata.len() != item.size_bytes
            || source_metadata.len() > ARTWORK_SOURCE_MAX_FILE_BYTES
        {
            continue;
        }
        let Ok(source_bytes) = std::fs::read(&source_path) else {
            continue;
        };
        if source_bytes.len() as u64 != item.size_bytes
            || !crate::security::sha256_bytes(&source_bytes).eq_ignore_ascii_case(expected_sha256)
        {
            continue;
        }
        if let Some(existing) = previews
            .iter()
            .find(|preview| preview.sha256.eq_ignore_ascii_case(expected_sha256))
            .cloned()
        {
            previews.push(CertificateArtworkPreview {
                role,
                evidence_id: item.id.clone(),
                file_name: item.file_name.clone(),
                relative_path: item.relative_path.clone(),
                sha256: expected_sha256.to_ascii_lowercase(),
                width_pixels: existing.width_pixels,
                height_pixels: existing.height_pixels,
                cmyk_pixels: existing.cmyk_pixels,
                resource_id: existing.resource_id,
            });
            continue;
        }
        let Ok(mut reader) =
            image::ImageReader::new(Cursor::new(source_bytes)).with_guessed_format()
        else {
            continue;
        };
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(ARTWORK_SOURCE_MAX_DIMENSION);
        limits.max_image_height = Some(ARTWORK_SOURCE_MAX_DIMENSION);
        limits.max_alloc = Some(ARTWORK_SOURCE_MAX_ALLOCATION);
        reader.limits(limits);
        let Ok(decoded) = reader.decode() else {
            continue;
        };
        let maximum_pixels = if matches!(
            role,
            EvidenceRole::FinalArtwork | EvidenceRole::ReleaseArtwork
        ) {
            ARTWORK_PREVIEW_MAX_PIXELS
        } else {
            ARTWORK_PROCESS_PREVIEW_MAX_PIXELS
        };
        let thumbnail = decoded.resize(maximum_pixels, maximum_pixels, FilterType::Lanczos3);
        let rgba = thumbnail.to_rgba8();
        let (width_pixels, height_pixels) = rgba.dimensions();
        if width_pixels == 0 || height_pixels == 0 {
            continue;
        }
        let cmyk_pixels = rgba_to_cmyk_on_white(rgba.as_raw());
        previews.push(CertificateArtworkPreview {
            role,
            evidence_id: item.id.clone(),
            file_name: item.file_name.clone(),
            relative_path: item.relative_path.clone(),
            sha256: expected_sha256.to_ascii_lowercase(),
            width_pixels,
            height_pixels,
            cmyk_pixels: Arc::new(cmyk_pixels),
            resource_id: format!("SunoDMArtwork{:02}", previews.len() + 1),
        });
    }
    previews
}

pub(super) fn resolve_preview_source(
    track_root: &Path,
    item: &EvidenceItem,
    expected_sha256: &str,
) -> Option<PathBuf> {
    let relative = Path::new(&item.relative_path);
    if let Ok(path) = crate::security::contained_path(track_root, relative, true) {
        return Some(path);
    }

    // Historical case-only path inconsistencies are not corrected in-place.
    // A preview may use a unique byte-identical sibling solely as a render
    // source; the displayed identity remains the immutable registered path.
    let parent = relative.parent()?;
    let directory = crate::security::contained_path(track_root, parent, true).ok()?;
    let expected_extension = relative
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let mut matches = std::fs::read_dir(directory)
        .ok()?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() || file_type.is_symlink() {
                return None;
            }
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            if metadata.len() != item.size_bytes
                || metadata.len() > ARTWORK_SOURCE_MAX_FILE_BYTES
                || !extension.eq_ignore_ascii_case(expected_extension)
            {
                return None;
            }
            crate::security::sha256_file(&path)
                .ok()
                .filter(|digest| digest.eq_ignore_ascii_case(expected_sha256))
                .map(|_| path)
        });
    let unique = matches.next()?;
    matches.next().is_none().then_some(unique)
}

pub(super) fn rgba_to_cmyk_on_white(rgba: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        let alpha = u32::from(pixel[3]);
        let red = ((u32::from(pixel[0]) * alpha + 255 * (255 - alpha)) / 255) as u8;
        let green = ((u32::from(pixel[1]) * alpha + 255 * (255 - alpha)) / 255) as u8;
        let blue = ((u32::from(pixel[2]) * alpha + 255 * (255 - alpha)) / 255) as u8;
        let black = 255_u8.saturating_sub(red.max(green).max(blue));
        if black == 255 {
            output.extend_from_slice(&[0, 0, 0, 255]);
            continue;
        }
        let denominator = u32::from(255 - black);
        let cyan = ((u32::from(255 - red - black) * 255) / denominator) as u8;
        let magenta = ((u32::from(255 - green - black) * 255) / denominator) as u8;
        let yellow = ((u32::from(255 - blue - black) * 255) / denominator) as u8;
        output.extend_from_slice(&[cyan, magenta, yellow, black]);
    }
    output
}
