import type {
  EvidenceMetadata,
  ExternalTimestampRecord,
  ExternalTimestampStatus,
  ExternalTimestampType,
  FactOrigin,
  TimestampProviderCapabilities,
  TimestampProviderKind,
  TimestampProviderStatus,
  TimestampQualificationStatus,
  TimestampReferencedArtifact,
  TimestampSettings,
  TrackDetail
} from "../domain/types";
import { escapeHtml } from "../ui/format";
import { icon } from "../ui/icons";

export function externalTimestampMatchLabel(value: boolean | null): string {
  return value === true ? "YES" : value === false ? "NO" : "NOT VERIFIED";
}

export function externalTimestampIntegrityPresentation(
  record: Pick<ExternalTimestampRecord, "integrityVerified" | "integrityIssues">
): { label: "VERIFIED" | "FAILED"; issues: string[] } {
  return {
    label: record.integrityVerified ? "VERIFIED" : "FAILED",
    issues: record.integrityIssues.map((issue) => issue.trim()).filter(Boolean)
  };
}

export function externalTimestampRecordUsesOpenTimestamps(
  record: Pick<ExternalTimestampRecord, "provider" | "providerMetadata">
): boolean {
  const adapter = record.providerMetadata?.adapter.toLowerCase() ?? "";
  const protocol = record.providerMetadata?.protocol.toLowerCase() ?? "";
  return (
    adapter.includes("open_timestamps") ||
    protocol.includes("opentimestamps") ||
    record.provider.toLowerCase() === "opentimestamps"
  );
}

export function externalTimestampRecordPresentation(
  record: Pick<ExternalTimestampRecord, "provider" | "providerMetadata" | "timestampValue">
): {
  openTimestamps: boolean;
  timestampLabel: string;
  timestampValue: string;
  bindingLabel: string;
  cmsTrustValue: string;
  endpointLabel: string;
} {
  const openTimestamps = externalTimestampRecordUsesOpenTimestamps(record);
  return {
    openTimestamps,
    timestampLabel: openTimestamps ? "Confirmed timestamp" : "Timestamp",
    timestampValue:
      record.timestampValue ||
      (openTimestamps ? "PENDING — OpenTimestamps verification / upgrade required" : "Not documented"),
    bindingLabel: openTimestamps ? "Local manifest / proof binding" : "Provider digest match",
    cmsTrustValue: openTimestamps ? "N/A — not RFC 3161" : "",
    endpointLabel: openTimestamps ? "Calendar endpoint" : "Provider verification URL"
  };
}

export function timestampProviderUsesRfc3161(provider: TimestampProviderKind): boolean {
  return provider === "free_tsa" || provider === "sigstore_public_tsa" || provider === "custom_rfc3161";
}

export function externalTimestampAttachmentIsTerminal(
  status: ExternalTimestampStatus,
  record: Pick<ExternalTimestampRecord, "provider" | "providerMetadata" | "provenance"> | undefined,
  configuredProvider: TimestampProviderKind
): boolean {
  if (status === "verified") return true;
  if (status !== "attached") return false;
  if (!record || /user-confirmed|manually recorded/i.test(record.provenance)) return false;
  // An initial OpenTimestamps proof is not RFC 3161. After the user explicitly
  // switches to a ready RFC 3161 provider, a second certificate-bound sidecar
  // may therefore be added without changing either existing attachment.
  return !(externalTimestampRecordUsesOpenTimestamps(record) && timestampProviderUsesRfc3161(configuredProvider));
}

export function timestampProviderProtocolPresentation(
  provider: TimestampProviderKind,
  capabilities?: TimestampProviderCapabilities
): { label: string; detail: string } {
  const openTimestamps = capabilities?.openTimestamps ?? provider === "open_timestamps";
  const rfc3161 = capabilities?.rfc3161 ?? timestampProviderUsesRfc3161(provider);
  if (openTimestamps) {
    return {
      label: "OpenTimestamps — nicht RFC 3161",
      detail:
        "SHA-256-Detached-Proof; ein erster Nachweis bleibt ATTACHED, bis OpenTimestamps-Upgrade und -Verifikation die Bitcoin-Verankerung bestätigen. CMS- und TSA-Trust-Chain-Prüfungen sind nicht anwendbar."
    };
  }
  if (rfc3161) {
    return {
      label: "RFC 3161",
      detail:
        "Signierter TimeStampResp; VERIFIED erfordert CMS-, Nonce-, Policy-, EKU- und Vertrauensketteprüfung gegen den ausdrücklich gewählten TSA Trust Anchor."
    };
  }
  return {
    label: "Kein Timestamp-Protokoll aktiv",
    detail: "Es wird kein externer Zeitstempelnachweis angefordert."
  };
}

export function timestampProviderLabel(value: TimestampProviderKind): string {
  return (
    {
      disabled: "Disabled",
      free_tsa: "FreeTSA",
      open_timestamps: "OpenTimestamps",
      sigstore_public_tsa: "Sigstore Public TSA",
      custom_rfc3161: "Custom RFC 3161"
    } as const
  )[value];
}

export function timestampProviderStatusLabel(value: TimestampProviderStatus): string {
  return (
    {
      disabled: "Disabled",
      not_configured: "Not configured",
      ready: "Ready",
      authentication_required: "Authentication required",
      authentication_failed: "Authentication failed",
      connection_failed: "Connection failed",
      verification_configuration_incomplete: "Verification configuration incomplete",
      provider_error: "Provider error"
    } as const
  )[value];
}

export function timestampQualificationStatusLabel(value: TimestampQualificationStatus): string {
  return (
    {
      not_checked: "NOT CHECKED",
      not_documented: "NOT DOCUMENTED",
      not_verified: "NOT VERIFIED",
      provider_identity_verified: "PROVIDER IDENTITY VERIFIED",
      trust_service_verified: "TRUST SERVICE VERIFIED",
      qualified_service_verified: "QUALIFIED SERVICE VERIFIED",
      check_failed: "CHECK FAILED"
    } as const
  )[value];
}

export function externalTimestampStatusLabel(value: ExternalTimestampStatus): string {
  return (
    {
      not_recorded: "NOT RECORDED",
      requesting: "REQUESTING",
      attached: "ATTACHED",
      verified: "VERIFIED",
      verification_failed: "VERIFICATION FAILED",
      provider_unavailable: "PROVIDER UNAVAILABLE",
      authentication_failed: "AUTHENTICATION FAILED",
      anchor_mismatch: "ANCHOR MISMATCH",
      disabled: "DISABLED",
      ready: "READY",
      configuration_incomplete: "CONFIGURATION INCOMPLETE",
      authentication_required: "AUTHENTICATION REQUIRED",
      connection_failed: "CONNECTION FAILED",
      unsupported_response: "UNSUPPORTED RESPONSE",
      verification_configuration_incomplete: "VERIFICATION CONFIGURATION INCOMPLETE"
    } as const
  )[value];
}

export function timestampProviderIsReady(settings: TimestampSettings): boolean {
  return settings.enabled && settings.provider !== "disabled" && settings.status === "ready";
}

export function externalTimestampSummaryFor(
  track: Pick<TrackDetail, "externalTimestampSummary" | "externalTimestamps">
): { status: ExternalTimestampStatus; message: string; provider?: string; recordId?: string; updatedAt?: string } {
  if (track.externalTimestampSummary) return track.externalTimestampSummary;
  const record = track.externalTimestamps.at(-1);
  if (!record) {
    return {
      status: "not_recorded",
      message:
        "Der Track ist technisch vollständig finalisiert. Ein externer Zeitstempel wurde für diesen Zertifikatssnapshot noch nicht hinterlegt.",
      provider: ""
    };
  }
  const legacy = /user-confirmed|manually recorded/i.test(record.provenance);
  return {
    // Sidecar/hash integrity is not a provider-response verification. Legacy
    // manual records must never be promoted to VERIFIED by UI inference.
    status: "attached",
    message: legacy
      ? "Legacy manually recorded timestamp evidence is attached and has not been automatically promoted to verified."
      : "A timestamp response is attached; review its technical verification details.",
    provider: record.provider,
    recordId: record.id,
    updatedAt: record.importedAt
  };
}

export function legacySunoPlanNoticeMarkup(value: string): string {
  if (!value.trim()) return "";
  return `<div class="legacy-data-notice">${icon("info")}<div><strong>Historischer Tarif bei Erstellung – keine Aussage zum Tarif bei der finalen Generation</strong><p>${escapeHtml(value)}</p><small>Dieser unverändert erhaltene Altwert füllt „Suno-Tarif bei der finalen Generation“ nicht aus und erfüllt die aktuelle Workflow-Anforderung nicht.</small></div></div>`;
}

export function externalTimestampTypeLabel(value: ExternalTimestampType): string {
  return (
    {
      qualified_electronic_timestamp_user_declared: "Qualified electronic timestamp – user declared",
      electronic_timestamp: "Electronic timestamp",
      external_integrity_timestamp: "External integrity timestamp",
      other: "Other",
      not_documented: "Not documented"
    } as const
  )[value];
}

export function timestampArtifactLabel(value: TimestampReferencedArtifact): string {
  return (
    {
      evidence_manifest: "EVIDENCE_MANIFEST.json",
      sha256sums: "SHA256SUMS.txt",
      documentation_certificate_markdown: "DOCUMENTATION_CERTIFICATE.md",
      certificate_pdf: "Certificate PDF (English)",
      final_evidence_package: "Final Evidence Package",
      other: "Other"
    } as const
  )[value];
}

export function termsMetadataComplete(metadata: Partial<EvidenceMetadata> | undefined): boolean {
  return Boolean(metadata?.documentTitle?.trim() && metadata.provider?.trim() && metadata.retrievalDate?.trim());
}

export function isAutomaticDateReadonly(origin: FactOrigin): boolean {
  return origin === "evidence_derived_metadata";
}
