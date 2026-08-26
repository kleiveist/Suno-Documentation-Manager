use crate::error::{AppError, Result};
use std::fs;
use std::io::Read;
use std::path::Path;

const OPEN_TIMESTAMPS_DETACHED_MAGIC: &[u8] = &[
    0x00, b'O', b'p', b'e', b'n', b'T', b'i', b'm', b'e', b's', b't', b'a', b'm', b'p', b's', 0x00,
    0x00, b'P', b'r', b'o', b'o', b'f', 0x00, 0xbf, 0x89, 0xe2, 0xe8, 0x84, 0xe8, 0x92, 0x94,
];

pub(super) fn validate_signature(source: &Path, extension: &str) -> Result<()> {
    let mut file = fs::File::open(source).map_err(|error| AppError::io(source, error))?;
    let mut header = [0_u8; 64];
    let count = file
        .read(&mut header)
        .map_err(|error| AppError::io(source, error))?;
    let bytes = &header[..count];
    let matches = signature_matches(extension, bytes);
    if count == 0 || !matches {
        return Err(AppError::Validation(format!(
            "Evidence file contents do not match the .{extension} file type."
        )));
    }
    Ok(())
}

fn signature_matches(extension: &str, bytes: &[u8]) -> bool {
    image_signature(extension, bytes)
        .or_else(|| document_signature(extension, bytes))
        .or_else(|| audio_signature(extension, bytes))
        .or_else(|| text_signature(extension, bytes))
        .or_else(|| timestamp_signature(extension, bytes))
        .unwrap_or(false)
}

fn image_signature(extension: &str, bytes: &[u8]) -> Option<bool> {
    match extension {
        "png" => Some(bytes.starts_with(b"\x89PNG\r\n\x1a\n")),
        "jpg" | "jpeg" => Some(bytes.starts_with(&[0xff, 0xd8, 0xff])),
        "webp" => Some(riff_kind(bytes, b"WEBP")),
        _ => None,
    }
}

fn document_signature(extension: &str, bytes: &[u8]) -> Option<bool> {
    match extension {
        "pdf" => Some(bytes.starts_with(b"%PDF-")),
        "zip" => Some(
            bytes.starts_with(b"PK\x03\x04")
                || bytes.starts_with(b"PK\x05\x06")
                || bytes.starts_with(b"PK\x07\x08"),
        ),
        _ => None,
    }
}

fn audio_signature(extension: &str, bytes: &[u8]) -> Option<bool> {
    match extension {
        "wav" => Some(riff_kind(bytes, b"WAVE")),
        "aif" | "aiff" => Some(
            bytes.starts_with(b"FORM")
                && bytes.len() >= 12
                && (&bytes[8..12] == b"AIFF" || &bytes[8..12] == b"AIFC"),
        ),
        "mp3" => Some(
            bytes.starts_with(b"ID3")
                || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0),
        ),
        "flac" => Some(bytes.starts_with(b"fLaC")),
        "ogg" => Some(bytes.starts_with(b"OggS")),
        "mp4" | "m4v" | "m4a" => Some(bytes.len() >= 12 && &bytes[4..8] == b"ftyp"),
        _ => None,
    }
}

fn text_signature(extension: &str, bytes: &[u8]) -> Option<bool> {
    const TEXT_EXTENSIONS: &[&str] = &[
        "txt", "md", "json", "rb", "py", "js", "jsx", "ts", "tsx", "mjs", "cjs", "java", "kt",
        "kts", "c", "h", "cc", "cpp", "cxx", "hpp", "cs", "rs", "go", "php", "swift", "scala",
        "sh", "bash", "zsh", "fish", "ps1", "lua", "r", "jl", "ex", "exs", "erl", "hrl", "fs",
        "fsx", "vb", "sql", "html", "htm", "css", "scss", "sass", "less", "xml", "yaml", "yml",
        "toml", "csv", "ipynb", "svg",
    ];
    TEXT_EXTENSIONS
        .contains(&extension)
        .then(|| valid_text_prefix(bytes))
}

fn timestamp_signature(extension: &str, bytes: &[u8]) -> Option<bool> {
    match extension {
        // RFC 3161 responses and detached signature containers are opaque
        // binary evidence. Their legal/cryptographic qualification is not
        // inferred here; the dedicated timestamp workflow records and hashes
        // the exact non-empty bytes.
        "tsr" | "tst" | "p7s" => Some(!bytes.is_empty()),
        "ots" => Some(bytes.starts_with(OPEN_TIMESTAMPS_DETACHED_MAGIC)),
        _ => None,
    }
}

fn riff_kind(bytes: &[u8], kind: &[u8; 4]) -> bool {
    bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == kind
}

fn valid_text_prefix(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }
    match std::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(error) => error.error_len().is_none() && error.valid_up_to() + 4 >= bytes.len(),
    }
}
