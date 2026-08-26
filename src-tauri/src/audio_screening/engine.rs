use super::*;

/// Returns an app-controlled `fpcalc` sidecar if it is present and has the
/// pinned digest.  It never searches `PATH` and never accepts a user path.
pub fn bundled_fpcalc_path() -> std::result::Result<PathBuf, &'static str> {
    let mut candidates = Vec::new();
    if cfg!(debug_assertions) {
        if let Some(name) = development_sidecar_file_name() {
            candidates.push(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("binaries")
                    .join(name),
            );
        }
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            let sidecar_name = packaged_sidecar_file_name();
            candidates.push(parent.join(sidecar_name));
            candidates.push(parent.join("binaries").join(sidecar_name));
            #[cfg(target_os = "macos")]
            if let Some(contents) = parent.parent() {
                candidates.push(contents.join("Resources").join(sidecar_name));
            }
        }
    }

    for candidate in candidates {
        if validate_pinned_fpcalc(&candidate).is_ok() {
            return Ok(candidate);
        }
    }
    Err("The bundled Chromaprint engine is unavailable for this installation.")
}

/// Availability is intentionally a local fact.  No network request is made.
pub fn local_engine_availability() -> (bool, String) {
    match super::bundled_fpcalc_path() {
        Ok(_) => (true, CHROMAPRINT_VERSION.into()),
        Err(_) => (false, String::new()),
    }
}

pub fn refresh_local_engine_status(settings: &mut AudioScreeningSettings) {
    let (available, version) = super::local_engine_availability();
    settings.local_engine_available = available;
    settings.local_engine_version = version;
}

pub(super) fn development_sidecar_file_name() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("fpcalc-x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("fpcalc-aarch64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("fpcalc-x86_64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("fpcalc-aarch64-apple-darwin")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("fpcalc-x86_64-pc-windows-msvc.exe")
    } else {
        None
    }
}

pub(super) fn packaged_sidecar_file_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "fpcalc.exe";
    }
    #[cfg(not(target_os = "windows"))]
    {
        "fpcalc"
    }
}

pub(super) fn expected_fpcalc_sha256() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("e7b14fbf9d544f6ba99b7aced3c07786258e09e37cfcb054a41d2a6eeb0887a7")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("9b6fb816312af0b3ca6052a973ba42f61b23e7a919dce4e3ee18e57c34bf3103")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("c1c368de7db49541320624d5f7d4ad827cbbaca96ee104ca6d4c4e0c917c575e")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("23046544591f275c6da7b0fa57c1290535eb844df271e186e37af1715040921f")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("00dcc56d911f2dea84737aa9dc8e2d118c9eb7a037d815d1ed001d8593e8fbee")
    } else {
        None
    }
}

pub(super) fn validate_pinned_fpcalc(path: &Path) -> std::result::Result<(), &'static str> {
    let Some(expected) = expected_fpcalc_sha256() else {
        return Err("No bundled Chromaprint engine is available for this platform.");
    };
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "Bundled Chromaprint engine is missing.")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Bundled Chromaprint engine is not a regular file.");
    }
    let actual = sha256_file(path).map_err(|_| "Bundled Chromaprint engine cannot be verified.")?;
    if actual != expected {
        return Err("Bundled Chromaprint engine verification failed.");
    }
    Ok(())
}

/// Generates and publishes a local fingerprint record.  Engine failures are a
/// persisted controlled status rather than a fabricated successful result.
pub fn local_fingerprint(
    source_path: &Path,
    track_id: &str,
    evidence: &EvidenceItem,
    track_root: &Path,
    mut progress: impl FnMut(&str, &str),
) -> Result<AudioScreeningLocalRecord> {
    progress("preparing_audio", "Preparing authoritative release audio");
    let mut record = local_record_base(track_id, evidence);
    // `fpcalc` must read a private immutable snapshot, not the live managed
    // file. This closes the check-then-use window where a release file could
    // otherwise change after its SHA-256 binding was verified but before the
    // native decoder opened it.
    let snapshot = match create_verified_source_snapshot(source_path, evidence, track_root) {
        Ok(snapshot) => snapshot,
        Err(()) => {
            record.status = AudioScreeningStatus::ProcessingFailed;
            record.message =
                "The authoritative release audio could not be verified for fingerprinting.".into();
            return finish_local_record(track_root, record, None, &mut progress);
        }
    };

    let binary = match super::bundled_fpcalc_path() {
        Ok(path) => path,
        Err(message) => {
            record.status = AudioScreeningStatus::EngineUnavailable;
            record.message = message.into();
            return finish_local_record(track_root, record, None, &mut progress);
        }
    };
    progress(
        "fingerprinting_audio",
        "Generating local Chromaprint fingerprint",
    );
    match invoke_fpcalc(&binary, &snapshot.path) {
        Ok(output) => {
            record.status = AudioScreeningStatus::FingerprintGenerated;
            record.message = "A local Chromaprint fingerprint was generated from the authoritative release audio.".into();
            record.engine_version = CHROMAPRINT_VERSION.into();
            record.duration_milliseconds = Some(output.duration_milliseconds);
            record.fingerprint = output.fingerprint;
            record.generated_at = Some(Utc::now().to_rfc3339());
            finish_local_record(track_root, record, None, &mut progress)
        }
        Err(failure) => {
            record.status = failure.status();
            record.message = failure.message().into();
            finish_local_record(track_root, record, None, &mut progress)
        }
    }
}

pub(super) fn finish_local_record(
    track_root: &Path,
    mut record: AudioScreeningLocalRecord,
    external: Option<&AudioScreeningExternalRecord>,
    progress: &mut impl FnMut(&str, &str),
) -> Result<AudioScreeningLocalRecord> {
    progress(
        "fingerprint_complete",
        "Local fingerprint operation completed",
    );
    progress("saving_screening_result", "Saving audio-screening record");
    publish_local_screening_artifacts(track_root, &mut record, external)?;
    progress("complete", "Audio screening completed");
    Ok(record)
}

pub(super) fn local_record_base(
    track_id: &str,
    evidence: &EvidenceItem,
) -> AudioScreeningLocalRecord {
    AudioScreeningLocalRecord {
        schema_version: 1,
        status: AudioScreeningStatus::NotRun,
        message: "No local Chromaprint fingerprint has been generated yet.".into(),
        engine: CHROMAPRINT_ENGINE.into(),
        engine_version: String::new(),
        fingerprint_algorithm: FINGERPRINT_ALGORITHM.into(),
        track_id: track_id.to_owned(),
        source_evidence_id: evidence.id.clone(),
        source_relative_path: evidence.relative_path.clone(),
        source_sha256: evidence.sha256.clone().unwrap_or_default(),
        source_size_bytes: evidence.size_bytes,
        duration_milliseconds: None,
        fingerprint: String::new(),
        generated_at: None,
        artifact_relative_path: LOCAL_FINGERPRINT_FILE.into(),
        artifact_sha256: String::new(),
    }
}

#[derive(Debug)]
pub(super) struct FpcalcOutput {
    pub(super) fingerprint: String,
    pub(super) duration_milliseconds: u64,
}

#[derive(Debug)]
pub(super) enum FpcalcFailure {
    EngineUnavailable,
    UnsupportedFormat,
    ProcessingFailed,
}

impl FpcalcFailure {
    fn status(&self) -> AudioScreeningStatus {
        match self {
            Self::EngineUnavailable => AudioScreeningStatus::EngineUnavailable,
            Self::UnsupportedFormat => AudioScreeningStatus::UnsupportedFormat,
            Self::ProcessingFailed => AudioScreeningStatus::ProcessingFailed,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::EngineUnavailable => "The bundled Chromaprint engine is unavailable.",
            Self::UnsupportedFormat => {
                "The authoritative release audio format is not supported by the bundled Chromaprint decoder."
            }
            Self::ProcessingFailed => {
                "Chromaprint could not generate a fingerprint for the authoritative release audio."
            }
        }
    }
}

pub(super) fn invoke_fpcalc(
    binary: &Path,
    source: &Path,
) -> std::result::Result<FpcalcOutput, FpcalcFailure> {
    let mut child = Command::new(binary)
        .arg("-json")
        .arg("-algorithm")
        .arg(FINGERPRINT_ALGORITHM)
        .arg("-length")
        .arg("0")
        .arg("--")
        .arg(source)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| FpcalcFailure::EngineUnavailable)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(FpcalcFailure::EngineUnavailable)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(FpcalcFailure::EngineUnavailable)?;
    let stdout_reader = read_bounded(stdout, FPCALC_STDOUT_LIMIT);
    let stderr_reader = read_bounded(stderr, FPCALC_STDERR_LIMIT);
    let status = wait_for_child(&mut child, Duration::from_secs(FPCALC_TIMEOUT_SECONDS))?;
    let stdout = stdout_reader
        .join()
        .ok()
        .and_then(|result| result.ok())
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    let stderr = stderr_reader
        .join()
        .ok()
        .and_then(|result| result.ok())
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    if stdout.exceeded || stderr.exceeded {
        return Err(FpcalcFailure::ProcessingFailed);
    }
    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr.bytes).to_ascii_lowercase();
        return if stderr_text.contains("unsupported")
            || stderr_text.contains("unknown format")
            || stderr_text.contains("invalid data")
        {
            Err(FpcalcFailure::UnsupportedFormat)
        } else {
            Err(FpcalcFailure::ProcessingFailed)
        };
    }
    parse_fpcalc_output(&stdout.bytes)
}

pub(super) fn parse_fpcalc_output(
    bytes: &[u8],
) -> std::result::Result<FpcalcOutput, FpcalcFailure> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| FpcalcFailure::ProcessingFailed)?;
    let fingerprint = value
        .get("fingerprint")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| value.len() <= FPCALC_STDOUT_LIMIT)
        .ok_or(FpcalcFailure::ProcessingFailed)?
        .to_owned();
    let duration_milliseconds = value
        .get("duration")
        .and_then(Value::as_f64)
        .and_then(seconds_to_milliseconds)
        .filter(|duration| *duration > 0)
        .ok_or(FpcalcFailure::ProcessingFailed)?;
    Ok(FpcalcOutput {
        fingerprint,
        duration_milliseconds,
    })
}

pub(super) fn seconds_to_milliseconds(seconds: f64) -> Option<u64> {
    if !seconds.is_finite() || seconds < 0.0 || seconds > (u64::MAX as f64 / 1000.0) {
        return None;
    }
    Some((seconds * 1000.0).round() as u64)
}

#[derive(Debug)]
pub(super) struct BoundedRead {
    bytes: Vec<u8>,
    exceeded: bool,
}

pub(super) fn read_bounded<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> thread::JoinHandle<io::Result<BoundedRead>> {
    thread::spawn(move || {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 8192];
        let mut exceeded = false;
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            let remaining = limit.saturating_sub(retained.len());
            if count > remaining {
                retained.extend_from_slice(&buffer[..remaining]);
                exceeded = true;
            } else {
                retained.extend_from_slice(&buffer[..count]);
            }
        }
        Ok(BoundedRead {
            bytes: retained,
            exceeded,
        })
    })
}

pub(super) fn wait_for_child(
    child: &mut Child,
    timeout: Duration,
) -> std::result::Result<ExitStatus, FpcalcFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| FpcalcFailure::ProcessingFailed)?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(FpcalcFailure::ProcessingFailed);
        }
        thread::sleep(Duration::from_millis(20));
    }
}
