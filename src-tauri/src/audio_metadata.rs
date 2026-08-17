use crate::error::{AppError, Result};
use crate::model::EmbeddedMetadata;
use chrono::DateTime;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use uuid::Uuid;

const RIFF_HEADER_LEN: u64 = 12;
const CHUNK_HEADER_LEN: u64 = 8;
// Keep persisted text values inside the EvidenceMetadata validation bound.
// Larger optional chunks are skipped without making the WAV import fail.
const MAX_TEXT_CHUNK_BYTES: u64 = 64 * 1024;
const MAX_EMBEDDED_METADATA_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EMBEDDED_METADATA_ENTRIES: usize = 256;
const MAX_RIFF_CHUNKS: usize = 4_096;

const INFO_TEXT_CHUNKS: [[u8; 4]; 25] = [
    *b"IARL", *b"IART", *b"ICMS", *b"ICMT", *b"ICOP", *b"ICRD", *b"ICRP", *b"IDIM", *b"IDPI",
    *b"IENG", *b"IGNR", *b"IKEY", *b"ILGT", *b"IMED", *b"INAM", *b"IPLT", *b"IPRD", *b"ISBJ",
    *b"ISFT", *b"ISHP", *b"ISRC", *b"ISRF", *b"ITCH", *b"ITRK", *b"IWEB",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WavMetadata {
    pub audio_format: String,
    pub channels: Option<u16>,
    pub sample_rate_hz: Option<u32>,
    pub duration_milliseconds: Option<u64>,
    pub bit_depth: Option<u16>,
    pub embedded_metadata: Vec<EmbeddedMetadata>,
    pub suno_studio_detected: bool,
    pub suno_created_timestamp: String,
    pub suno_created_date: String,
    pub suno_id: String,
    pub suno_raw_metadata: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSunoMetadata {
    pub(crate) created_timestamp: String,
    pub(crate) created_date: String,
    pub(crate) id: String,
}

#[derive(Debug, Clone, Copy)]
struct FormatChunk {
    channels: u16,
    sample_rate_hz: u32,
    byte_rate: u32,
    block_align: u16,
    bit_depth: u16,
}

/// Inspects a RIFF/WAVE file without reading audio payloads into memory.
///
/// `Ok(None)` means that the selected file is not a RIFF/WAVE file. A valid WAV
/// without provider metadata still returns `Some`, with empty Suno fields.
pub fn inspect_wav(path: &Path) -> Result<Option<WavMetadata>> {
    let mut file = File::open(path).map_err(|error| AppError::io(path, error))?;
    let file_len = file
        .metadata()
        .map_err(|error| AppError::io(path, error))?
        .len();

    if file_len < 4 {
        return Ok(None);
    }

    let mut riff_id = [0_u8; 4];
    read_exact(&mut file, path, &mut riff_id, "RIFF identifier")?;
    if riff_id != *b"RIFF" {
        return Ok(None);
    }
    if file_len < RIFF_HEADER_LEN {
        return Err(invalid_wav(path, "truncated RIFF header"));
    }

    let mut header_tail = [0_u8; 8];
    read_exact(&mut file, path, &mut header_tail, "RIFF header")?;
    if header_tail[4..8] != *b"WAVE" {
        return Ok(None);
    }

    let riff_size = u32::from_le_bytes(header_tail[0..4].try_into().expect("four-byte RIFF size"));
    if riff_size < 4 {
        return Err(invalid_wav(
            path,
            "RIFF size does not include the WAVE form type",
        ));
    }
    let riff_end = 8_u64
        .checked_add(u64::from(riff_size))
        .ok_or_else(|| invalid_wav(path, "RIFF size overflows the address space"))?;
    if riff_end > file_len {
        return Err(invalid_wav(
            path,
            "declared RIFF size exceeds the file length",
        ));
    }

    let mut format = None;
    let mut data_bytes = 0_u64;
    let mut embedded_metadata = Vec::new();
    let mut embedded_metadata_bytes = 0_u64;
    let mut position = RIFF_HEADER_LEN;
    let mut chunk_count = 0_usize;

    while position < riff_end {
        chunk_count += 1;
        if chunk_count > MAX_RIFF_CHUNKS {
            return Err(invalid_wav(path, "too many top-level chunks"));
        }
        let remaining = riff_end - position;
        if remaining < CHUNK_HEADER_LEN {
            return Err(invalid_wav(path, "truncated top-level chunk header"));
        }

        let (chunk_id, chunk_size, data_start, data_end, padded_end) =
            read_chunk_header(&mut file, path, position, riff_end, "top-level")?;

        match chunk_id {
            id if id == *b"fmt " => {
                if format.is_none() {
                    format = Some(read_format_chunk(&mut file, path, data_start, chunk_size)?);
                }
            }
            id if id == *b"data" => {
                data_bytes = data_bytes
                    .checked_add(chunk_size)
                    .ok_or_else(|| invalid_wav(path, "combined audio data size overflows"))?;
            }
            id if id == *b"LIST" => {
                read_list_chunk(
                    &mut file,
                    path,
                    data_start,
                    data_end,
                    &mut embedded_metadata,
                    &mut embedded_metadata_bytes,
                    &mut chunk_count,
                )?;
            }
            id if is_known_info_text_chunk(id) => {
                read_and_record_text_chunk(
                    &mut file,
                    path,
                    data_start,
                    chunk_size,
                    id,
                    &mut embedded_metadata,
                    &mut embedded_metadata_bytes,
                )?;
            }
            _ => {}
        }

        position = padded_end;
    }

    let duration_milliseconds = format.and_then(|format| duration_ms(format, data_bytes));
    let mut metadata = WavMetadata {
        audio_format: "WAV".to_owned(),
        channels: format.and_then(|value| nonzero_u16(value.channels)),
        sample_rate_hz: format.and_then(|value| nonzero_u32(value.sample_rate_hz)),
        duration_milliseconds,
        bit_depth: format.and_then(|value| nonzero_u16(value.bit_depth)),
        embedded_metadata,
        ..WavMetadata::default()
    };
    populate_suno_metadata(&mut metadata);
    Ok(Some(metadata))
}

fn read_chunk_header(
    file: &mut File,
    path: &Path,
    position: u64,
    container_end: u64,
    context: &str,
) -> Result<([u8; 4], u64, u64, u64, u64)> {
    file.seek(SeekFrom::Start(position))
        .map_err(|error| AppError::io(path, error))?;
    let mut header = [0_u8; 8];
    read_exact(file, path, &mut header, "chunk header")?;

    let chunk_id = header[0..4].try_into().expect("four-byte chunk identifier");
    let chunk_size = u64::from(u32::from_le_bytes(
        header[4..8].try_into().expect("four-byte chunk size"),
    ));
    let data_start = position
        .checked_add(CHUNK_HEADER_LEN)
        .ok_or_else(|| invalid_wav(path, &format!("{context} chunk position overflows")))?;
    let data_end = data_start
        .checked_add(chunk_size)
        .ok_or_else(|| invalid_wav(path, &format!("{context} chunk size overflows")))?;
    let padded_end = data_end
        .checked_add(chunk_size & 1)
        .ok_or_else(|| invalid_wav(path, &format!("{context} chunk padding overflows")))?;
    if padded_end > container_end {
        return Err(invalid_wav(
            path,
            &format!("declared {context} chunk exceeds its container"),
        ));
    }

    Ok((chunk_id, chunk_size, data_start, data_end, padded_end))
}

fn read_format_chunk(
    file: &mut File,
    path: &Path,
    data_start: u64,
    chunk_size: u64,
) -> Result<FormatChunk> {
    if chunk_size < 16 {
        return Err(invalid_wav(path, "the fmt chunk is shorter than 16 bytes"));
    }

    file.seek(SeekFrom::Start(data_start))
        .map_err(|error| AppError::io(path, error))?;
    let mut bytes = [0_u8; 16];
    read_exact(file, path, &mut bytes, "fmt chunk")?;
    Ok(FormatChunk {
        channels: u16::from_le_bytes(bytes[2..4].try_into().expect("two-byte channel count")),
        sample_rate_hz: u32::from_le_bytes(bytes[4..8].try_into().expect("four-byte sample rate")),
        byte_rate: u32::from_le_bytes(bytes[8..12].try_into().expect("four-byte byte rate")),
        block_align: u16::from_le_bytes(bytes[12..14].try_into().expect("two-byte block align")),
        bit_depth: u16::from_le_bytes(bytes[14..16].try_into().expect("two-byte bit depth")),
    })
}

fn read_list_chunk(
    file: &mut File,
    path: &Path,
    data_start: u64,
    data_end: u64,
    metadata: &mut Vec<EmbeddedMetadata>,
    metadata_bytes: &mut u64,
    chunk_count: &mut usize,
) -> Result<()> {
    if data_end - data_start < 4 {
        return Err(invalid_wav(path, "LIST chunk is missing its list type"));
    }

    file.seek(SeekFrom::Start(data_start))
        .map_err(|error| AppError::io(path, error))?;
    let mut list_type = [0_u8; 4];
    read_exact(file, path, &mut list_type, "LIST type")?;
    if list_type != *b"INFO" {
        return Ok(());
    }

    let mut position = data_start + 4;
    while position < data_end {
        *chunk_count += 1;
        if *chunk_count > MAX_RIFF_CHUNKS {
            return Err(invalid_wav(path, "too many RIFF chunks"));
        }
        if data_end - position < CHUNK_HEADER_LEN {
            return Err(invalid_wav(path, "truncated LIST/INFO subchunk header"));
        }
        let (chunk_id, chunk_size, value_start, _value_end, padded_end) =
            read_chunk_header(file, path, position, data_end, "LIST/INFO")?;
        if is_known_info_text_chunk(chunk_id) {
            read_and_record_text_chunk(
                file,
                path,
                value_start,
                chunk_size,
                chunk_id,
                metadata,
                metadata_bytes,
            )?;
        }
        position = padded_end;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn read_and_record_text_chunk(
    file: &mut File,
    path: &Path,
    data_start: u64,
    chunk_size: u64,
    chunk_id: [u8; 4],
    metadata: &mut Vec<EmbeddedMetadata>,
    metadata_bytes: &mut u64,
) -> Result<()> {
    let remaining_budget = MAX_EMBEDDED_METADATA_BYTES.saturating_sub(*metadata_bytes);
    if metadata.len() >= MAX_EMBEDDED_METADATA_ENTRIES || chunk_size > remaining_budget {
        return Ok(());
    }
    if let Some(value) = read_text_chunk(file, path, data_start, chunk_size)? {
        *metadata_bytes = metadata_bytes.saturating_add(value.len() as u64);
        metadata.push(EmbeddedMetadata {
            key: fourcc(chunk_id),
            value,
        });
    }
    Ok(())
}

fn read_text_chunk(
    file: &mut File,
    path: &Path,
    data_start: u64,
    chunk_size: u64,
) -> Result<Option<String>> {
    if chunk_size > MAX_TEXT_CHUNK_BYTES {
        return Ok(None);
    }

    let length = usize::try_from(chunk_size)
        .map_err(|_| invalid_wav(path, "text chunk cannot be represented in memory"))?;
    file.seek(SeekFrom::Start(data_start))
        .map_err(|error| AppError::io(path, error))?;
    let mut bytes = vec![0_u8; length];
    read_exact(file, path, &mut bytes, "metadata text chunk")?;
    while bytes.last() == Some(&0) {
        bytes.pop();
    }

    // WAV INFO text has no universally enforced character encoding. Lossy
    // conversion or removing embedded controls would violate exact-value
    // preservation, so unsafe optional values remain unreported rather than
    // being silently altered or making the evidence import fail validation.
    Ok(String::from_utf8(bytes)
        .ok()
        .filter(|value| is_persistable_metadata_text(value)))
}

fn populate_suno_metadata(metadata: &mut WavMetadata) {
    let mut matches = metadata
        .embedded_metadata
        .iter()
        .map(|entry| entry.value.as_str())
        .filter(|value| has_suno_studio_marker(value));
    let Some(raw) = matches.next() else {
        return;
    };

    metadata.suno_studio_detected = true;
    // More than one distinct provider record is ambiguous. Every original
    // entry remains preserved above, but no arbitrary timestamp becomes a
    // track fact.
    if matches.any(|candidate| candidate != raw) {
        return;
    }
    metadata.suno_raw_metadata = raw.to_owned();

    if let Some(parsed) = parse_suno_metadata(raw) {
        metadata.suno_created_timestamp = parsed.created_timestamp;
        metadata.suno_created_date = parsed.created_date;
        metadata.suno_id = parsed.id;
    }
}

/// Parses the complete provider record used both during import and when
/// checking persisted metadata. Partial records never become track facts.
pub(crate) fn parse_suno_metadata(raw: &str) -> Option<ParsedSunoMetadata> {
    if !is_persistable_metadata_text(raw) {
        return None;
    }

    let mut marker_count = 0_usize;
    let mut created = None;
    let mut id = None;
    for part in raw.split([';', '\n', '\r']) {
        let part = part.trim();
        if part.eq_ignore_ascii_case("made with suno studio") {
            marker_count += 1;
            continue;
        }
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if key.trim().eq_ignore_ascii_case("created") {
            if created.replace(value).is_some() {
                return None;
            }
        } else if key.trim().eq_ignore_ascii_case("id") && id.replace(value).is_some() {
            return None;
        }
    }

    if marker_count != 1 {
        return None;
    }
    let created = created?;
    if created.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let timestamp = DateTime::parse_from_rfc3339(created).ok()?;
    let id = Uuid::parse_str(id?).ok()?;
    Some(ParsedSunoMetadata {
        created_timestamp: created.to_owned(),
        created_date: timestamp.date_naive().to_string(),
        id: id.to_string(),
    })
}

fn duration_ms(format: FormatChunk, data_bytes: u64) -> Option<u64> {
    if data_bytes == 0 {
        return None;
    }
    let byte_rate = if format.byte_rate != 0 {
        u64::from(format.byte_rate)
    } else {
        u64::from(format.sample_rate_hz).checked_mul(u64::from(format.block_align))?
    };
    if byte_rate == 0 {
        return None;
    }
    let milliseconds = u128::from(data_bytes)
        .checked_mul(1000)?
        .checked_div(u128::from(byte_rate))?;
    u64::try_from(milliseconds).ok()
}

pub(crate) fn has_suno_studio_marker(value: &str) -> bool {
    value
        .split([';', '\n', '\r'])
        .any(|part| part.trim().eq_ignore_ascii_case("made with suno studio"))
}

pub(crate) fn is_persistable_metadata_text(value: &str) -> bool {
    let mut characters = 0_usize;
    for character in value.chars() {
        characters += 1;
        if characters > MAX_TEXT_CHUNK_BYTES as usize
            || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return false;
        }
    }
    true
}

fn is_known_info_text_chunk(id: [u8; 4]) -> bool {
    INFO_TEXT_CHUNKS.contains(&id)
}

fn fourcc(id: [u8; 4]) -> String {
    String::from_utf8_lossy(&id).into_owned()
}

fn nonzero_u16(value: u16) -> Option<u16> {
    (value != 0).then_some(value)
}

fn nonzero_u32(value: u32) -> Option<u32> {
    (value != 0).then_some(value)
}

fn read_exact(file: &mut File, path: &Path, buffer: &mut [u8], context: &str) -> Result<()> {
    file.read_exact(buffer).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            invalid_wav(path, &format!("truncated {context}"))
        } else {
            AppError::io(path, error)
        }
    })
}

fn invalid_wav(path: &Path, message: &str) -> AppError {
    AppError::Data(format!("Invalid WAV file '{}': {message}.", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const SUNO_TEXT: &str = "made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828";

    /// Test-only encoder deliberately shares no chunk-writing code with the
    /// parser, so parser and fixture cannot reproduce the same offset bug.
    #[derive(Default)]
    struct WavFixture {
        chunks: Vec<([u8; 4], Vec<u8>)>,
    }

    impl WavFixture {
        fn pcm16_stereo_48khz() -> Self {
            let mut fmt = Vec::new();
            fmt.extend_from_slice(&1_u16.to_le_bytes());
            fmt.extend_from_slice(&2_u16.to_le_bytes());
            fmt.extend_from_slice(&48_000_u32.to_le_bytes());
            fmt.extend_from_slice(&192_000_u32.to_le_bytes());
            fmt.extend_from_slice(&4_u16.to_le_bytes());
            fmt.extend_from_slice(&16_u16.to_le_bytes());
            Self {
                chunks: vec![(*b"fmt ", fmt), (*b"data", vec![0; 1_920])],
            }
        }

        fn chunk(mut self, id: [u8; 4], payload: impl Into<Vec<u8>>) -> Self {
            self.chunks.push((id, payload.into()));
            self
        }

        fn info(mut self, entries: Vec<([u8; 4], Vec<u8>)>) -> Self {
            let mut list = b"INFO".to_vec();
            for (id, value) in entries {
                list.extend_from_slice(&id);
                list.extend_from_slice(&(value.len() as u32).to_le_bytes());
                list.extend_from_slice(&value);
                if value.len() % 2 == 1 {
                    list.push(0);
                }
            }
            self.chunks.push((*b"LIST", list));
            self
        }

        fn bytes(self) -> Vec<u8> {
            let mut chunks = Vec::new();
            for (id, payload) in self.chunks {
                chunks.extend_from_slice(&id);
                chunks.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                chunks.extend_from_slice(&payload);
                if payload.len() % 2 == 1 {
                    chunks.push(0);
                }
            }

            let mut wav = b"RIFF".to_vec();
            wav.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
            wav.extend_from_slice(b"WAVE");
            wav.extend_from_slice(&chunks);
            wav
        }
    }

    fn inspect(bytes: &[u8]) -> Result<Option<WavMetadata>> {
        let temporary = TempDir::new().expect("temporary directory");
        let path = temporary.path().join("fixture.wav");
        fs::write(&path, bytes).expect("write fixture");
        inspect_wav(&path)
    }

    #[test]
    fn reads_pcm_properties_and_suno_info_comment() {
        let mut comment = SUNO_TEXT.as_bytes().to_vec();
        comment.push(0);
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", comment)])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(metadata.audio_format, "WAV");
        assert_eq!(metadata.channels, Some(2));
        assert_eq!(metadata.sample_rate_hz, Some(48_000));
        assert_eq!(metadata.duration_milliseconds, Some(10));
        assert_eq!(metadata.bit_depth, Some(16));
        assert_eq!(
            metadata.embedded_metadata,
            vec![EmbeddedMetadata {
                key: "ICMT".into(),
                value: SUNO_TEXT.into(),
            }]
        );
        assert!(metadata.suno_studio_detected);
        assert_eq!(metadata.suno_created_timestamp, "2026-08-17T06:38:06Z");
        assert_eq!(metadata.suno_created_date, "2026-08-17");
        assert_eq!(metadata.suno_id, "6c8a40fd-32bf-4c7b-ab59-23579ff95828");
        assert_eq!(metadata.suno_raw_metadata, SUNO_TEXT);
    }

    #[test]
    fn ordinary_wav_is_successful_without_invented_suno_metadata() {
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"IART", b"Example artist\0".to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(!metadata.suno_studio_detected);
        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_created_date.is_empty());
        assert!(metadata.suno_id.is_empty());
        assert!(metadata.suno_raw_metadata.is_empty());
        assert_eq!(metadata.embedded_metadata[0].key, "IART");
    }

    #[test]
    fn incidental_marker_words_are_not_a_provider_signature() {
        let raw = "This mix was not made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", raw.as_bytes().to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(!metadata.suno_studio_detected);
        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_id.is_empty());
    }

    #[test]
    fn never_scans_audio_payload_for_suno_text() {
        let wav = WavFixture::pcm16_stereo_48khz()
            .chunk(*b"data", SUNO_TEXT.as_bytes().to_vec())
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(!metadata.suno_studio_detected);
        assert!(metadata.embedded_metadata.is_empty());
    }

    #[test]
    fn filters_forbidden_controls_without_altering_safe_neighboring_text() {
        let unsafe_value = b"made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828\0injected".to_vec();
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![
                (*b"ICMT", unsafe_value),
                (*b"INAM", b"  safe text with trailing spaces  \0\0".to_vec()),
            ])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(
            metadata.embedded_metadata,
            vec![EmbeddedMetadata {
                key: "INAM".into(),
                value: "  safe text with trailing spaces  ".into(),
            }]
        );
        assert!(!metadata.suno_studio_detected);
        assert!(metadata.suno_raw_metadata.is_empty());
    }

    #[test]
    fn accepts_rfc3339_offset_and_uses_its_calendar_date() {
        let raw = "Made With Suno Studio; CREATED=2026-08-17T00:30:00+02:00; ID=6C8A40FD-32BF-4C7B-AB59-23579FF95828";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", raw.as_bytes().to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(metadata.suno_studio_detected);
        assert_eq!(metadata.suno_created_timestamp, "2026-08-17T00:30:00+02:00");
        assert_eq!(metadata.suno_created_date, "2026-08-17");
        assert_eq!(metadata.suno_id, "6c8a40fd-32bf-4c7b-ab59-23579ff95828");
        assert_eq!(metadata.suno_raw_metadata, raw);
    }

    #[test]
    fn rejects_incomplete_non_rfc3339_record_without_partially_accepting_its_uuid() {
        let raw = "made with suno studio; created=2026-08-17 06:38:06; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", raw.as_bytes().to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(metadata.suno_studio_detected);
        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_created_date.is_empty());
        assert!(metadata.suno_id.is_empty());
        assert_eq!(metadata.suno_raw_metadata, raw);
    }

    #[test]
    fn rejects_incomplete_non_uuid_record_without_partially_accepting_its_timestamp() {
        let raw = "made with suno studio; created=2026-08-17T06:38:06Z; id=not-a-uuid";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", raw.as_bytes().to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_created_date.is_empty());
        assert!(metadata.suno_id.is_empty());
        assert_eq!(metadata.suno_raw_metadata, raw);
    }

    #[test]
    fn duplicate_structured_keys_are_treated_as_ambiguous() {
        let raw = "made with suno studio; created=2026-08-17T06:38:06Z; created=2026-08-18T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828; id=180ee4f0-977b-4db8-8968-e93e3ac9d506";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![(*b"ICMT", raw.as_bytes().to_vec())])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(metadata.suno_studio_detected);
        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_created_date.is_empty());
        assert!(metadata.suno_id.is_empty());
    }

    #[test]
    fn central_suno_parser_requires_one_marker_timestamp_and_uuid() {
        assert_eq!(
            parse_suno_metadata(SUNO_TEXT),
            Some(ParsedSunoMetadata {
                created_timestamp: "2026-08-17T06:38:06Z".into(),
                created_date: "2026-08-17".into(),
                id: "6c8a40fd-32bf-4c7b-ab59-23579ff95828".into(),
            })
        );
        for invalid in [
            "made with suno studio; created=2026-08-17T06:38:06Z",
            "made with suno studio; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828",
            "made with suno studio; created=2026-08-17 06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828",
            "made with suno studio; created=2026-08-17T06:38:06Z; id=not-a-uuid",
            "made with suno studio; made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828",
            "made with suno studio; created=2026-08-17T06:38:06Z; created=2026-08-18T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828",
            "made with suno studio; created=2026-08-17T06:38:06Z; id=6c8a40fd-32bf-4c7b-ab59-23579ff95828\u{1b}",
        ] {
            assert_eq!(parse_suno_metadata(invalid), None, "accepted {invalid:?}");
        }
    }

    #[test]
    fn distinct_suno_records_are_preserved_but_not_arbitrarily_selected() {
        let second = "made with suno studio; created=2026-08-18T06:38:06Z; id=180ee4f0-977b-4db8-8968-e93e3ac9d506";
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![
                (*b"ICMT", SUNO_TEXT.as_bytes().to_vec()),
                (*b"ICMT", second.as_bytes().to_vec()),
            ])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert!(metadata.suno_studio_detected);
        assert_eq!(metadata.embedded_metadata.len(), 2);
        assert!(metadata.suno_raw_metadata.is_empty());
        assert!(metadata.suno_created_timestamp.is_empty());
        assert!(metadata.suno_created_date.is_empty());
        assert!(metadata.suno_id.is_empty());
    }

    #[test]
    fn parses_known_top_level_text_and_ignores_unknown_info_entries() {
        let wav = WavFixture::pcm16_stereo_48khz()
            .chunk(*b"INAM", b"Top-level title\0".to_vec())
            .info(vec![
                (*b"ZZZZ", SUNO_TEXT.as_bytes().to_vec()),
                (*b"ISFT", b"Encoder\0".to_vec()),
            ])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(metadata.embedded_metadata.len(), 2);
        assert_eq!(metadata.embedded_metadata[0].key, "INAM");
        assert_eq!(metadata.embedded_metadata[1].key, "ISFT");
        assert!(!metadata.suno_studio_detected);
    }

    #[test]
    fn skips_non_utf8_and_oversized_text_without_large_parser_allocation() {
        let oversized = vec![b'x'; (MAX_TEXT_CHUNK_BYTES + 1) as usize];
        let wav = WavFixture::pcm16_stereo_48khz()
            .info(vec![
                (*b"IART", vec![0xff, 0xfe]),
                (*b"ICMT", oversized),
                (*b"INAM", b"Retained\0".to_vec()),
            ])
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(
            metadata.embedded_metadata,
            vec![EmbeddedMetadata {
                key: "INAM".into(),
                value: "Retained".into(),
            }]
        );
    }

    #[test]
    fn bounds_the_number_of_retained_metadata_entries() {
        let entries = (0..MAX_EMBEDDED_METADATA_ENTRIES + 10)
            .map(|index| (*b"ICMT", format!("comment {index}").into_bytes()))
            .collect();
        let wav = WavFixture::pcm16_stereo_48khz().info(entries).bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(
            metadata.embedded_metadata.len(),
            MAX_EMBEDDED_METADATA_ENTRIES
        );
        assert_eq!(metadata.embedded_metadata[0].value, "comment 0");
        assert_eq!(
            metadata.embedded_metadata.last().expect("last entry").value,
            "comment 255"
        );
    }

    #[test]
    fn rejects_pathological_chunk_counts_without_scanning_unboundedly() {
        let mut fixture = WavFixture::pcm16_stereo_48khz();
        for _ in 0..=MAX_RIFF_CHUNKS {
            fixture = fixture.chunk(*b"JUNK", Vec::new());
        }

        assert!(inspect(&fixture.bytes()).is_err());
    }

    #[test]
    fn respects_odd_chunk_padding_and_fmt_after_data() {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&8_000_u32.to_le_bytes());
        fmt.extend_from_slice(&8_000_u32.to_le_bytes());
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&8_u16.to_le_bytes());
        let wav = WavFixture::default()
            .chunk(*b"JUNK", vec![1, 2, 3])
            .chunk(*b"data", vec![0; 800])
            .chunk(*b"fmt ", fmt)
            .bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(metadata.channels, Some(1));
        assert_eq!(metadata.sample_rate_hz, Some(8_000));
        assert_eq!(metadata.duration_milliseconds, Some(100));
        assert_eq!(metadata.bit_depth, Some(8));
    }

    #[test]
    fn wav_without_fmt_keeps_technical_fields_optional() {
        let wav = WavFixture::default().chunk(*b"data", vec![0; 16]).bytes();

        let metadata = inspect(&wav).expect("inspect WAV").expect("WAV metadata");

        assert_eq!(metadata.audio_format, "WAV");
        assert_eq!(metadata.channels, None);
        assert_eq!(metadata.sample_rate_hz, None);
        assert_eq!(metadata.duration_milliseconds, None);
        assert_eq!(metadata.bit_depth, None);
    }

    #[test]
    fn returns_none_for_non_wav_files() {
        assert_eq!(inspect(b"plain text").expect("inspect non-WAV"), None);

        let mut other_riff = b"RIFF\x04\0\0\0AVI ".to_vec();
        other_riff.extend_from_slice(b"ignored trailing bytes");
        assert_eq!(inspect(&other_riff).expect("inspect non-WAVE RIFF"), None);
    }

    #[test]
    fn reports_truncated_riff_and_chunk_boundaries_without_panicking() {
        assert!(matches!(
            inspect(b"RIFF\x04\0\0").unwrap_err(),
            AppError::Data(_)
        ));

        let mut truncated_chunk = b"RIFF".to_vec();
        truncated_chunk.extend_from_slice(&12_u32.to_le_bytes());
        truncated_chunk.extend_from_slice(b"WAVE");
        truncated_chunk.extend_from_slice(b"data");
        assert!(matches!(
            inspect(&truncated_chunk).unwrap_err(),
            AppError::Data(_)
        ));

        let mut oversized_chunk = b"RIFF".to_vec();
        oversized_chunk.extend_from_slice(&13_u32.to_le_bytes());
        oversized_chunk.extend_from_slice(b"WAVE");
        oversized_chunk.extend_from_slice(b"data");
        oversized_chunk.extend_from_slice(&20_u32.to_le_bytes());
        oversized_chunk.push(0);
        assert!(matches!(
            inspect(&oversized_chunk).unwrap_err(),
            AppError::Data(_)
        ));
    }

    #[test]
    fn reports_malformed_fmt_and_info_subchunks() {
        let short_fmt = WavFixture::default().chunk(*b"fmt ", vec![0; 15]).bytes();
        assert!(matches!(
            inspect(&short_fmt).unwrap_err(),
            AppError::Data(_)
        ));

        let malformed_info = WavFixture::default()
            .chunk(*b"LIST", b"INFOICMT".to_vec())
            .bytes();
        assert!(matches!(
            inspect(&malformed_info).unwrap_err(),
            AppError::Data(_)
        ));
    }
}
