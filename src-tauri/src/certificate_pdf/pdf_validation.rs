use super::*;

pub(super) fn name_archive_fonts(document: &mut LoDocument) -> Result<()> {
    let type_zero_ids = document
        .objects
        .iter()
        .filter_map(|(id, object)| {
            let dictionary = object.as_dict().ok()?;
            (dictionary.get(b"Subtype").and_then(LoObject::as_name).ok() == Some(b"Type0"))
                .then_some(*id)
        })
        .collect::<Vec<_>>();

    for font_id in type_zero_ids {
        let (descriptor_id, archive_name) = {
            let font = document
                .get_dictionary_mut(font_id)
                .map_err(|_| AppError::Data("PDF/A Type0 font dictionary is invalid.".into()))?;
            let base_font = font
                .get(b"BaseFont")
                .and_then(LoObject::as_name)
                .map_err(|_| AppError::Data("PDF/A font has no BaseFont name.".into()))?;
            let archive_name = archive_font_name(base_font).ok_or_else(|| {
                AppError::Data(format!(
                    "Unexpected bundled PDF/A font {}.",
                    String::from_utf8_lossy(base_font)
                ))
            })?;
            let archive_name = archive_name.as_bytes().to_vec();
            font.set("BaseFont", LoObject::Name(archive_name.clone()));

            let descendants = font
                .get_mut(b"DescendantFonts")
                .and_then(LoObject::as_array_mut)
                .map_err(|_| AppError::Data("PDF/A descendant font list is invalid.".into()))?;
            let descendant = descendants
                .first_mut()
                .ok_or_else(|| AppError::Data("PDF/A descendant font list is empty.".into()))?
                .as_dict_mut()
                .map_err(|_| AppError::Data("PDF/A descendant font is invalid.".into()))?;
            descendant.set("BaseFont", LoObject::Name(archive_name.clone()));
            let descriptor_id = descendant
                .get(b"FontDescriptor")
                .and_then(LoObject::as_reference)
                .map_err(|_| {
                    AppError::Data("PDF/A font descriptor reference is invalid.".into())
                })?;
            (descriptor_id, archive_name)
        };

        let descriptor = document
            .get_dictionary_mut(descriptor_id)
            .map_err(|_| AppError::Data("PDF/A font descriptor is invalid.".into()))?;
        descriptor.set("FontName", LoObject::Name(archive_name));
    }
    Ok(())
}

pub(super) fn archive_font_name(base_font: &[u8]) -> Option<&'static str> {
    match base_font {
        b"SANSRG" | b"DejaVuSans" => Some("DejaVuSans"),
        b"SANSBD" | b"DejaVuSans-Bold" => Some("DejaVuSans-Bold"),
        b"MONORG" | b"DejaVuSansMono" => Some("DejaVuSansMono"),
        b"MONOBD" | b"DejaVuSansMono-Bold" => Some("DejaVuSansMono-Bold"),
        _ => None,
    }
}

/// Parse staged PDF bytes and reject malformed documents. Current PDF/A documents receive the
/// stricter archive-profile checks while historical non-PDF/A certificates remain readable.
pub fn validate_pdf_bytes(bytes: &[u8]) -> Result<()> {
    validate_parseable_pdf(bytes)?;
    if pdf_claims_pdfa(bytes)? {
        validate_pdfa_2b_bytes(bytes)?;
    }
    Ok(())
}

pub(super) fn validate_parseable_pdf(bytes: &[u8]) -> Result<()> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(AppError::Validation(
            "Generated certificate PDF has no valid PDF header.".into(),
        ));
    }

    let mut warnings = Vec::new();
    let document =
        PdfDocument::parse(bytes, &PdfParseOptions::default(), &mut warnings).map_err(|error| {
            AppError::Validation(format!(
                "Generated certificate PDF cannot be parsed: {error}"
            ))
        })?;
    if document.pages.is_empty() {
        return Err(AppError::Validation(
            "Generated certificate PDF contains no pages.".into(),
        ));
    }
    if let Some(warning) = warnings
        .iter()
        .find(|warning| warning.severity == PdfParseErrorSeverity::Error)
    {
        return Err(AppError::Validation(format!(
            "Generated certificate PDF contains an invalid operation: {}",
            warning.msg
        )));
    }
    Ok(())
}

pub(super) fn pdf_claims_pdfa(bytes: &[u8]) -> Result<bool> {
    let document = LoDocument::load_mem_with_options(
        bytes,
        LoLoadOptions::with_max_decompressed_size(PDF_STREAM_MAX_DECOMPRESSED_BYTES),
    )
    .map_err(|error| AppError::Validation(format!("PDF structure is invalid: {error}")))?;
    let metadata = pdf_metadata_bytes(&document)?;
    Ok(metadata.as_deref().is_some_and(|value| {
        value
            .windows(b"pdfaid:part".len())
            .any(|part| part == b"pdfaid:part")
    }))
}

pub fn validate_pdfa_2b_bytes(bytes: &[u8]) -> Result<()> {
    let archive_metadata = certificate_pdf_archive_metadata();
    if archive_metadata.archive_format != "PDF/A-2b" || archive_metadata.font_embedding != "full" {
        return Err(AppError::Validation(
            "Certificate PDF archive metadata configuration is invalid.".into(),
        ));
    }
    let document = LoDocument::load_mem_with_options(
        bytes,
        LoLoadOptions::with_max_decompressed_size(PDF_STREAM_MAX_DECOMPRESSED_BYTES),
    )
    .map_err(|error| AppError::Validation(format!("PDF/A structure is invalid: {error}")))?;
    if document.version != "1.7" {
        return Err(AppError::Validation(
            "Certificate PDF/A document must use PDF 1.7.".into(),
        ));
    }
    if document.trailer.has(b"Encrypt") || document.encryption_state.is_some() {
        return Err(AppError::Validation(
            "Certificate PDF/A document must not be encrypted.".into(),
        ));
    }
    if document.get_pages().is_empty() {
        return Err(AppError::Validation(
            "Certificate PDF/A document contains no pages.".into(),
        ));
    }

    validate_pdfa_xmp(&document)?;
    validate_pdfa_output_intent(&document, archive_metadata.output_intent)?;
    validate_a4_page_boxes(&document)?;
    validate_pdfa_page_colors(&document)?;
    validate_pdfa_image_xobjects(&document)?;
    validate_embedded_fonts(&document, archive_metadata.embedded_fonts)?;
    validate_deterministic_trailer_id(&document)?;
    Ok(())
}

pub(super) fn validate_a4_page_boxes(document: &LoDocument) -> Result<()> {
    let expected = [
        0.0_f32,
        0.0_f32,
        PAGE_WIDTH_MM * 72.0 / 25.4,
        PAGE_HEIGHT_MM * 72.0 / 25.4,
    ];
    for page_id in document.get_pages().values() {
        let page = document
            .get_dictionary(*page_id)
            .map_err(|_| AppError::Validation("PDF/A page dictionary is invalid.".into()))?;
        if page
            .get(b"Rotate")
            .and_then(LoObject::as_i64)
            .is_ok_and(|rotation| rotation != 0)
        {
            return Err(AppError::Validation(
                "Certificate PDF pages must not be rotated.".into(),
            ));
        }
        for box_name in [b"MediaBox".as_slice(), b"CropBox", b"TrimBox"] {
            let values = page
                .get(box_name)
                .and_then(LoObject::as_array)
                .map_err(|_| {
                    AppError::Validation(format!(
                        "Certificate PDF page has no valid {}.",
                        String::from_utf8_lossy(box_name)
                    ))
                })?;
            if values.len() != expected.len()
                || values.iter().zip(expected).any(|(actual, expected)| {
                    actual
                        .as_float()
                        .map_or(true, |actual| (actual - expected).abs() > 0.02)
                })
            {
                return Err(AppError::Validation(format!(
                    "Certificate PDF {} is not A4.",
                    String::from_utf8_lossy(box_name)
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn pdf_metadata_bytes(document: &LoDocument) -> Result<Option<Vec<u8>>> {
    let root_id = document
        .trailer
        .get(b"Root")
        .and_then(LoObject::as_reference)
        .map_err(|_| AppError::Validation("PDF catalog reference is missing.".into()))?;
    let catalog = document
        .get_dictionary(root_id)
        .map_err(|_| AppError::Validation("PDF catalog is invalid.".into()))?;
    let Ok(metadata_id) = catalog.get(b"Metadata").and_then(LoObject::as_reference) else {
        return Ok(None);
    };
    let metadata = document
        .get_object(metadata_id)
        .and_then(LoObject::as_stream)
        .map_err(|_| AppError::Validation("PDF metadata stream is invalid.".into()))?;
    if metadata.dict.get(b"Type").and_then(LoObject::as_name).ok() != Some(b"Metadata")
        || metadata
            .dict
            .get(b"Subtype")
            .and_then(LoObject::as_name)
            .ok()
            != Some(b"XML")
    {
        return Err(AppError::Validation(
            "PDF metadata stream is not declared as XML metadata.".into(),
        ));
    }
    metadata
        .decompressed_content()
        .map(Some)
        .map_err(|_| AppError::Validation("PDF metadata stream cannot be decoded.".into()))
}

pub(super) fn validate_pdfa_xmp(document: &LoDocument) -> Result<()> {
    let metadata = pdf_metadata_bytes(document)?.ok_or_else(|| {
        AppError::Validation("Certificate PDF/A document has no XMP metadata.".into())
    })?;
    let xmp = std::str::from_utf8(&metadata)
        .map_err(|_| AppError::Validation("Certificate PDF/A XMP is not UTF-8.".into()))?;
    for required in [
        "<pdfaid:part>2</pdfaid:part>",
        "<pdfaid:conformance>B</pdfaid:conformance>",
        "<dc:format>application/pdf</dc:format>",
        "<xmp:CreateDate>",
        "<xmpMM:DocumentID>uuid:",
        "<xmpMM:InstanceID>uuid:",
    ] {
        if !xmp.contains(required) {
            return Err(AppError::Validation(format!(
                "Certificate PDF/A XMP is missing {required}."
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_pdfa_output_intent(
    document: &LoDocument,
    expected_name: &str,
) -> Result<()> {
    let root_id = document
        .trailer
        .get(b"Root")
        .and_then(LoObject::as_reference)
        .map_err(|_| AppError::Validation("PDF/A catalog reference is missing.".into()))?;
    let catalog = document
        .get_dictionary(root_id)
        .map_err(|_| AppError::Validation("PDF/A catalog is invalid.".into()))?;
    let intents = catalog
        .get(b"OutputIntents")
        .and_then(LoObject::as_array)
        .map_err(|_| AppError::Validation("PDF/A output intent is missing.".into()))?;
    for item in intents {
        let resolved = resolve_object(document, item)?;
        let Ok(intent) = resolved.as_dict() else {
            continue;
        };
        if intent.get(b"S").and_then(LoObject::as_name).ok() != Some(b"GTS_PDFA1") {
            continue;
        }
        if intent.get(b"Info").and_then(LoObject::as_str).ok() != Some(expected_name.as_bytes()) {
            return Err(AppError::Validation(
                "PDF/A output intent identifier is unexpected.".into(),
            ));
        }
        let profile = intent
            .get(b"DestOutputProfile")
            .map_err(|_| AppError::Validation("PDF/A ICC profile reference is missing.".into()))?;
        let profile = resolve_object(document, profile)?;
        let profile = profile
            .as_stream()
            .map_err(|_| AppError::Validation("PDF/A ICC profile is invalid.".into()))?;
        let bytes = profile
            .decompressed_content()
            .map_err(|_| AppError::Validation("PDF/A ICC profile cannot be decoded.".into()))?;
        if profile.dict.get(b"N").and_then(LoObject::as_i64).ok() != Some(4)
            || bytes.len() < 128
            || bytes.get(8).is_none_or(|major_version| *major_version >= 5)
            || bytes.get(12..16) != Some(b"prtr")
            || bytes.get(16..20) != Some(b"CMYK")
            || bytes.get(36..40) != Some(b"acsp")
        {
            return Err(AppError::Validation(
                "PDF/A output intent does not contain a valid CMYK ICC profile.".into(),
            ));
        }
        return Ok(());
    }
    Err(AppError::Validation(
        "Certificate PDF/A document has no GTS_PDFA1 output intent.".into(),
    ))
}

pub(super) fn validate_pdfa_page_colors(document: &LoDocument) -> Result<()> {
    for page_id in document.get_pages().values() {
        let content = document
            .get_page_content_with_limit(*page_id, PDF_PAGE_CONTENT_MAX_DECOMPRESSED_BYTES)
            .map_err(|_| {
                AppError::Validation("PDF/A page content exceeds the validation limit.".into())
            })?;
        let content = LoContent::decode(&content)
            .map_err(|_| AppError::Validation("PDF/A page operators are invalid.".into()))?;
        if content
            .operations
            .iter()
            .any(|operation| matches!(operation.operator.as_str(), "rg" | "RG"))
        {
            return Err(AppError::Validation(
                "PDF/A page uses DeviceRGB with a CMYK output intent.".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_pdfa_image_xobjects(document: &LoDocument) -> Result<()> {
    for object in document.objects.values() {
        let LoObject::Stream(stream) = object else {
            continue;
        };
        if stream.dict.get(b"Subtype").and_then(LoObject::as_name).ok() != Some(b"Image") {
            continue;
        }
        validate_pdfa_image_stream(stream)?;
    }
    Ok(())
}

fn validate_pdfa_image_stream(stream: &LoStream) -> Result<()> {
    let width = stream
        .dict
        .get(b"Width")
        .and_then(LoObject::as_i64)
        .map_err(|_| AppError::Validation("PDF artwork image width is missing.".into()))?;
    let height = stream
        .dict
        .get(b"Height")
        .and_then(LoObject::as_i64)
        .map_err(|_| AppError::Validation("PDF artwork image height is missing.".into()))?;
    let interpolation_is_pdfa_safe = match stream.dict.get(b"Interpolate") {
        Err(_) | Ok(LoObject::Boolean(false)) => true,
        Ok(_) => false,
    };
    if width <= 0
        || height <= 0
        || width > i64::from(ARTWORK_PREVIEW_MAX_PIXELS)
        || height > i64::from(ARTWORK_PREVIEW_MAX_PIXELS)
        || stream
            .dict
            .get(b"BitsPerComponent")
            .and_then(LoObject::as_i64)
            .ok()
            != Some(8)
        || stream
            .dict
            .get(b"ColorSpace")
            .and_then(LoObject::as_name)
            .ok()
            != Some(b"DeviceCMYK")
        || stream.dict.has(b"SMask")
        || stream.dict.has(b"Mask")
        || stream.dict.has(b"Alternates")
        || stream.dict.has(b"OPI")
        || !interpolation_is_pdfa_safe
    {
        return Err(AppError::Validation(
            "PDF artwork image is not a bounded, opaque 8-bit CMYK preview.".into(),
        ));
    }
    let expected = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AppError::Validation("PDF artwork image size overflow.".into()))?;
    let pixels = stream
        .decompressed_content_with_limit(expected.saturating_add(1))
        .map_err(|_| AppError::Validation("PDF artwork image cannot be decoded.".into()))?;
    if pixels.len() != expected {
        return Err(AppError::Validation(
            "PDF artwork image pixel length is inconsistent.".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_embedded_fonts(
    document: &LoDocument,
    expected_fonts: &[&str],
) -> Result<()> {
    let mut embedded_fonts = HashSet::new();
    for page_id in document.get_pages().values() {
        let page = document
            .get_dictionary(*page_id)
            .map_err(|_| AppError::Validation("PDF/A page dictionary is invalid.".into()))?;
        let resources = page
            .get(b"Resources")
            .map_err(|_| AppError::Validation("PDF/A page resources are missing.".into()))?;
        let resources = resolve_object(document, resources)?;
        let resources = resources
            .as_dict()
            .map_err(|_| AppError::Validation("PDF/A page resources are invalid.".into()))?;
        let fonts = resources
            .get(b"Font")
            .map_err(|_| AppError::Validation("PDF/A page font resources are missing.".into()))?;
        let fonts = resolve_object(document, fonts)?;
        let fonts = fonts.as_dict().map_err(|_| {
            AppError::Validation("PDF/A font resource dictionary is invalid.".into())
        })?;
        for (resource_name, font) in fonts.iter() {
            let font_name = validate_embedded_font(document, resource_name, font)?;
            embedded_fonts.insert(font_name);
        }
    }
    if embedded_fonts.is_empty() {
        return Err(AppError::Validation(
            "Certificate PDF/A document contains no font resources.".into(),
        ));
    }
    let expected = expected_fonts
        .iter()
        .copied()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if !embedded_fonts.is_subset(&expected) {
        return Err(AppError::Validation(
            "Certificate PDF/A embedded font set contains an unexpected font.".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_embedded_font(
    document: &LoDocument,
    resource_name: &[u8],
    font: &LoObject,
) -> Result<String> {
    let font = resolve_object(document, font)?;
    let font = font
        .as_dict()
        .map_err(|_| AppError::Validation("PDF/A font dictionary is invalid.".into()))?;
    let (base_font, base_font_name) = validate_type0_font_header(font, resource_name)?;
    validate_to_unicode_map(document, font)?;
    validate_descendant_font_embedding(document, font, &base_font)?;
    Ok(base_font_name)
}

fn validate_type0_font_header(
    font: &LoDictionary,
    resource_name: &[u8],
) -> Result<(Vec<u8>, String)> {
    if font.get(b"Subtype").and_then(LoObject::as_name).ok() != Some(b"Type0") {
        return Err(AppError::Validation(format!(
            "PDF/A font {} is not an embedded composite font.",
            String::from_utf8_lossy(resource_name)
        )));
    }
    let base_font = font
        .get(b"BaseFont")
        .and_then(LoObject::as_name)
        .map_err(|_| AppError::Validation("PDF/A font has no BaseFont name.".into()))?;
    if base_font.contains(&b'+') {
        return Err(AppError::Validation(
            "PDF/A font metadata claims full embedding but uses a subset name.".into(),
        ));
    }
    let base_font_name = String::from_utf8(base_font.to_vec())
        .map_err(|_| AppError::Validation("PDF/A embedded font name is not ASCII.".into()))?;
    Ok((base_font.to_vec(), base_font_name))
}

fn validate_to_unicode_map(document: &LoDocument, font: &LoDictionary) -> Result<()> {
    let to_unicode = font
        .get(b"ToUnicode")
        .map_err(|_| AppError::Validation("PDF/A embedded font has no ToUnicode map.".into()))?;
    let to_unicode = resolve_object(document, to_unicode)?;
    let to_unicode = to_unicode
        .as_stream()
        .map_err(|_| AppError::Validation("PDF/A ToUnicode map is invalid.".into()))?
        .decompressed_content()
        .map_err(|_| AppError::Validation("PDF/A ToUnicode map cannot be decoded.".into()))?;
    if to_unicode.is_empty() {
        return Err(AppError::Validation("PDF/A ToUnicode map is empty.".into()));
    }
    Ok(())
}

fn validate_descendant_font_embedding(
    document: &LoDocument,
    font: &LoDictionary,
    base_font: &[u8],
) -> Result<()> {
    let descendants = font
        .get(b"DescendantFonts")
        .and_then(LoObject::as_array)
        .map_err(|_| AppError::Validation("PDF/A descendant font is missing.".into()))?;
    let descendant = descendants
        .first()
        .ok_or_else(|| AppError::Validation("PDF/A descendant font list is empty.".into()))?;
    let descendant = resolve_object(document, descendant)?;
    let descendant = descendant
        .as_dict()
        .map_err(|_| AppError::Validation("PDF/A descendant font is invalid.".into()))?;
    if descendant.get(b"BaseFont").and_then(LoObject::as_name).ok() != Some(base_font) {
        return Err(AppError::Validation(
            "PDF/A descendant font name does not match its Type0 font.".into(),
        ));
    }
    let descriptor = descendant
        .get(b"FontDescriptor")
        .map_err(|_| AppError::Validation("PDF/A font descriptor is missing.".into()))?;
    let descriptor = resolve_object(document, descriptor)?;
    let descriptor = descriptor
        .as_dict()
        .map_err(|_| AppError::Validation("PDF/A font descriptor is invalid.".into()))?;
    if descriptor.get(b"FontName").and_then(LoObject::as_name).ok() != Some(base_font) {
        return Err(AppError::Validation(
            "PDF/A font descriptor name does not match its Type0 font.".into(),
        ));
    }
    let font_file = descriptor
        .get(b"FontFile2")
        .or_else(|_| descriptor.get(b"FontFile3"))
        .map_err(|_| AppError::Validation("PDF/A font program is not embedded.".into()))?;
    let font_file = resolve_object(document, font_file)?;
    let font_file = font_file
        .as_stream()
        .map_err(|_| AppError::Validation("PDF/A embedded font program is invalid.".into()))?
        .decompressed_content()
        .map_err(|_| {
            AppError::Validation("PDF/A embedded font program cannot be decoded.".into())
        })?;
    if font_file.len() < 4 {
        return Err(AppError::Validation(
            "PDF/A embedded font program is empty.".into(),
        ));
    }
    validate_font_embedding_permissions(&font_file)?;
    Ok(())
}

pub(super) fn validate_font_embedding_permissions(font_file: &[u8]) -> Result<()> {
    let table_count = font_file
        .get(4..6)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| AppError::Validation("PDF/A embedded font header is invalid.".into()))?;
    let mut os2_range = None;
    for index in 0..usize::from(table_count) {
        let record = 12 + index * 16;
        let Some(tag) = font_file.get(record..record + 4) else {
            break;
        };
        if tag != b"OS/2" {
            continue;
        }
        let offset = font_file
            .get(record + 8..record + 12)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_be_bytes)
            .map(|value| value as usize)
            .ok_or_else(|| {
                AppError::Validation("PDF/A embedded font OS/2 offset is invalid.".into())
            })?;
        let length = font_file
            .get(record + 12..record + 16)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_be_bytes)
            .map(|value| value as usize)
            .ok_or_else(|| {
                AppError::Validation("PDF/A embedded font OS/2 length is invalid.".into())
            })?;
        os2_range = offset.checked_add(length).map(|end| (offset, end));
        break;
    }
    let Some((offset, end)) = os2_range else {
        return Err(AppError::Validation(
            "PDF/A embedded TrueType font has no OS/2 permissions table.".into(),
        ));
    };
    let os2 = font_file
        .get(offset..end)
        .ok_or_else(|| AppError::Validation("PDF/A embedded font OS/2 table is invalid.".into()))?;
    let fs_type = os2
        .get(8..10)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_be_bytes)
        .ok_or_else(|| {
            AppError::Validation("PDF/A embedded font permissions are invalid.".into())
        })?;
    if fs_type & 0x0002 != 0 || fs_type & 0x0100 != 0 {
        return Err(AppError::Validation(
            "PDF/A font license flags forbid embedding or subsetting.".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_deterministic_trailer_id(document: &LoDocument) -> Result<()> {
    let ids = document
        .trailer
        .get(b"ID")
        .and_then(LoObject::as_array)
        .map_err(|_| AppError::Validation("PDF/A trailer ID is missing.".into()))?;
    if ids.len() != 2
        || ids
            .iter()
            .any(|id| id.as_str().map_or(true, |value| value.len() != 32))
    {
        return Err(AppError::Validation(
            "PDF/A trailer ID has an unexpected format.".into(),
        ));
    }
    Ok(())
}

pub(super) fn resolve_object(document: &LoDocument, object: &LoObject) -> Result<LoObject> {
    match object {
        LoObject::Reference(id) => document
            .get_object(*id)
            .cloned()
            .map_err(|_| AppError::Validation("PDF indirect object is missing.".into())),
        object => Ok(object.clone()),
    }
}
