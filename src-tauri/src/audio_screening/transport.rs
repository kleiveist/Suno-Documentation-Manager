use super::*;

pub(super) fn execute_ureq_request(
    method: &str,
    request: AcrCloudRequest,
) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
    let timeout = Duration::from_secs(u64::from(request.timeout_seconds.max(1)));
    // Redirects are intentionally disabled.  A validated ACRCloud host is the
    // sole destination of this provider adapter.
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .redirects(0)
        .build();
    let mut outgoing = match method {
        "GET" => agent.get(&request.url),
        _ => agent.post(&request.url),
    };
    for (name, value) in request.headers {
        outgoing = outgoing.set(&name, &value);
    }
    let response = if method == "GET" {
        outgoing.call()
    } else {
        outgoing.send_bytes(&request.body)
    };
    match response {
        Ok(response) => read_acrcloud_response(response),
        Err(ureq::Error::Status(_, response)) => read_acrcloud_response(response),
        Err(ureq::Error::Transport(_)) => Err(ProviderFailure {
            status: AudioScreeningStatus::ProviderUnavailable,
            message: "ACRCloud could not be reached.",
        }),
    }
}

pub(super) fn read_acrcloud_response(
    response: ureq::Response,
) -> std::result::Result<AcrCloudResponse, ProviderFailure> {
    let status = response.status();
    let mut reader = response
        .into_reader()
        .take((MAX_PROVIDER_RESPONSE_BYTES + 1) as u64);
    let mut body = Vec::new();
    reader.read_to_end(&mut body).map_err(|_| ProviderFailure {
        status: AudioScreeningStatus::ProviderUnavailable,
        message: "The ACRCloud response could not be read.",
    })?;
    if body.len() > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: AudioScreeningStatus::ProcessingFailed,
            message: "The ACRCloud response exceeds the supported size limit.",
        });
    }
    Ok(AcrCloudResponse { status, body })
}
