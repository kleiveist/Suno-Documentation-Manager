use super::*;

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn normalized_unsigned_hex(value: &[u8]) -> String {
    let first_nonzero = value
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(value.len().saturating_sub(1));
    value[first_nonzero..]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn der_sequence(elements: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for element in elements {
        content.extend_from_slice(element);
    }
    der_tlv(0x30, &content)
}

pub(super) fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
    result.extend_from_slice(&der_length(content.len()));
    result.extend_from_slice(content);
    result
}

pub(super) fn der_length(length: usize) -> Vec<u8> {
    if length < 128 {
        return vec![length as u8];
    }
    let mut bytes = length.to_be_bytes().to_vec();
    while bytes.first() == Some(&0) {
        bytes.remove(0);
    }
    let mut result = vec![0x80 | bytes.len() as u8];
    result.extend_from_slice(&bytes);
    result
}

pub(super) fn der_integer_unsigned(value: &[u8]) -> Vec<u8> {
    let mut content = value.to_vec();
    while content.len() > 1 && content.first() == Some(&0) {
        content.remove(0);
    }
    if content.first().is_some_and(|byte| byte & 0x80 != 0) {
        content.insert(0, 0);
    }
    der_tlv(0x02, &content)
}

pub(super) fn der_oid(value: &str) -> std::result::Result<Vec<u8>, String> {
    let arcs = value
        .split('.')
        .map(|part| {
            part.parse::<u64>()
                .map_err(|_| "Invalid object identifier.".into())
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    if arcs.len() < 2 || arcs[0] > 2 || (arcs[0] < 2 && arcs[1] > 39) {
        return Err("Invalid object identifier.".into());
    }
    let mut encoded = encode_base128(arcs[0] * 40 + arcs[1]);
    for arc in arcs.iter().skip(2) {
        encoded.extend_from_slice(&encode_base128(*arc));
    }
    Ok(der_tlv(0x06, &encoded))
}

pub(super) fn encode_base128(mut value: u64) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push(0x80 | (value & 0x7f) as u8);
        value >>= 7;
    }
    bytes.reverse();
    bytes
}

#[derive(Clone, Copy)]
pub(super) struct DerElement<'a> {
    pub(super) tag: u8,
    pub(super) content: &'a [u8],
}

pub(super) fn der_element(bytes: &[u8]) -> std::result::Result<(DerElement<'_>, &[u8]), String> {
    if bytes.len() < 2 {
        return Err("DER element is truncated.".into());
    }
    let tag = bytes[0];
    let first_length = bytes[1];
    let (length, header_length) = if first_length & 0x80 == 0 {
        (first_length as usize, 2)
    } else {
        let count = (first_length & 0x7f) as usize;
        if count == 0 || count > std::mem::size_of::<usize>() || bytes.len() < 2 + count {
            return Err("DER length is invalid.".into());
        }
        let mut length = 0_usize;
        for byte in &bytes[2..2 + count] {
            length = length
                .checked_mul(256)
                .and_then(|value| value.checked_add(*byte as usize))
                .ok_or_else(|| "DER length overflows.".to_owned())?;
        }
        if length < 128 {
            return Err("DER length is not canonical.".into());
        }
        (length, 2 + count)
    };
    let end = header_length
        .checked_add(length)
        .ok_or_else(|| "DER element length overflows.".to_owned())?;
    if end > bytes.len() {
        return Err("DER element is truncated.".into());
    }
    Ok((
        DerElement {
            tag,
            content: &bytes[header_length..end],
        },
        &bytes[end..],
    ))
}

pub(super) fn der_elements(mut bytes: &[u8]) -> std::result::Result<Vec<DerElement<'_>>, String> {
    let mut output = Vec::new();
    while !bytes.is_empty() {
        let (element, remaining) = der_element(bytes)?;
        output.push(element);
        bytes = remaining;
    }
    Ok(output)
}

pub(super) fn der_sequence_content(bytes: &[u8]) -> std::result::Result<&[u8], String> {
    let (element, remaining) = der_element(bytes)?;
    if element.tag != 0x30 || !remaining.is_empty() {
        return Err("Expected one DER SEQUENCE.".into());
    }
    Ok(element.content)
}

pub(super) fn der_oid_text(element: DerElement<'_>) -> std::result::Result<String, String> {
    if element.tag != 0x06 || element.content.is_empty() {
        return Err("Expected an object identifier.".into());
    }
    let mut bytes = element.content.iter().copied();
    let first = decode_base128(&mut bytes)?;
    let (first_arc, second_arc) = if first < 40 {
        (0, first)
    } else if first < 80 {
        (1, first - 40)
    } else {
        (2, first - 80)
    };
    let mut arcs = vec![first_arc.to_string(), second_arc.to_string()];
    while bytes.clone().next().is_some() {
        arcs.push(decode_base128(&mut bytes)?.to_string());
    }
    Ok(arcs.join("."))
}

pub(super) fn decode_base128(
    iterator: &mut std::iter::Copied<std::slice::Iter<'_, u8>>,
) -> std::result::Result<u64, String> {
    let mut value = 0_u64;
    let mut count = 0_u8;
    loop {
        let byte = iterator
            .next()
            .ok_or_else(|| "Object identifier is truncated.".to_owned())?;
        value = value
            .checked_mul(128)
            .and_then(|current| current.checked_add((byte & 0x7f) as u64))
            .ok_or_else(|| "Object identifier is too large.".to_owned())?;
        count = count.saturating_add(1);
        if byte & 0x80 == 0 {
            break;
        }
        if count > 10 {
            return Err("Object identifier is too large.".into());
        }
    }
    Ok(value)
}

pub(super) fn der_integer_hex(element: DerElement<'_>) -> std::result::Result<String, String> {
    if element.tag != 0x02 || element.content.is_empty() || element.content[0] & 0x80 != 0 {
        return Err("Expected a non-negative INTEGER.".into());
    }
    Ok(element
        .content
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn der_integer_status(element: DerElement<'_>) -> std::result::Result<u64, String> {
    let raw = der_integer_hex(element)?;
    let normalized = raw.trim_start_matches('0');
    if normalized.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(normalized, 16)
        .map_err(|_| "Timestamp response status is too large.".into())
}

#[derive(Default)]
pub(super) struct ParsedRfc3161Response {
    pub(super) timestamp_value: String,
    pub(super) serial_number: String,
    pub(super) policy_oid: String,
    pub(super) nonce_hex: String,
    pub(super) digest_match: Option<bool>,
    pub(super) nonce_match: Option<bool>,
    pub(super) policy_match: Option<bool>,
    pub(super) error: Option<String>,
}

pub(super) fn parse_rfc3161_response(
    bytes: &[u8],
    expected_digest: &str,
    expected_nonce_hex: &str,
    expected_policy_oid: Option<&str>,
) -> ParsedRfc3161Response {
    match parse_rfc3161_response_inner(
        bytes,
        expected_digest,
        expected_nonce_hex,
        expected_policy_oid,
    ) {
        Ok(value) => value,
        Err(error) => ParsedRfc3161Response {
            error: Some(error),
            ..Default::default()
        },
    }
}

pub(super) fn parse_rfc3161_response_inner(
    bytes: &[u8],
    expected_digest: &str,
    expected_nonce_hex: &str,
    expected_policy_oid: Option<&str>,
) -> std::result::Result<ParsedRfc3161Response, String> {
    let token = successful_rfc3161_token(bytes)?;
    let tst_info = cms_tst_info(token)?;
    parse_tst_info(
        tst_info,
        expected_digest,
        expected_nonce_hex,
        expected_policy_oid,
    )
}

pub(super) fn successful_rfc3161_token(
    bytes: &[u8],
) -> std::result::Result<DerElement<'_>, String> {
    let response = der_elements(der_sequence_content(bytes)?)?;
    let status_info = response
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 response has no status information.".to_owned())?;
    if status_info.tag != 0x30 {
        return Err("RFC 3161 status information is invalid.".into());
    }
    let status = der_elements(status_info.content)?
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 response status is missing.".to_owned())?;
    let status = der_integer_status(status)?;
    if !matches!(status, 0 | 1) {
        return Err(format!(
            "Timestamp authority rejected the request (RFC 3161 status {status})."
        ));
    }
    response
        .get(1)
        .copied()
        .ok_or_else(|| "RFC 3161 response has no timestamp token.".to_owned())
}

pub(super) fn cms_signed_data(
    token: DerElement<'_>,
) -> std::result::Result<DerElement<'_>, String> {
    let token_contents = der_elements(token.content)?;
    if token.tag != 0x30 || token_contents.len() < 2 {
        return Err("RFC 3161 timestamp token is invalid.".into());
    }
    if der_oid_text(token_contents[0])? != "1.2.840.113549.1.7.2" {
        return Err("RFC 3161 timestamp token is not CMS SignedData.".into());
    }
    let signed_wrapper = token_contents[1];
    if signed_wrapper.tag != 0xa0 {
        return Err("RFC 3161 CMS SignedData wrapper is missing.".into());
    }
    let (signed_data, remaining) = der_element(signed_wrapper.content)?;
    if signed_data.tag != 0x30 || !remaining.is_empty() {
        return Err("RFC 3161 CMS SignedData is invalid.".into());
    }
    Ok(signed_data)
}

pub(super) fn cms_tst_info(token: DerElement<'_>) -> std::result::Result<&[u8], String> {
    let signed_data = cms_signed_data(token)?;
    let signed_values = der_elements(signed_data.content)?;
    let encapsulated = signed_values
        .get(2)
        .copied()
        .ok_or_else(|| "RFC 3161 CMS encapsulated content is missing.".to_owned())?;
    if encapsulated.tag != 0x30 {
        return Err("RFC 3161 CMS encapsulated content is invalid.".into());
    }
    let encapsulated_values = der_elements(encapsulated.content)?;
    if encapsulated_values.len() < 2
        || der_oid_text(encapsulated_values[0])? != "1.2.840.113549.1.9.16.1.4"
        || encapsulated_values[1].tag != 0xa0
    {
        return Err("RFC 3161 CMS payload is not TSTInfo.".into());
    }
    let (tst_octet, remaining) = der_element(encapsulated_values[1].content)?;
    if tst_octet.tag != 0x04 || !remaining.is_empty() {
        return Err("RFC 3161 TSTInfo payload is invalid.".into());
    }
    Ok(tst_octet.content)
}

pub(super) fn tst_info_values(bytes: &[u8]) -> std::result::Result<Vec<DerElement<'_>>, String> {
    let values = der_elements(der_sequence_content(bytes)?)?;
    if values.len() < 5 || values[0].tag != 0x02 || values[1].tag != 0x06 || values[2].tag != 0x30 {
        return Err("RFC 3161 TSTInfo structure is invalid.".into());
    }
    if normalized_unsigned_hex(values[0].content) != "01" {
        return Err("RFC 3161 TSTInfo version must be 1.".into());
    }
    Ok(values)
}

pub(super) fn parse_message_imprint(
    imprint: DerElement<'_>,
) -> std::result::Result<String, String> {
    let imprint_values = der_elements(imprint.content)?;
    if imprint_values.len() != 2 || imprint_values[0].tag != 0x30 || imprint_values[1].tag != 0x04 {
        return Err("RFC 3161 message imprint is invalid.".into());
    }
    let algorithm = der_elements(imprint_values[0].content)?
        .first()
        .copied()
        .ok_or_else(|| "RFC 3161 message imprint algorithm is missing.".to_owned())?;
    if der_oid_text(algorithm)? != "2.16.840.1.101.3.4.2.1" {
        return Err("RFC 3161 response does not use SHA-256 message imprint.".into());
    }
    Ok(imprint_values[1]
        .content
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

pub(super) fn parse_tst_info(
    bytes: &[u8],
    expected_digest: &str,
    expected_nonce_hex: &str,
    expected_policy_oid: Option<&str>,
) -> std::result::Result<ParsedRfc3161Response, String> {
    let values = tst_info_values(bytes)?;
    let policy_oid = der_oid_text(values[1])?;
    let returned_digest = parse_message_imprint(values[2])?;
    let serial_number = der_integer_hex(values[3])?;
    if values[4].tag != 0x18 {
        return Err("RFC 3161 generation time is missing.".into());
    }
    let timestamp_value = generalized_time_to_rfc3339(values[4].content)?;
    let (nonce_hex, nonce_match) = parse_tst_nonce(&values[5..], expected_nonce_hex)?;
    let policy_match = expected_policy_oid.map(|expected| policy_oid == expected);
    Ok(ParsedRfc3161Response {
        timestamp_value,
        serial_number,
        policy_oid,
        nonce_hex,
        digest_match: Some(returned_digest.eq_ignore_ascii_case(expected_digest)),
        nonce_match,
        policy_match,
        error: None,
    })
}

pub(super) fn parse_tst_nonce(
    optional_values: &[DerElement<'_>],
    expected_nonce_hex: &str,
) -> std::result::Result<(String, Option<bool>), String> {
    let returned_nonces = optional_values
        .iter()
        .filter(|value| value.tag == 0x02)
        .copied()
        .collect::<Vec<_>>();
    if returned_nonces.len() > 1 {
        return Err("RFC 3161 TSTInfo contains more than one nonce.".into());
    }
    let nonce_hex = returned_nonces
        .first()
        .map(|value| {
            der_integer_hex(*value)?;
            Ok::<String, String>(normalized_unsigned_hex(value.content))
        })
        .transpose()?;
    let nonce_match = Some(
        nonce_hex
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(expected_nonce_hex)),
    );
    Ok((nonce_hex.unwrap_or_default(), nonce_match))
}

pub(super) fn generalized_time_to_rfc3339(bytes: &[u8]) -> std::result::Result<String, String> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| "RFC 3161 generation time is not UTF-8.".to_owned())?;
    if let Ok(value) = chrono::DateTime::parse_from_str(value, "%Y%m%d%H%M%SZ") {
        return Ok(value.to_rfc3339());
    }
    // GeneralizedTime may include fractional seconds. Preserve a valid UTC
    // timestamp in RFC 3339 form without accepting a local/ambiguous zone.
    if let Some(value) = value.strip_suffix('Z') {
        let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
        let datetime = chrono::NaiveDateTime::parse_from_str(whole, "%Y%m%d%H%M%S")
            .map_err(|_| "RFC 3161 generation time is invalid.".to_owned())?;
        let fraction = fraction.trim_end_matches('0');
        let base = datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        return Ok(if fraction.is_empty() {
            format!("{base}Z")
        } else {
            format!("{base}.{fraction}Z")
        });
    }
    Err("RFC 3161 generation time must use UTC (Z).".into())
}
