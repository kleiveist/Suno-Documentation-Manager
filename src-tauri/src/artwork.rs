use crate::error::{AppError, Result};
use crate::model::{EvidenceItem, EvidenceProvenance, EvidenceRole};
use crate::security::{atomic_write_new, contained_path, portable_relative, sha256_file, slugify};
use chrono::Utc;
use font8x8::UnicodeFonts;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const DISCLOSURE_GENERATOR_VERSION: &str = "local-disclosure-v1";

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
    let relative =
        PathBuf::from("05_ARTWORK").join(format!("{}_AI_EDITED.png", slugify(track_title)?));
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
    fn artwork_disclosure_preserves_original_and_creates_traceable_copy() {
        let directory = tempdir().expect("temporary directory");
        let track_root = directory.path().join("track");
        let artwork_directory = track_root.join("05_ARTWORK");
        fs::create_dir_all(&artwork_directory).expect("artwork directory");
        let original = artwork_directory.join("Test-Track_AI_ORIGINAL.png");
        RgbaImage::from_pixel(320, 320, Rgba([20, 40, 80, 255]))
            .save(&original)
            .expect("original fixture");
        let original_hash = sha256_file(&original).expect("original digest");
        let source = EvidenceItem {
            id: Uuid::new_v4().to_string(),
            role: EvidenceRole::AiArtworkOriginal,
            file_name: "Test-Track_AI_ORIGINAL.png".into(),
            relative_path: "05_ARTWORK/Test-Track_AI_ORIGINAL.png".into(),
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
        };

        let generated = generate_disclosure(&track_root, "Test Track", &source, "AI-assisted")
            .expect("local disclosure generation");
        assert_eq!(generated.role, EvidenceRole::AiArtworkEdited);
        assert_eq!(
            generated.relative_path,
            "05_ARTWORK/Test-Track_AI_EDITED.png"
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
}
