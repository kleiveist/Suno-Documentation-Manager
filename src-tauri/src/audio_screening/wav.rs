use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct PcmFormat {
    pub(super) channels: u16,
    pub(super) sample_rate: u32,
    pub(super) byte_rate: u32,
    pub(super) block_align: u16,
    pub(super) bit_depth: u16,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DataSegment {
    offset: u64,
    bytes: u64,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedPcmWav {
    pub(super) format: PcmFormat,
    data_segments: Vec<DataSegment>,
    pub(super) total_frames: u64,
}

/// Supports ordinary RIFF PCM WAV only.  Other accepted release formats still
/// get local Chromaprint analysis through bundled `fpcalc`; external sampling
/// refuses them explicitly instead of silently converting or uploading them.
#[cfg(test)]
pub fn extract_bounded_pcm_wav_sample(
    source: &Path,
) -> std::result::Result<ExtractedWavSample, WavSampleError> {
    let parsed = parse_pcm_wav(source)?;
    let sample_frames = parsed.total_frames.min(max_pcm_sample_frames(&parsed)?);
    if sample_frames == 0 {
        return Err(WavSampleError::InvalidAudio);
    }
    let start_frame = (parsed.total_frames - sample_frames) / 2;
    extract_pcm_wav_sample_at(source, &parsed, start_frame, sample_frames)
}

pub(super) fn parse_pcm_wav(source: &Path) -> std::result::Result<ParsedPcmWav, WavSampleError> {
    let metadata = fs::symlink_metadata(source).map_err(|_| WavSampleError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WavSampleError::Io);
    }
    let mut file = File::open(source).map_err(|_| WavSampleError::Io)?;
    let riff_end = read_riff_end(&mut file, metadata.len())?;
    let (format, data_segments) = read_pcm_chunks(&mut file, riff_end)?;
    finish_pcm_wav_parse(format, data_segments)
}

fn read_riff_end(file: &mut File, file_length: u64) -> std::result::Result<u64, WavSampleError> {
    if file_length < 12 {
        return Err(WavSampleError::UnsupportedFormat);
    }
    let mut header = [0_u8; 12];
    file.read_exact(&mut header)
        .map_err(|_| WavSampleError::InvalidAudio)?;
    if header[0..4] != *b"RIFF" || header[8..12] != *b"WAVE" {
        return Err(WavSampleError::UnsupportedFormat);
    }
    let declared_size = u32::from_le_bytes(header[4..8].try_into().expect("four bytes"));
    if declared_size < 4 {
        return Err(WavSampleError::InvalidAudio);
    }
    let riff_end = 8_u64
        .checked_add(u64::from(declared_size))
        .ok_or(WavSampleError::InvalidAudio)?;
    if riff_end > file_length {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(riff_end)
}

fn read_pcm_chunks(
    file: &mut File,
    riff_end: u64,
) -> std::result::Result<(Option<PcmFormat>, Vec<DataSegment>), WavSampleError> {
    let mut position = 12_u64;
    let mut chunk_count = 0_usize;
    let mut format = None;
    let mut data_segments = Vec::new();
    while position < riff_end {
        chunk_count += 1;
        if chunk_count > MAX_RIFF_CHUNKS || riff_end - position < 8 {
            return Err(WavSampleError::InvalidAudio);
        }
        file.seek(SeekFrom::Start(position))
            .map_err(|_| WavSampleError::Io)?;
        let mut chunk_header = [0_u8; 8];
        file.read_exact(&mut chunk_header)
            .map_err(|_| WavSampleError::InvalidAudio)?;
        let chunk_size = u64::from(u32::from_le_bytes(
            chunk_header[4..8].try_into().expect("four bytes"),
        ));
        let data_start = position
            .checked_add(8)
            .ok_or(WavSampleError::InvalidAudio)?;
        let data_end = data_start
            .checked_add(chunk_size)
            .ok_or(WavSampleError::InvalidAudio)?;
        let padded_end = data_end
            .checked_add(chunk_size & 1)
            .ok_or(WavSampleError::InvalidAudio)?;
        if padded_end > riff_end {
            return Err(WavSampleError::InvalidAudio);
        }
        match &chunk_header[0..4] {
            b"fmt " if format.is_none() => {
                format = Some(read_pcm_format(file, data_start, chunk_size)?);
            }
            b"data" => data_segments.push(DataSegment {
                offset: data_start,
                bytes: chunk_size,
            }),
            _ => {}
        }
        position = padded_end;
    }
    Ok((format, data_segments))
}

fn finish_pcm_wav_parse(
    format: Option<PcmFormat>,
    data_segments: Vec<DataSegment>,
) -> std::result::Result<ParsedPcmWav, WavSampleError> {
    let format = format.ok_or(WavSampleError::UnsupportedFormat)?;
    if data_segments.is_empty() {
        return Err(WavSampleError::InvalidAudio);
    }
    let total_data_bytes = data_segments.iter().try_fold(0_u64, |total, segment| {
        total
            .checked_add(segment.bytes)
            .ok_or(WavSampleError::InvalidAudio)
    })?;
    if total_data_bytes == 0 || total_data_bytes % u64::from(format.block_align) != 0 {
        return Err(WavSampleError::InvalidAudio);
    }
    let total_frames = total_data_bytes / u64::from(format.block_align);
    Ok(ParsedPcmWav {
        format,
        data_segments,
        total_frames,
    })
}

pub(super) fn max_pcm_sample_frames(
    parsed: &ParsedPcmWav,
) -> std::result::Result<u64, WavSampleError> {
    let duration_limit_frames = u64::from(parsed.format.sample_rate)
        .checked_mul(MAX_SAMPLE_SECONDS)
        .ok_or(WavSampleError::InvalidAudio)?;
    // The provider's upload cap applies to the complete RIFF document, not
    // only its PCM data chunk. Reserve the fixed header and a possible pad
    // byte before rounding down to whole PCM frames.
    let max_pcm_payload_bytes = MAX_SAMPLE_AUDIO_BYTES
        .checked_sub(PCM_WAV_HEADER_BYTES + PCM_WAV_MAX_PADDING_BYTES)
        .ok_or(WavSampleError::InvalidAudio)?;
    let upload_limit_frames = max_pcm_payload_bytes / u64::from(parsed.format.block_align);
    Ok(duration_limit_frames.min(upload_limit_frames))
}

pub(super) fn extract_pcm_wav_sample_at(
    source: &Path,
    parsed: &ParsedPcmWav,
    start_frame: u64,
    sample_frames: u64,
) -> std::result::Result<ExtractedWavSample, WavSampleError> {
    if sample_frames == 0
        || sample_frames > max_pcm_sample_frames(parsed)?
        || start_frame
            .checked_add(sample_frames)
            .is_none_or(|end| end > parsed.total_frames)
    {
        return Err(WavSampleError::InvalidAudio);
    }
    let sample_bytes_len = sample_frames
        .checked_mul(u64::from(parsed.format.block_align))
        .ok_or(WavSampleError::InvalidAudio)?;
    let sample_bytes_len =
        usize::try_from(sample_bytes_len).map_err(|_| WavSampleError::InvalidAudio)?;
    let mut file = File::open(source).map_err(|_| WavSampleError::Io)?;
    let samples = copy_pcm_range(
        &mut file,
        &parsed.data_segments,
        start_frame * u64::from(parsed.format.block_align),
        sample_bytes_len,
    )?;
    let bytes = build_pcm_wav(parsed.format, &samples)?;
    if bytes.len() > MAX_SAMPLE_AUDIO_BYTES as usize {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(ExtractedWavSample {
        bytes,
        offset_milliseconds: frames_to_milliseconds(start_frame, parsed.format.sample_rate),
        duration_milliseconds: frames_to_milliseconds(sample_frames, parsed.format.sample_rate),
        source_duration_milliseconds: frames_to_milliseconds(
            parsed.total_frames,
            parsed.format.sample_rate,
        ),
    })
}

pub(super) fn read_pcm_format(
    file: &mut File,
    data_start: u64,
    chunk_size: u64,
) -> std::result::Result<PcmFormat, WavSampleError> {
    if chunk_size < 16 {
        return Err(WavSampleError::InvalidAudio);
    }
    file.seek(SeekFrom::Start(data_start))
        .map_err(|_| WavSampleError::Io)?;
    let mut bytes = [0_u8; 16];
    file.read_exact(&mut bytes)
        .map_err(|_| WavSampleError::InvalidAudio)?;
    let format_tag = u16::from_le_bytes(bytes[0..2].try_into().expect("two bytes"));
    let channels = u16::from_le_bytes(bytes[2..4].try_into().expect("two bytes"));
    let sample_rate = u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes"));
    let byte_rate = u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes"));
    let block_align = u16::from_le_bytes(bytes[12..14].try_into().expect("two bytes"));
    let bit_depth = u16::from_le_bytes(bytes[14..16].try_into().expect("two bytes"));
    if format_tag != 1 {
        return Err(WavSampleError::UnsupportedFormat);
    }
    if channels == 0 || channels > 8 || sample_rate == 0 || !matches!(bit_depth, 8 | 16 | 24 | 32) {
        return Err(WavSampleError::InvalidAudio);
    }
    let bytes_per_sample = u32::from(bit_depth) / 8;
    let expected_align = u32::from(channels)
        .checked_mul(bytes_per_sample)
        .ok_or(WavSampleError::InvalidAudio)?;
    let expected_rate = sample_rate
        .checked_mul(expected_align)
        .ok_or(WavSampleError::InvalidAudio)?;
    if u32::from(block_align) != expected_align || byte_rate != expected_rate {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(PcmFormat {
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bit_depth,
    })
}

pub(super) fn copy_pcm_range(
    file: &mut File,
    segments: &[DataSegment],
    mut global_offset: u64,
    requested_bytes: usize,
) -> std::result::Result<Vec<u8>, WavSampleError> {
    let mut output = Vec::with_capacity(requested_bytes);
    let mut remaining = requested_bytes as u64;
    for segment in segments {
        if global_offset >= segment.bytes {
            global_offset -= segment.bytes;
            continue;
        }
        let available = segment.bytes - global_offset;
        let to_copy = available.min(remaining);
        file.seek(SeekFrom::Start(
            segment
                .offset
                .checked_add(global_offset)
                .ok_or(WavSampleError::InvalidAudio)?,
        ))
        .map_err(|_| WavSampleError::Io)?;
        let mut left = to_copy;
        let mut buffer = [0_u8; 64 * 1024];
        while left > 0 {
            let count = usize::try_from(left.min(buffer.len() as u64))
                .map_err(|_| WavSampleError::InvalidAudio)?;
            file.read_exact(&mut buffer[..count])
                .map_err(|_| WavSampleError::InvalidAudio)?;
            output.extend_from_slice(&buffer[..count]);
            left -= count as u64;
        }
        remaining -= to_copy;
        if remaining == 0 {
            break;
        }
        global_offset = 0;
    }
    if remaining != 0 || output.len() != requested_bytes {
        return Err(WavSampleError::InvalidAudio);
    }
    Ok(output)
}

pub(super) fn build_pcm_wav(
    format: PcmFormat,
    samples: &[u8],
) -> std::result::Result<Vec<u8>, WavSampleError> {
    let sample_length = u32::try_from(samples.len()).map_err(|_| WavSampleError::InvalidAudio)?;
    let data_padding = sample_length & 1;
    let riff_size = 4_u32
        .checked_add(8 + 16)
        .and_then(|size| size.checked_add(8))
        .and_then(|size| size.checked_add(sample_length))
        .and_then(|size| size.checked_add(data_padding))
        .ok_or(WavSampleError::InvalidAudio)?;
    let mut bytes = Vec::with_capacity(riff_size as usize + 8);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&format.channels.to_le_bytes());
    bytes.extend_from_slice(&format.sample_rate.to_le_bytes());
    bytes.extend_from_slice(&format.byte_rate.to_le_bytes());
    bytes.extend_from_slice(&format.block_align.to_le_bytes());
    bytes.extend_from_slice(&format.bit_depth.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&sample_length.to_le_bytes());
    bytes.extend_from_slice(samples);
    if data_padding != 0 {
        bytes.push(0);
    }
    Ok(bytes)
}

pub(super) fn frames_to_milliseconds(frames: u64, sample_rate: u32) -> u64 {
    let milliseconds = (u128::from(frames) * 1000) / u128::from(sample_rate);
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
}
