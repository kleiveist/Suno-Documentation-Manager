use super::*;

/// Render an A4, multi-page technical certificate from a finalized snapshot.
pub fn generate_pdf(snapshot: &CertificatePdfSnapshot<'_>) -> Result<Vec<u8>> {
    let workflow_config = validate_snapshot(snapshot)?;
    let step_labels = workflow_config
        .steps
        .iter()
        .map(|step| (step.id.as_str(), step.name.as_str()))
        .collect::<HashMap<_, _>>();

    let view_model = CertificateViewModel::from_snapshot(snapshot);
    let mut layout = PdfLayout::new(snapshot.render_options);
    layout.set_running_header(&snapshot.track.fields.title, &snapshot.profile.artist_name);
    render_certificate_summary(&mut layout, &view_model);
    layout.new_page();
    render_evidence_overview(&mut layout, &view_model);
    layout.new_page();
    let toc_page = layout.current_page_index();
    layout.mark_bookmark("Contents", toc_page);
    layout.new_page();
    render_full_technical_intro(&mut layout);
    render_header_metadata(&mut layout, snapshot);
    render_track_data(&mut layout, snapshot);
    render_final_suno_generation(&mut layout, snapshot);
    render_sources_and_generation(&mut layout, snapshot);
    render_human_contribution(&mut layout, snapshot);
    render_suno_field(&mut layout, snapshot);
    render_ai_usage(&mut layout, snapshot);
    render_artwork_checks(&mut layout, snapshot);
    render_license_and_rights(&mut layout, snapshot);
    render_external_timestamp_base(&mut layout, snapshot);
    render_evidence_register(&mut layout, snapshot.evidence);
    render_integrity_anchors(&mut layout, snapshot);
    render_workflow(&mut layout, snapshot.steps, &step_labels);
    render_audio_screening(&mut layout, snapshot);
    render_certificate_statement(&mut layout, snapshot);
    render_technical_appendix(&mut layout, snapshot);
    layout.render_contents_page(toc_page);

    let (pages, bookmarks) = layout.into_pages(snapshot.certificate_id);
    let document_title = localized_certificate_label(snapshot.render_options, DOCUMENT_TITLE);
    let mut document = PdfDocument::new(&document_title);
    document.metadata.info.author = snapshot.profile.artist_name.clone();
    document.metadata.info.creator = GENERATED_BY.to_owned();
    document.metadata.info.producer = GENERATED_BY.to_owned();
    document.metadata.info.subject = localized_certificate_paragraph(
        snapshot.render_options,
        "Finalized technical documentation, evidence, and integrity snapshot",
    );
    document.metadata.info.identifier = snapshot.certificate_id.to_owned();
    document.metadata.info.keywords = vec![
        "SunoDM".to_owned(),
        "documentation".to_owned(),
        "evidence".to_owned(),
        "SHA-256".to_owned(),
    ];
    install_artwork_xobjects(&mut document, snapshot.artwork_previews);
    if bookmarks
        .iter()
        .any(|bookmark| bookmark.page_index >= pages.len())
    {
        return Err(AppError::Validation(
            "Certificate PDF bookmark target is outside the document.".into(),
        ));
    }
    for (index, bookmark) in bookmarks.iter().enumerate() {
        document.bookmarks.map.insert(
            PageAnnotId(format!("SunoDMBookmark{index:03}")),
            printpdf::PageAnnotation {
                name: bookmark.title.clone(),
                // printpdf serializes bookmark destinations as 1-based page
                // numbers and subtracts one internally.
                page: bookmark.page_index + 1,
            },
        );
    }
    document.with_pages(pages);

    let language = match snapshot.render_options.language {
        CertificateLanguage::De => "de-DE",
        CertificateLanguage::En => "en-US",
    };
    serialize_pdfa_2b(
        document,
        snapshot.certificate_id,
        snapshot.finalized_at,
        language,
    )
}

pub(super) fn install_artwork_xobjects(
    document: &mut PdfDocument,
    previews: &[CertificateArtworkPreview],
) {
    let mut installed = HashSet::new();
    for preview in previews {
        if !installed.insert(preview.resource_id.as_str()) {
            continue;
        }
        let mut dictionary = BTreeMap::new();
        dictionary.insert("Type".into(), DictItem::Name(b"XObject".to_vec()));
        dictionary.insert("Subtype".into(), DictItem::Name(b"Image".to_vec()));
        dictionary.insert(
            "Width".into(),
            DictItem::Int(i64::from(preview.width_pixels)),
        );
        dictionary.insert(
            "Height".into(),
            DictItem::Int(i64::from(preview.height_pixels)),
        );
        dictionary.insert("ColorSpace".into(), DictItem::Name(b"DeviceCMYK".to_vec()));
        dictionary.insert("BitsPerComponent".into(), DictItem::Int(8));
        // PDF/A-2 requires image interpolation to be absent or explicitly
        // disabled.  Keep it explicit so the conformance validator can prove
        // the generated thumbnail resources are compliant.
        dictionary.insert("Interpolate".into(), DictItem::Bool(false));
        let xobject = ExternalXObject {
            stream: ExternalStream {
                dict: dictionary,
                content: preview.cmyk_pixels.as_ref().clone(),
                compress: true,
            },
            width: Some(Px(preview.width_pixels as usize)),
            height: Some(Px(preview.height_pixels as usize)),
            dpi: Some(300.0),
        };
        document
            .resources
            .xobjects
            .map
            .insert(preview.xobject_id(), XObject::External(xobject));
    }
}

pub(super) fn serialize_pdfa_2b(
    mut document: PdfDocument,
    deterministic_seed: &str,
    created_at: &str,
    language: &str,
) -> Result<Vec<u8>> {
    // DE/EN certificates are distinct archival renditions of one snapshot and
    // therefore need distinct, deterministic XMP/trailer identifiers.
    let rendition_seed = format!("{deterministic_seed}:{language}");
    let (pdf_date, xmp_date) = deterministic_pdf_dates(created_at)?;
    document.metadata.info.creation_date = pdf_date;
    document.metadata.info.modification_date = pdf_date;
    document.metadata.info.metadata_date = pdf_date;
    document.metadata.info.conformance = PdfConformance::A2B_2011_PDF_1_7;
    install_archive_fonts(&mut document)?;

    let xmp = render_pdfa_xmp(
        &document.metadata.info,
        &rendition_seed,
        &xmp_date,
        language,
    );
    // The four project-owned archive assets are embedded in full. This keeps the
    // renderer independent of host fonts and preserves all glyphs needed for DE/EN.
    let save_options = PdfSaveOptions {
        subset_fonts: false,
        ..PdfSaveOptions::default()
    };
    let mut warnings = Vec::new();
    let mut archive = document.to_lopdf_document(&save_options, &mut warnings);
    if let Some(warning) = warnings
        .iter()
        .find(|warning| warning.severity == PdfParseErrorSeverity::Error)
    {
        return Err(AppError::Data(format!(
            "Certificate PDF serialization failed: {}",
            warning.msg
        )));
    }
    postprocess_pdfa_2b(&mut archive, &rendition_seed, language, xmp)?;

    let mut bytes = Vec::new();
    archive
        .save_to(&mut bytes)
        .map_err(|error| AppError::Data(format!("PDF/A-2b serialization failed: {error}")))?;
    validate_pdfa_2b_bytes(&bytes)?;
    Ok(bytes)
}

pub(super) fn deterministic_pdf_dates(
    created_at: &str,
) -> Result<(printpdf::date::OffsetDateTime, String)> {
    let parsed = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| AppError::Data("Certificate PDF timestamp is not valid RFC 3339.".into()))?;
    let utc = parsed.with_timezone(&Utc);
    let pdf_date =
        printpdf::date::OffsetDateTime::from_unix_timestamp(utc.timestamp()).map_err(|error| {
            AppError::Data(format!("Certificate PDF timestamp is invalid: {error}"))
        })?;
    Ok((pdf_date, utc.to_rfc3339_opts(SecondsFormat::Secs, true)))
}

pub(super) fn render_pdfa_xmp(
    info: &printpdf::PdfDocumentInfo,
    deterministic_seed: &str,
    date: &str,
    language: &str,
) -> String {
    let document_id = deterministic_uuid(deterministic_seed, "document");
    let instance_id = deterministic_uuid(deterministic_seed, "instance");
    let title = xml_escape(&info.document_title);
    let author = xml_escape(&info.author);
    let creator = xml_escape(&info.creator);
    let producer = xml_escape(&info.producer);
    let subject = xml_escape(&info.subject);
    let identifier = xml_escape(&info.identifier);
    let keywords = xml_escape(&info.keywords.join(","));
    let date = xml_escape(date);
    let language = xml_escape(language);

    format!(
        r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="{producer}">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
      xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/"
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/">
      <pdfaid:part>{PDFA_PART}</pdfaid:part>
      <pdfaid:conformance>{PDFA_CONFORMANCE}</pdfaid:conformance>
      <dc:format>application/pdf</dc:format>
      <dc:title><rdf:Alt><rdf:li xml:lang="x-default">{title}</rdf:li></rdf:Alt></dc:title>
      <dc:creator><rdf:Seq><rdf:li>{author}</rdf:li></rdf:Seq></dc:creator>
      <dc:description><rdf:Alt><rdf:li xml:lang="x-default">{subject}</rdf:li></rdf:Alt></dc:description>
      <dc:identifier>{identifier}</dc:identifier>
      <dc:language><rdf:Bag><rdf:li>{language}</rdf:li></rdf:Bag></dc:language>
      <xmp:CreateDate>{date}</xmp:CreateDate>
      <xmp:ModifyDate>{date}</xmp:ModifyDate>
      <xmp:MetadataDate>{date}</xmp:MetadataDate>
      <xmp:CreatorTool>{creator}</xmp:CreatorTool>
      <pdf:Producer>{producer}</pdf:Producer>
      <pdf:Keywords>{keywords}</pdf:Keywords>
      <xmpMM:DocumentID>uuid:{document_id}</xmpMM:DocumentID>
      <xmpMM:InstanceID>uuid:{instance_id}</xmpMM:InstanceID>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
}

pub(super) fn xml_escape(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\'' => "&apos;".to_owned(),
            character => character.to_string(),
        })
        .collect()
}

pub(super) fn deterministic_uuid(seed: &str, purpose: &str) -> String {
    let digest = crate::security::sha256_bytes(format!("{purpose}:{seed}").as_bytes());
    format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    )
}

pub(super) fn postprocess_pdfa_2b(
    document: &mut LoDocument,
    deterministic_seed: &str,
    language: &str,
    xmp: String,
) -> Result<()> {
    document.version = "1.7".into();
    document.binary_mark = vec![0xe2, 0xe3, 0xcf, 0xd3];

    // `ExternalStream::compress` only marks the stream as compressible in
    // printpdf 0.12.5; its document conversion does not call `compress()`.
    // Compress derived thumbnails explicitly to keep certificate size bounded.
    for object in document.objects.values_mut() {
        let LoObject::Stream(stream) = object else {
            continue;
        };
        let is_image =
            stream.dict.get(b"Subtype").and_then(LoObject::as_name).ok() == Some(b"Image");
        let already_filtered = stream.dict.has(b"Filter");
        if is_image && !already_filtered {
            stream.compress().map_err(|error| {
                AppError::Data(format!("Artwork preview compression failed: {error}"))
            })?;
        }
    }

    let root_id = document
        .trailer
        .get(b"Root")
        .and_then(LoObject::as_reference)
        .map_err(|_| AppError::Data("PDF/A catalog reference is missing.".into()))?;
    let metadata = LoStream::new(
        LoDictionary::from_iter([
            ("Type", LoObject::Name(b"Metadata".to_vec())),
            ("Subtype", LoObject::Name(b"XML".to_vec())),
        ]),
        xmp.into_bytes(),
    );
    let metadata_id = document.add_object(LoObject::Stream(metadata));

    {
        let catalog = document
            .get_dictionary_mut(root_id)
            .map_err(|_| AppError::Data("PDF/A catalog is invalid.".into()))?;
        catalog.set("Metadata", LoObject::Reference(metadata_id));
        catalog.set(
            "Lang",
            LoObject::String(language.as_bytes().to_vec(), LoStringFormat::Literal),
        );
        let intents = catalog
            .get_mut(b"OutputIntents")
            .and_then(LoObject::as_array_mut)
            .map_err(|_| AppError::Data("PDF/A output intent is missing.".into()))?;
        let intent = intents
            .first_mut()
            .ok_or_else(|| AppError::Data("PDF/A output intent is empty.".into()))?
            .as_dict_mut()
            .map_err(|_| AppError::Data("PDF/A output intent is invalid.".into()))?;
        intent.set("S", LoObject::Name(b"GTS_PDFA1".to_vec()));
        if let Some(profile) = intent.remove(b"DestinationOutputProfile") {
            intent.set("DestOutputProfile", profile);
        }
        intent.remove(b"License");
    }
    name_archive_fonts(document)?;

    if let Ok(info_id) = document
        .trailer
        .get(b"Info")
        .and_then(LoObject::as_reference)
    {
        if let Ok(info) = document.get_dictionary_mut(info_id) {
            info.remove(b"GTS_PDFXVersion");
        }
    }

    let digest = crate::security::sha256_bytes(deterministic_seed.as_bytes());
    document.trailer.set(
        "ID",
        LoObject::Array(vec![
            LoObject::String(digest.as_bytes()[..32].to_vec(), LoStringFormat::Literal),
            LoObject::String(digest.as_bytes()[32..].to_vec(), LoStringFormat::Literal),
        ]),
    );
    Ok(())
}
