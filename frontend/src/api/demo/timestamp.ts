import { emptyTimestampSettings } from "../../domain/types";
import type {
  ExternalTimestampRecord,
  ExternalTimestampSummary,
  FinalizationAnchor,
  TimestampProviderCapabilities,
  TimestampProviderKind,
  TimestampProviderMetadata,
  TimestampSettings,
  TrackDetail
} from "../../domain/types";
import { clone, now } from "./helpers";

export function timestampProviderLabel(provider: TimestampProviderKind): string {
  switch (provider) {
    case "free_tsa":
      return "FreeTSA";
    case "open_timestamps":
      return "OpenTimestamps";
    case "sigstore_public_tsa":
      return "Sigstore Public TSA";
    case "custom_rfc3161":
      return "Custom RFC 3161";
    default:
      return "Disabled";
  }
}

export function timestampProviderUsesRfc3161(provider: TimestampProviderKind): boolean {
  return provider === "free_tsa" || provider === "sigstore_public_tsa" || provider === "custom_rfc3161";
}

export function timestampRecordUsesOpenTimestamps(record: ExternalTimestampRecord | undefined): boolean {
  if (!record) return false;
  return (
    record.providerMetadata?.adapter.toLowerCase().includes("open_timestamps") === true ||
    record.providerMetadata?.protocol.toLowerCase().includes("opentimestamps") === true ||
    record.provider.toLowerCase() === "opentimestamps"
  );
}

export function notRecordedExternalTimestampSummary(): ExternalTimestampSummary {
  return {
    status: "not_recorded",
    message: "External timestamp evidence is optional and is not required for technical finalization.",
    provider: ""
  };
}

export function timestampProviderCapabilities(provider: TimestampProviderKind): TimestampProviderCapabilities {
  const rfc3161 = provider === "free_tsa" || provider === "sigstore_public_tsa" || provider === "custom_rfc3161";
  return {
    rfc3161,
    openTimestamps: provider === "open_timestamps",
    requiresAuthentication: provider === "custom_rfc3161",
    supportsSha256: provider !== "disabled",
    supportsOfflineVerification: provider === "open_timestamps" || rfc3161,
    returnsSignedTimestamp: rfc3161,
    externalTrustRootAvailable: false,
    qualificationStatus: "not_checked"
  };
}

export function configuredTimestampProviderStatus(
  settings: TimestampSettings,
  timestampSecretConfigured = false
): Pick<TimestampSettings, "status" | "statusMessage"> {
  if (!settings.enabled || settings.provider === "disabled") {
    return { status: "disabled", statusMessage: "External timestamp service is disabled." };
  }
  if (settings.provider === "open_timestamps") {
    return {
      status: "ready",
      statusMessage:
        "OpenTimestamps calendar service is ready. This is not RFC 3161; an initial proof remains ATTACHED pending OpenTimestamps verification or upgrade."
    };
  }
  if (settings.provider !== "custom_rfc3161") {
    if (!settings.custom.caCertificatePath.trim()) {
      return {
        status: "verification_configuration_incomplete",
        statusMessage: "Select an explicit TSA CA trust-anchor file before RFC 3161 responses can be marked VERIFIED."
      };
    }
    return {
      status: "ready",
      statusMessage: `${timestampProviderLabel(settings.provider)} RFC 3161 service and its explicit TSA trust anchor are ready.`
    };
  }
  if (!settings.custom.providerName.trim() || !settings.custom.endpoint.trim()) {
    return {
      status: "not_configured",
      statusMessage: "Enter a provider name and TSA endpoint for Custom RFC 3161."
    };
  }
  if (settings.custom.authenticationMode === "basic" && !settings.custom.username.trim()) {
    return {
      status: "authentication_required",
      statusMessage: "Enter the account name and configure its secret separately."
    };
  }
  if (["basic", "bearer_token", "api_key"].includes(settings.custom.authenticationMode) && !timestampSecretConfigured) {
    return {
      status: "authentication_required",
      statusMessage: "Configure the provider token in secure local settings."
    };
  }
  if (settings.custom.authenticationMode === "client_certificate" && !settings.custom.clientCertificatePath.trim()) {
    return {
      status: "authentication_required",
      statusMessage: "Select a configured client certificate for this provider."
    };
  }
  if (!settings.custom.caCertificatePath.trim()) {
    return {
      status: "verification_configuration_incomplete",
      statusMessage: "Select an explicit TSA CA trust-anchor file before RFC 3161 responses can be marked VERIFIED."
    };
  }
  return {
    status: "ready",
    statusMessage: `${settings.custom.providerName.trim()} and its explicit trust anchor are ready.`
  };
}

export function normalizeTimestampSettings(
  next: TimestampSettings,
  timestampSecretConfigured = false
): TimestampSettings {
  const custom = { ...emptyTimestampSettings.custom, ...clone(next.custom ?? emptyTimestampSettings.custom) };
  const normalized: TimestampSettings = {
    ...emptyTimestampSettings,
    ...clone(next),
    custom
  };
  const status = configuredTimestampProviderStatus(normalized, timestampSecretConfigured);
  return { ...normalized, ...status };
}

function assertTimestampAttachable(track: TrackDetail): void {
  if (track.status === "FINALIZED" && track.certificate.valid && track.certificate.certificateId) return;
  throw new Error("Ein externer Zeitstempel kann erst nach der technischen Finalisierung angehängt werden.");
}

function currentTimestampRecord(track: TrackDetail): ExternalTimestampRecord | undefined {
  const recordId = track.externalTimestampSummary?.recordId;
  if (!recordId) return track.externalTimestamps.at(-1);
  return track.externalTimestamps.find((record) => record.id === recordId) ?? track.externalTimestamps.at(-1);
}

function timestampAttachmentTerminal(track: TrackDetail, settings: TimestampSettings): boolean {
  const summaryStatus = track.externalTimestampSummary?.status ?? "not_recorded";
  if (summaryStatus === "verified") return true;
  if (summaryStatus !== "attached") return false;
  const currentRecord = currentTimestampRecord(track);
  return !(timestampRecordUsesOpenTimestamps(currentRecord) && timestampProviderUsesRfc3161(settings.provider));
}

function timestampEvidenceFileName(provider: TimestampProviderKind, openTimestamps: boolean): string {
  if (openTimestamps) return "TIMESTAMP_EVIDENCE.ots";
  if (provider === "custom_rfc3161" || provider === "free_tsa") return "TIMESTAMP_RESPONSE.tsr";
  return "TIMESTAMP_RESPONSE.json";
}

function timestampVerificationMessage(openTimestamps: boolean): string {
  return openTimestamps
    ? "Detached proof is locally bound to the requested SHA-256; explicit OpenTimestamps verification or upgrade is pending."
    : "Structural and digest checks completed; provider signature and trust verification are not asserted.";
}

interface TimestampRecordInput {
  track: TrackDetail;
  settings: TimestampSettings;
  anchor: FinalizationAnchor;
  id: string;
  timestampedAt: string;
  provider: string;
  evidenceFileName: string;
  openTimestamps: boolean;
}

function timestampProviderMetadata(input: TimestampRecordInput): TimestampProviderMetadata {
  const { track, settings, timestampedAt, provider, evidenceFileName, openTimestamps } = input;
  return {
    adapter: openTimestamps ? "open_timestamps" : `demo-${settings.provider}`,
    protocol: openTimestamps
      ? "OpenTimestamps detached proof; Bitcoin anchoring pending verification/upgrade"
      : "RFC 3161",
    requestAlgorithm: "SHA-256",
    responseFormat: settings.provider === "open_timestamps" ? ".ots proof" : "RFC 3161 TimeStampResp",
    providerEndpointIdentifier: settings.provider === "custom_rfc3161" ? settings.custom.endpoint : provider,
    providerResponseFileName: evidenceFileName,
    providerResponseSha256: "f".repeat(64),
    referencedRevisionId: `demo-finalized-snapshot-${track.id}`,
    issuer: "",
    certificateSubject: "",
    certificateSerialNumber: "",
    policyOid: settings.provider === "custom_rfc3161" ? settings.custom.policyOid : "",
    responseStructureValid: openTimestamps ? null : true,
    providerDigestMatch: true,
    signatureVerified: null,
    trustChainVerified: null,
    verificationResult: "attached",
    verificationMessage: timestampVerificationMessage(openTimestamps),
    verificationTimestamp: timestampedAt
  };
}

function externalTimestampRecord(input: TimestampRecordInput): ExternalTimestampRecord {
  const { track, anchor, id, timestampedAt, provider, evidenceFileName, openTimestamps } = input;
  return {
    id,
    certificateId: track.certificate.certificateId!,
    provider,
    timestampType: "external_integrity_timestamp",
    timestampValue: openTimestamps ? "" : timestampedAt,
    referencedArtifact: "evidence_manifest",
    referencedArtifactPath: anchor.relativePath,
    referencedSha256: anchor.sha256,
    actualSha256: anchor.sha256,
    referencedHashMatch: true,
    externalReferenceId: `demo-${id.slice(0, 8)}`,
    providerVerificationUrl: openTimestamps ? "https://a.pool.opentimestamps.org/digest" : "",
    note: openTimestamps
      ? "OpenTimestamps detached proof archived; Bitcoin anchoring remains pending verification or upgrade."
      : "",
    evidenceFileName,
    evidenceSha256: "f".repeat(64),
    importedAt: timestampedAt,
    provenance: "Automatic provider response; structural and digest checks",
    providerMetadata: timestampProviderMetadata(input),
    recordRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD.json`,
    markdownRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.md`,
    pdfRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/EXTERNAL_TIMESTAMP_ADDENDUM.pdf`,
    hashListRelativePath: `06_CERTIFICATE/EXTERNAL_TIMESTAMPS/${id}/TIMESTAMP_RECORD_SHA256.txt`,
    integrityVerified: true,
    integrityIssues: []
  };
}

function timestampUnavailable(track: TrackDetail, settings: TimestampSettings): void {
  track.externalTimestampSummary = {
    status: settings.status === "authentication_required" ? "authentication_failed" : "provider_unavailable",
    message: settings.statusMessage,
    provider: timestampProviderLabel(settings.provider)
  };
}

function timestampAnchorMismatch(track: TrackDetail, settings: TimestampSettings): void {
  track.externalTimestampSummary = {
    status: "anchor_mismatch",
    message: "The finalized evidence-manifest anchor is not available.",
    provider: timestampProviderLabel(settings.provider)
  };
}

export function attachConfiguredTimestamp(track: TrackDetail, settings: TimestampSettings): TrackDetail {
  assertTimestampAttachable(track);
  if (timestampAttachmentTerminal(track, settings)) return track;
  if (settings.status !== "ready") {
    timestampUnavailable(track, settings);
    return track;
  }
  const anchor = track.finalizationAnchors.find((item) => item.artifact === "evidence_manifest");
  if (!anchor) {
    timestampAnchorMismatch(track, settings);
    return track;
  }
  const timestampedAt = now();
  const openTimestamps = settings.provider === "open_timestamps";
  track.externalTimestampSummary = {
    status: "requesting",
    message: "External timestamp request is being prepared.",
    provider: timestampProviderLabel(settings.provider)
  };
  const id = crypto.randomUUID();
  const provider = timestampProviderLabel(settings.provider);
  const evidenceFileName = timestampEvidenceFileName(settings.provider, openTimestamps);
  track.externalTimestamps.push(
    externalTimestampRecord({
      track,
      settings,
      anchor,
      id,
      timestampedAt,
      provider,
      evidenceFileName,
      openTimestamps
    })
  );
  track.externalTimestampSummary = {
    // A stored RFC-3161 response with a matching digest is ATTACHED until a
    // provider-specific signature/trust verification is actually available.
    status: "attached",
    message: openTimestamps
      ? "OpenTimestamps detached proof attached; later verification or upgrade is required."
      : "External timestamp response attached; structural and digest checks completed.",
    provider,
    recordId: id,
    updatedAt: timestampedAt
  };
  return track;
}
