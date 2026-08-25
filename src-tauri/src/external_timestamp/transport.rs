use super::*;

#[derive(Debug, Clone)]
pub(super) struct HttpRequest {
    pub(super) url: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
    pub(super) timeout_seconds: u32,
}

#[derive(Debug, Clone)]
pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) body: Vec<u8>,
}

/// Kept behind a narrow interface so provider parsing and attachment behavior
/// can be tested with deterministic byte-for-byte fake responses without a
/// network connection or a real TSA account.
pub(super) trait TimestampHttpTransport {
    fn post(&self, request: HttpRequest) -> std::result::Result<HttpResponse, ProviderFailure>;
}

pub(super) struct UreqTimestampHttpTransport;

impl TimestampHttpTransport for UreqTimestampHttpTransport {
    fn post(&self, request: HttpRequest) -> std::result::Result<HttpResponse, ProviderFailure> {
        let timeout = Duration::from_secs(u64::from(request.timeout_seconds.max(1)));
        let agent = ureq::AgentBuilder::new().timeout(timeout).build();
        let mut outgoing = agent.post(&request.url);
        for (name, value) in request.headers {
            outgoing = outgoing.set(&name, &value);
        }
        match outgoing.send_bytes(&request.body) {
            Ok(response) => read_http_response(response),
            Err(ureq::Error::Status(_, response)) => read_http_response(response),
            Err(ureq::Error::Transport(_)) => Err(ProviderFailure {
                status: ExternalTimestampStatus::ProviderUnavailable,
                message: "Timestamp provider could not be reached.".into(),
            }),
        }
    }
}

pub(super) fn read_http_response(
    response: ureq::Response,
) -> std::result::Result<HttpResponse, ProviderFailure> {
    let status = response.status();
    let mut reader = response.into_reader().take(MAX_PROVIDER_RESPONSE_BYTES + 1);
    let mut body = Vec::new();
    reader.read_to_end(&mut body).map_err(|_| ProviderFailure {
        status: ExternalTimestampStatus::ProviderUnavailable,
        message: "Timestamp provider response could not be read.".into(),
    })?;
    if body.len() as u64 > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderFailure {
            status: ExternalTimestampStatus::UnsupportedResponse,
            message: "Timestamp provider response exceeds the supported size limit.".into(),
        });
    }
    Ok(HttpResponse { status, body })
}

pub(super) trait TimestampProviderAdapter {
    fn display_name(&self, settings: &TimestampSettings) -> String;
    fn capabilities(&self) -> TimestampProviderCapabilities;
    fn request(
        &self,
        settings: &TimestampSettings,
        secret: Option<&str>,
        digest: &str,
        artifact_bytes: Option<&[u8]>,
        transport: &dyn TimestampHttpTransport,
    ) -> std::result::Result<ProviderTimestampResponse, ProviderFailure>;
}

pub(super) struct Rfc3161TimestampAdapter {
    pub(super) provider: &'static str,
    pub(super) endpoint: &'static str,
    pub(super) adapter: &'static str,
    pub(super) trust_root_available: bool,
}

pub(super) struct OpenTimestampsAdapter;

pub(super) struct CustomRfc3161Adapter;
