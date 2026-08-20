use crate::error::{AppError, Result};
use crate::model::{EvidenceItem, EvidenceProvenance, EvidenceRole};
use crate::security::{
    atomic_write_new, canonical_artwork_stem, contained_path, portable_relative, sha256_file,
};
use chrono::Utc;
use font8x8::UnicodeFonts;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const DISCLOSURE_GENERATOR_VERSION: &str = "local-disclosure-v1";
pub const COVER_PREVIEW_SIZE: u32 = 192;
const COVER_PREVIEW_SOURCE_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
const COVER_PREVIEW_PIXEL_LIMIT: u64 = 100_000_000;

pub fn centered_cover_thumbnail(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|error| AppError::io(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::Validation(
            "Track cover preview requires a regular managed image.".into(),
        ));
    }
    if metadata.len() > COVER_PREVIEW_SOURCE_LIMIT_BYTES {
        return Err(AppError::Validation(
            "The final artwork is too large for the track cover preview.".into(),
        ));
    }
    let (width, height) =
        image::image_dimensions(path).map_err(|error| AppError::Image(error.to_string()))?;
    if width == 0
        || height == 0
        || u64::from(width).saturating_mul(u64::from(height)) > COVER_PREVIEW_PIXEL_LIMIT
    {
        return Err(AppError::Validation(
            "The final artwork dimensions are not safe for the track cover preview.".into(),
        ));
    }
    let image = image::open(path).map_err(|error| AppError::Image(error.to_string()))?;
    let thumbnail =
        image.resize_to_fill(COVER_PREVIEW_SIZE, COVER_PREVIEW_SIZE, FilterType::Lanczos3);
    let mut encoded = Vec::new();
    thumbnail
        .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
        .map_err(|error| AppError::Image(error.to_string()))?;
    Ok(encoded)
}

pub fn generate_disclosure(
    track_root: &Path,
    track_title: &str,
    source: &EvidenceItem,
    disclosure_text: &str,
) -> Result<EvidenceItem> {
    if source.role != EvidenceRole::AiArtworkOriginal {
        return Err(AppError::Validation(
            "Visible disclosure requires the verified AI artwork original.".into(),
        ));
    }
    let text = disclosure_text.trim();
    if text.is_empty() || text.chars().count() > 80 || text.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Disclosure text must contain 1 to 80 visible characters.".into(),
        ));
    }
    let source_path = contained_path(track_root, Path::new(&source.relative_path), true)?;
    if sha256_file(&source_path)? != source.sha256.clone().unwrap_or_default() {
        return Err(AppError::Validation(
            "The AI artwork source changed after import.".into(),
        ));
    }
    let image = image::open(&source_path).map_err(|e| AppError::Image(e.to_string()))?;
    let output = draw_label(image, text)?;
    let relative = PathBuf::from("05_ARTWORK").join(format!(
        "{}_AI_EDITED.png",
        canonical_artwork_stem(track_title)?
    ));
    let destination = contained_path(track_root, &relative, false)?;
    let mut bytes = Vec::new();
    output
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| AppError::Image(e.to_string()))?;
    atomic_write_new(&destination, &bytes)?;
    Ok(EvidenceItem {
        id: Uuid::new_v4().to_string(),
        // Applying the disclosure creates another AI-processing stage. It must not
        // silently satisfy the independently confirmed final-artwork requirement.
        role: EvidenceRole::AiArtworkEdited,
        file_name: destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("AI_EDITED.png")
            .to_owned(),
        relative_path: portable_relative(&relative),
        sha256: Some(sha256_file(&destination)?),
        size_bytes: fs::metadata(&destination)
            .map_err(|e| AppError::io(&destination, e))?
            .len(),
        imported_at: Utc::now().to_rfc3339(),
        verified: true,
        verification_error: None,
        source_global_evidence_id: None,
        coverage_start: None,
        coverage_end: None,
        provenance: EvidenceProvenance::GeneratedDisclosure,
        derived_from_evidence_id: Some(source.id.clone()),
        generator_version: Some(DISCLOSURE_GENERATOR_VERSION.into()),
        generated_disclosure_text: Some(text.to_owned()),
        metadata: Default::default(),
    })
}

fn draw_label(image: DynamicImage, text: &str) -> Result<RgbaImage> {
    let mut output = image.to_rgba8();
    let scale = (output.width().min(output.height()) / 256).clamp(1, 8);
    let glyph_width = 8 * scale;
    let glyph_height = 8 * scale;
    let padding = 5 * scale;
    let text_width = text.chars().count() as u32 * glyph_width;
    if text_width + padding * 2 > output.width() || glyph_height + padding * 2 > output.height() {
        return Err(AppError::Image(
            "Artwork is too small for the disclosure text.".into(),
        ));
    }
    let left = output.width() - text_width - padding * 2;
    let top = output.height() - glyph_height - padding * 2;
    for y in top..output.height() {
        for x in left..output.width() {
            let pixel = output.get_pixel_mut(x, y);
            blend(pixel, Rgba([0, 0, 0, 150]));
        }
    }
    for (index, character) in text.chars().enumerate() {
        let glyph = font8x8::BASIC_FONTS
            .get(character)
            .or_else(|| font8x8::LATIN_FONTS.get(character))
            .ok_or_else(|| {
                AppError::Image(format!(
                    "Disclosure character '{character}' is not supported by the local renderer."
                ))
            })?;
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8 {
                if bits & (1 << column) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x =
                                left + padding + index as u32 * glyph_width + column * scale + sx;
                            let y = top + padding + row as u32 * scale + sy;
                            output.put_pixel(x, y, Rgba([255, 255, 255, 235]));
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}

fn blend(target: &mut Rgba<u8>, source: Rgba<u8>) {
    let alpha = source[3] as u16;
    for channel in 0..3 {
        target[channel] = (((source[channel] as u16 * alpha)
            + (target[channel] as u16 * (255 - alpha)))
            / 255) as u8;
    }
    target[3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cover_thumbnail_is_square_and_center_cropped() {
        let directory = tempdir().expect("temporary directory");
        let source = directory.path().join("wide.png");
        let mut image = RgbaImage::from_pixel(600, 200, Rgba([20, 40, 80, 255]));
        for pixel in image.pixels_mut().skip(200 * 200).take(200 * 200) {
            *pixel = Rgba([180, 40, 60, 255]);
        }
        image.save(&source).expect("wide cover fixture");

        let encoded = centered_cover_thumbnail(&source).expect("cover thumbnail");
        let decoded = image::load_from_memory_with_format(&encoded, ImageFormat::Png)
            .expect("decode thumbnail");

        assert_eq!(decoded.width(), COVER_PREVIEW_SIZE);
        assert_eq!(decoded.height(), COVER_PREVIEW_SIZE);
    }

    #[test]
    fn artwork_disclosure_preserves_original_and_creates_traceable_copy() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        let artwork_directory = track_root.join("05_ARTWORK");
        fs::create_dir_all(&artwork_directory).expect("artwork directory");
        let original = artwork_directory.join("TEST_TRACK_AI_ORIGINAL.png");
        RgbaImage::from_pixel(320, 320, Rgba([20, 40, 80, 255]))
            .save(&original)
            .expect("original fixture");
        let original_hash = sha256_file(&original).expect("original digest");
        let source = EvidenceItem {
            id: Uuid::new_v4().to_string(),
            role: EvidenceRole::AiArtworkOriginal,
            file_name: "TEST_TRACK_AI_ORIGINAL.png".into(),
            relative_path: "05_ARTWORK/TEST_TRACK_AI_ORIGINAL.png".into(),
            sha256: Some(original_hash.clone()),
            size_bytes: fs::metadata(&original).expect("original metadata").len(),
            imported_at: Utc::now().to_rfc3339(),
            verified: true,
            verification_error: None,
            source_global_evidence_id: None,
            coverage_start: None,
            coverage_end: None,
            provenance: EvidenceProvenance::ManagedCopy,
            derived_from_evidence_id: None,
            generator_version: None,
            generated_disclosure_text: None,
            metadata: Default::default(),
        };

        let generated = generate_disclosure(&track_root, "Test Track", &source, "AI-assisted")
            .expect("local disclosure generation");
        assert_eq!(generated.role, EvidenceRole::AiArtworkEdited);
        assert_eq!(
            generated.relative_path,
            "05_ARTWORK/TEST_TRACK_AI_EDITED.png"
        );
        assert_eq!(
            sha256_file(&original).expect("original digest after generation"),
            original_hash
        );
        assert!(track_root.join(&generated.relative_path).is_file());
        assert_ne!(generated.sha256, source.sha256);

        let collision = generate_disclosure(&track_root, "Test Track", &source, "AI-assisted")
            .expect_err("existing disclosed copy must not be overwritten");
        assert!(matches!(collision, AppError::Collision(_)));
    }

    #[test]
    fn disclosure_renderer_preserves_supported_latin_text_and_rejects_unknown_glyphs() {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(640, 640, Rgba([20, 40, 80, 255])));
        draw_label(image.clone(), "KI-unterstützt").expect("Latin-1 disclosure");
        assert!(matches!(
            draw_label(image, "AI-assisted 🚀"),
            Err(AppError::Image(_))
        ));
    }

    #[test]
    fn disclosure_renderer_is_deterministic_and_bottom_right_only() {
        let source = RgbaImage::from_pixel(640, 640, Rgba([20, 40, 80, 255]));
        let first = draw_label(
            DynamicImage::ImageRgba8(source.clone()),
            "Custom disclosure",
        )
        .expect("first disclosure render");
        let second = draw_label(
            DynamicImage::ImageRgba8(source.clone()),
            "Custom disclosure",
        )
        .expect("second disclosure render");
        assert_eq!(first.as_raw(), second.as_raw(), "rendered pixels differ");

        let changed = first
            .enumerate_pixels()
            .filter(|(x, y, pixel)| pixel != &source.get_pixel(*x, *y))
            .map(|(x, y, _)| (x, y))
            .collect::<Vec<_>>();
        assert!(!changed.is_empty(), "disclosure did not change any pixel");
        assert!(changed.iter().all(|(x, y)| *x >= 348 && *y >= 604));
        assert!(changed.iter().any(|(x, y)| *x > 620 && *y > 620));
        assert_eq!(first.get_pixel(0, 0), source.get_pixel(0, 0));
    }
}
