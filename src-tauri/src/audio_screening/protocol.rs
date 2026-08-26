use super::*;

pub(super) fn build_acrcloud_request(
    settings: &AudioScreeningSettings,
    access_key: &str,
    access_secret: &str,
    timestamp: &str,
    sample: &[u8],
) -> std::result::Result<(AcrCloudRequest, String), ProviderFailure> {
    if sample.is_empty() || sample.len() > MAX_SAMPLE_AUDIO_BYTES as usize {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The bounded ACRCloud sample is outside the supported size limit.",
        });
    }
    if access_key.trim().is_empty()
        || access_secret.trim().is_empty()
        || access_key.contains(['\r', '\n'])
        || access_secret.contains(['\r', '\n'])
    {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud credentials are invalid.",
        });
    }
    let url = acrcloud_identify_url(settings)?;
    let signature = acrcloud_signature(access_key, access_secret, timestamp)?;
    let boundary = format!("----SunoDMAcrCloud{}", Uuid::new_v4().simple());
    let body = build_acrcloud_multipart(&boundary, access_key, timestamp, &signature, sample);
    Ok((
        AcrCloudRequest {
            url,
            headers: vec![(
                "Content-Type".into(),
                format!("multipart/form-data; boundary={boundary}"),
            )],
            body,
            timeout_seconds: settings.timeout_seconds,
        },
        signature,
    ))
}

/// ACRCloud Identification API v1 signature:
/// `POST\n/v1/identify\n{access_key}\naudio\n1\n{timestamp}` HMAC-SHA1,
/// encoded with standard Base64.
pub(super) fn acrcloud_signature(
    access_key: &str,
    access_secret: &str,
    timestamp: &str,
) -> std::result::Result<String, ProviderFailure> {
    let canonical = format!("POST\n/v1/identify\n{access_key}\naudio\n1\n{timestamp}");
    let mut mac =
        HmacSha1::new_from_slice(access_secret.as_bytes()).map_err(|_| ProviderFailure {
            status: AudioScreeningStatus::ConfigurationInvalid,
            message: "ACRCloud credentials are invalid.",
        })?;
    mac.update(canonical.as_bytes());
    Ok(BASE64_STANDARD.encode(mac.finalize().into_bytes()))
}

pub(super) fn build_acrcloud_multipart(
    boundary: &str,
    access_key: &str,
    timestamp: &str,
    signature: &str,
    sample: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(sample.len().saturating_add(1024));
    multipart_text_part(&mut body, boundary, "access_key", access_key);
    multipart_text_part(&mut body, boundary, "data_type", "audio");
    multipart_text_part(&mut body, boundary, "signature_version", "1");
    multipart_text_part(&mut body, boundary, "signature", signature);
    multipart_text_part(&mut body, boundary, "timestamp", timestamp);
    multipart_text_part(
        &mut body,
        boundary,
        "sample_bytes",
        &sample.len().to_string(),
    );
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"sample\"; filename=\"sunodm-screening.wav\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(sample);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

pub(super) fn multipart_text_part(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
}

pub(super) struct ParsedAcrCloudResponse {
    pub(super) status: AudioScreeningStatus,
    pub(super) message: &'static str,
    pub(super) provider_status_code: Option<i64>,
    pub(super) provider_status_message: Option<String>,
    pub(super) provider_api_version: Option<String>,
    pub(super) matches: Vec<AudioScreeningMatch>,
    pub(super) raw_response: Option<SanitizedProviderResponse>,
}

pub(super) fn parse_acrcloud_response(
    http_status: u16,
    body: &[u8],
    request_sensitive_values: &RequestSensitiveValues<'_>,
) -> std::result::Result<ParsedAcrCloudResponse, ProviderFailure> {
    if body.len() > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The ACRCloud response exceeds the supported size limit.",
        });
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| ProviderFailure {
        status: status_for_http_error(http_status),
        message: message_for_http_error(http_status),
    })?;
    // A provider response is archival material, so fail closed before even
    // parsing a match: it may not contain a credential-shaped field *or* any
    // exact value sent in this request.  `serde_json` has already decoded
    // normal JSON escapes; the helper additionally walks embedded escaped
    // JSON strings so an echo cannot hide behind a benign key or `\\u` form.
    if response_contains_sensitive_content(&value, request_sensitive_values) {
        return Ok(ParsedAcrCloudResponse {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The provider response contained unsafe credential-like fields and was not documented.",
            provider_status_code: None,
            provider_status_message: None,
            provider_api_version: None,
            matches: Vec::new(),
            raw_response: None,
        });
    }
    let raw_response = SanitizedProviderResponse(body.to_vec());

    let status_value = value.get("status").and_then(Value::as_object);
    let (provider_code, provider_status_message, provider_api_version) =
        provider_response_metadata(status_value);

    if !(200..300).contains(&http_status) {
        return Ok(ParsedAcrCloudResponse {
            status: status_for_http_error(http_status),
            message: message_for_http_error(http_status),
            provider_status_code: provider_code,
            provider_status_message,
            provider_api_version,
            matches: Vec::new(),
            raw_response: Some(raw_response),
        });
    }
    status_value.ok_or(ProviderFailure {
        status: AudioScreeningStatus::ProcessingFailed,
        message: "ACRCloud returned an unexpected response.",
    })?;
    let provider_message = provider_status_message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if provider_code == Some(0) {
        return Ok(parse_successful_acrcloud_response(
            &value,
            provider_code,
            provider_status_message,
            provider_api_version,
            raw_response,
        ));
    }
    if provider_message.contains("no result") || provider_message.contains("no match") {
        return Ok(ParsedAcrCloudResponse {
            status: AudioScreeningStatus::NoMatchDetected,
            message: "ACRCloud returned no catalog match for the submitted audio sample.",
            provider_status_code: provider_code,
            provider_status_message,
            provider_api_version,
            matches: Vec::new(),
            raw_response: Some(raw_response),
        });
    }
    let status = provider_error_status(&provider_message);
    Ok(ParsedAcrCloudResponse {
        status,
        message: provider_error_message(status),
        provider_status_code: provider_code,
        provider_status_message,
        provider_api_version,
        matches: Vec::new(),
        raw_response: Some(raw_response),
    })
}

fn provider_response_metadata(
    status_value: Option<&serde_json::Map<String, Value>>,
) -> (Option<i64>, Option<String>, Option<String>) {
    let provider_code = status_value.and_then(|status| json_integer(status.get("code")));
    let provider_status_message = status_value
        .and_then(|status| status.get("msg"))
        .and_then(provider_text);
    let provider_api_version = status_value
        .and_then(|status| status.get("version"))
        .and_then(provider_text);
    (provider_code, provider_status_message, provider_api_version)
}

fn parse_successful_acrcloud_response(
    value: &Value,
    provider_code: Option<i64>,
    provider_status_message: Option<String>,
    provider_api_version: Option<String>,
    raw_response: SanitizedProviderResponse,
) -> ParsedAcrCloudResponse {
    let matches = match parse_matches(value) {
        Ok(matches) => matches,
        Err(failure) => {
            return ParsedAcrCloudResponse {
                status: failure.status,
                message: failure.message,
                provider_status_code: provider_code,
                provider_status_message: provider_status_message.clone(),
                provider_api_version: provider_api_version.clone(),
                matches: Vec::new(),
                raw_response: None,
            };
        }
    };
    ParsedAcrCloudResponse {
        status: if matches.is_empty() {
            AudioScreeningStatus::NoMatchDetected
        } else {
            AudioScreeningStatus::MatchDetected
        },
        message: if matches.is_empty() {
            "ACRCloud returned no catalog match for the submitted audio sample."
        } else {
            "ACRCloud returned one or more catalog matches for the submitted audio sample."
        },
        provider_status_code: provider_code,
        provider_status_message,
        provider_api_version,
        matches,
        raw_response: Some(raw_response),
    }
}

fn provider_error_status(provider_message: &str) -> AudioScreeningStatus {
    if provider_message.contains("access key")
        || provider_message.contains("signature")
        || provider_message.contains("authentication")
        || provider_message.contains("authorization")
    {
        AudioScreeningStatus::AuthenticationFailed
    } else if provider_message.contains("busy")
        || provider_message.contains("unavailable")
        || provider_message.contains("limit")
        || provider_message.contains("timeout")
    {
        AudioScreeningStatus::ProviderUnavailable
    } else {
        AudioScreeningStatus::ProcessingFailed
    }
}

fn provider_error_message(status: AudioScreeningStatus) -> &'static str {
    match status {
        AudioScreeningStatus::AuthenticationFailed => {
            "ACRCloud did not accept the configured credentials."
        }
        AudioScreeningStatus::ProviderUnavailable => {
            "ACRCloud is temporarily unavailable for this screening request."
        }
        _ => "ACRCloud returned an unexpected response.",
    }
}

pub(super) fn status_for_http_error(status: u16) -> AudioScreeningStatus {
    match status {
        401 | 403 => AudioScreeningStatus::AuthenticationFailed,
        408 | 425 | 429 | 500..=599 => AudioScreeningStatus::ProviderUnavailable,
        _ => AudioScreeningStatus::ProcessingFailed,
    }
}

pub(super) fn message_for_http_error(status: u16) -> &'static str {
    match status_for_http_error(status) {
        AudioScreeningStatus::AuthenticationFailed => {
            "ACRCloud did not accept the configured credentials."
        }
        AudioScreeningStatus::ProviderUnavailable => {
            "ACRCloud is temporarily unavailable for this screening request."
        }
        _ => "ACRCloud returned an unexpected HTTP response.",
    }
}

pub(super) fn json_integer(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64).or_else(|| {
        value
            .and_then(Value::as_str)
            .and_then(|value| value.trim().parse::<i64>().ok())
    })
}

pub(super) fn parse_matches(
    value: &Value,
) -> std::result::Result<Vec<AudioScreeningMatch>, ProviderFailure> {
    let Some(music) = value
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("music"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut matches = Vec::new();
    for item in music.iter().take(MAX_PROVIDER_MATCHES) {
        if let Some(parsed) = parse_match(item)? {
            matches.push(parsed);
        }
    }
    Ok(matches)
}

pub(super) fn parse_match(
    value: &Value,
) -> std::result::Result<Option<AudioScreeningMatch>, ProviderFailure> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    // Validate score before deciding whether the object has enough display
    // data to become a match.  A malformed entry without a title must not
    // bypass the non-finite-value barrier and leave raw JSON to be archived.
    let score = object.get("score").and_then(Value::as_f64).or_else(|| {
        object
            .get("score")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<f64>().ok())
    });
    if score.is_some_and(|score| !score.is_finite()) {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "ACRCloud returned a non-finite provider score.",
        });
    }
    let Some(title) = object.get("title").and_then(provider_text) else {
        return Ok(None);
    };
    let artists = object
        .get("artists")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|artist| artist.get("name").and_then(provider_text))
        .collect();
    let album = object
        .get("album")
        .and_then(Value::as_object)
        .and_then(|album| album.get("name"))
        .and_then(provider_text);
    let external_ids = object.get("external_ids").and_then(Value::as_object);
    let isrc = external_ids
        .and_then(|identifiers| identifiers.get("isrc"))
        .and_then(provider_text);
    let acrid = object.get("acrid").and_then(provider_text);
    Ok(Some(AudioScreeningMatch {
        title,
        artists,
        album,
        isrc,
        acrid,
        score,
    }))
}

pub(super) fn provider_text(value: &Value) -> Option<String> {
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    let sanitized = text
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let sanitized = sanitized.trim();
    (!sanitized.is_empty()).then(|| truncate_utf8(sanitized, MAX_PROVIDER_TEXT_BYTES))
}

pub(super) fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value[..boundary].to_owned()
}

pub(super) fn response_contains_sensitive_content(
    value: &Value,
    request_sensitive_values: &RequestSensitiveValues<'_>,
) -> bool {
    response_contains_sensitive_content_at_depth(value, request_sensitive_values, 0)
}

pub(super) fn response_contains_sensitive_content_at_depth(
    value: &Value,
    request_sensitive_values: &RequestSensitiveValues<'_>,
    depth: usize,
) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            credential_like_field_name(key)
                || response_string_contains_sensitive_content(key, request_sensitive_values, depth)
                || response_contains_sensitive_content_at_depth(
                    value,
                    request_sensitive_values,
                    depth,
                )
        }),
        Value::Array(values) => values.iter().any(|value| {
            response_contains_sensitive_content_at_depth(value, request_sensitive_values, depth)
        }),
        Value::String(text) => {
            response_string_contains_sensitive_content(text, request_sensitive_values, depth)
        }
        Value::Number(number) => request_sensitive_values.occurs_in(&number.to_string()),
        Value::Bool(value) => {
            request_sensitive_values.occurs_in(if *value { "true" } else { "false" })
        }
        Value::Null => false,
    }
}

/// Scans normal decoded strings, JSON nested inside a string, and a bounded
/// number of JSON-escape layers.  The outer `serde_json` parse handles the
/// usual `"\\u0061"` case; the extra bounded walk covers a provider that
/// embeds a second JSON document as a string.
pub(super) fn response_string_contains_sensitive_content(
    text: &str,
    request_sensitive_values: &RequestSensitiveValues<'_>,
    depth: usize,
) -> bool {
    if request_sensitive_values.occurs_in(text) {
        return true;
    }
    if depth >= MAX_PROVIDER_RESPONSE_STRING_DECODE_DEPTH {
        return false;
    }

    if let Ok(nested) = serde_json::from_str::<Value>(text) {
        let differs_from_same_string = !matches!(&nested, Value::String(value) if value == text);
        if differs_from_same_string
            && response_contains_sensitive_content_at_depth(
                &nested,
                request_sensitive_values,
                depth + 1,
            )
        {
            return true;
        }
    }

    // If this is a literal JSON-escape sequence left after one encoded layer
    // (for example `\\u0061ccess-key`), decode it once and scan again.  A raw
    // quote/control character simply makes this speculative decode fail.
    if text.contains('\\') {
        let encoded = format!("\"{text}\"");
        if let Ok(decoded) = serde_json::from_str::<String>(&encoded) {
            if decoded != text
                && response_string_contains_sensitive_content(
                    &decoded,
                    request_sensitive_values,
                    depth + 1,
                )
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn response_contains_credential_like_field(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            credential_like_field_name(key) || response_contains_credential_like_field(value)
        }),
        Value::Array(values) => values.iter().any(response_contains_credential_like_field),
        _ => false,
    }
}

pub(super) fn credential_like_field_name(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "accesskey",
        "accesssecret",
        "apikey",
        "privatekey",
        "secretkey",
        "signature",
        "authorization",
        "password",
        "token",
        "secret",
        "clientsecret",
        "credential",
        "bearer",
        "session",
        "cookie",
        "refresh",
        "idtoken",
        "csrf",
        "jwt",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}
