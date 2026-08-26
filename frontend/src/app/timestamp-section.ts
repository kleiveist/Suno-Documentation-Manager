import type {
  ExternalTimestampRecord,
  TimestampProviderMetadata,
  TimestampQualificationRecord,
  TimestampQualificationStatus,
  TimestampSettings,
  TrackDetail
} from "../domain/types";
import { escapeHtml, formatDate } from "../ui/format";
import type { AppLanguage } from "../ui/i18n";
import { icon } from "../ui/icons";
import {
  externalTimestampAttachmentIsTerminal,
  externalTimestampIntegrityPresentation,
  externalTimestampMatchLabel,
  externalTimestampRecordPresentation,
  externalTimestampRecordUsesOpenTimestamps,
  externalTimestampStatusLabel,
  externalTimestampSummaryFor,
  timestampArtifactLabel,
  timestampProviderIsReady,
  timestampProviderStatusLabel,
  timestampProviderUsesRfc3161,
  timestampQualificationStatusLabel
} from "./timestamp-presentation";

interface TimestampSectionContext {
  settings: TimestampSettings;
  language: AppLanguage;
  translate: (value: string) => string;
  systemText: (value: string) => string;
}

type TimestampPresentation = ReturnType<typeof externalTimestampRecordPresentation>;
type ExternalTimestampSummary = ReturnType<typeof externalTimestampSummaryFor>;

const RETRYABLE_TIMESTAMP_STATUSES = new Set([
  "provider_unavailable",
  "authentication_failed",
  "verification_failed",
  "anchor_mismatch",
  "connection_failed",
  "unsupported_response",
  "verification_configuration_incomplete"
]);

function timestampRetryable(summary: ExternalTimestampSummary): boolean {
  return RETRYABLE_TIMESTAMP_STATUSES.has(summary.status);
}

function timestampRecordFor(
  track: TrackDetail,
  summary: ExternalTimestampSummary
): ExternalTimestampRecord | undefined {
  const records = track.externalTimestamps ?? [];
  if (!summary.recordId) return records.at(-1);
  return records.find((item) => item.id === summary.recordId) ?? records.at(-1);
}

function timestampSectionAction(
  track: TrackDetail,
  summary: ExternalTimestampSummary,
  record: ExternalTimestampRecord | undefined,
  context: TimestampSectionContext
): string {
  if (track.status !== "FINALIZED" || !track.certificate.valid) return "";
  if (externalTimestampAttachmentIsTerminal(summary.status, record, context.settings.provider)) {
    const label = summary.status === "verified" ? "Bereits verifiziert" : "Timestamp angehängt";
    return `<button class="button button--secondary" disabled>${icon("check")} ${label}</button>`;
  }
  if (!timestampProviderIsReady(context.settings)) {
    return `<button class="button button--secondary" data-action="open-timestamp-settings">${icon("settings")} Zu End-Einstellungen → 05 Externer Zeitstempel</button>`;
  }
  const addingRfc3161AfterOpenTimestamps =
    summary.status === "attached" &&
    Boolean(record && externalTimestampRecordUsesOpenTimestamps(record)) &&
    timestampProviderUsesRfc3161(context.settings.provider);
  const label = addingRfc3161AfterOpenTimestamps
    ? "RFC-3161-Zeitstempel zusätzlich anhängen"
    : timestampRetryable(summary)
      ? "Erneut versuchen"
      : "Externen Zeitstempel anhängen";
  return `<button class="button button--secondary" data-action="attach-external-timestamp">${icon("upload")} ${label}</button>`;
}

function timestampStatusClass(summary: ExternalTimestampSummary): string {
  if (summary.status === "verified") return "is-valid";
  return timestampRetryable(summary) ? "is-warning" : "";
}

function timestampStatusIcon(summary: ExternalTimestampSummary): string {
  if (summary.status === "verified") return "check";
  return summary.status === "not_recorded" || timestampRetryable(summary) ? "alert" : "info";
}

function rawProviderProof(metadata: TimestampProviderMetadata): string {
  if (!metadata.providerResponseFileName && !metadata.providerResponseSha256) return "";
  const digest = metadata.providerResponseSha256
    ? ` · <code>${escapeHtml(metadata.providerResponseSha256)}</code>`
    : "";
  return `<div><dt>Raw provider proof</dt><dd>${escapeHtml(metadata.providerResponseFileName || "Not documented")}${digest}</dd></div>`;
}

function providerRequestDetails(metadata: TimestampProviderMetadata): string {
  const nonce = metadata.requestNonce
    ? `<div><dt>Request/response nonce</dt><dd><code>${escapeHtml(metadata.requestNonce)}</code> · ${externalTimestampMatchLabel(metadata.nonceMatch ?? null)}</dd></div>`
    : "";
  const policy =
    metadata.requestedPolicyOid || metadata.policyOid
      ? `<div><dt>Requested/returned policy</dt><dd>${escapeHtml(metadata.requestedPolicyOid || "none")} / ${escapeHtml(metadata.policyOid || "Not documented")} · ${metadata.requestedPolicyOid ? externalTimestampMatchLabel(metadata.policyMatch ?? null) : "N/A"}</dd></div>`
      : "";
  return `${nonce}
                  ${policy}`;
}

function providerVerificationDetails(metadata: TimestampProviderMetadata, presentation: TimestampPresentation): string {
  const cmsTrust = presentation.openTimestamps
    ? presentation.cmsTrustValue
    : `${externalTimestampMatchLabel(metadata.signatureVerified)} / ${externalTimestampMatchLabel(metadata.trustChainVerified)}`;
  const verifier = metadata.cryptographicVerifier
    ? `<div><dt>Cryptographic verifier</dt><dd>${escapeHtml(metadata.cryptographicVerifier)}</dd></div>`
    : "";
  const anchors = metadata.trustAnchorSha256?.length
    ? `<div><dt>Trust-anchor SHA-256</dt><dd><code>${escapeHtml(metadata.trustAnchorSha256.join(", "))}</code></dd></div>`
    : "";
  return `<div><dt>CMS signature / trust chain</dt><dd>${cmsTrust}</dd></div>
                  ${verifier}
                  ${anchors}
                  <div><dt>Verification result</dt><dd>${escapeHtml(externalTimestampStatusLabel(metadata.verificationResult))}</dd></div>`;
}

function providerIdentityDetails(metadata: TimestampProviderMetadata): string {
  const issuer = metadata.issuer ? `<div><dt>Timestamp issuer</dt><dd>${escapeHtml(metadata.issuer)}</dd></div>` : "";
  const subject = metadata.certificateSubject
    ? `<div><dt>Signer certificate subject</dt><dd>${escapeHtml(metadata.certificateSubject)}</dd></div>`
    : "";
  const serial = metadata.certificateSerialNumber
    ? `<div><dt>Signer certificate serial</dt><dd><code>${escapeHtml(metadata.certificateSerialNumber)}</code></dd></div>`
    : "";
  const certificateDigest = metadata.certificateSha256
    ? `<div><dt>Signer certificate SHA-256</dt><dd><code>${escapeHtml(metadata.certificateSha256)}</code></dd></div>`
    : "";
  const policy = metadata.policyOid ? `<div><dt>Policy OID</dt><dd>${escapeHtml(metadata.policyOid)}</dd></div>` : "";
  return `${issuer}${subject}${serial}${certificateDigest}<div><dt>Provider identity technically recognized</dt><dd>${externalTimestampMatchLabel(metadata.providerIdentityVerified ?? null)}</dd></div><div><dt>Signature applicable / valid</dt><dd>${externalTimestampMatchLabel(metadata.signatureVerificationApplicable ?? null)} / ${externalTimestampMatchLabel(metadata.signatureVerified)}</dd></div><div><dt>Certificate chain applicable / verified</dt><dd>${externalTimestampMatchLabel(metadata.trustChainVerificationApplicable ?? null)} / ${externalTimestampMatchLabel(metadata.trustChainVerified)}</dd></div>
                  ${policy}`;
}

function providerMetadataMarkup(record: ExternalTimestampRecord, presentation: TimestampPresentation): string {
  const metadata = record.providerMetadata;
  if (!metadata) return "";
  return `<dl>
                  <div><dt>Protocol</dt><dd>${escapeHtml(metadata.protocol || "Not documented")}</dd></div>
                  <div><dt>Adapter</dt><dd>${escapeHtml(metadata.adapter || "Not documented")}</dd></div>
                  ${rawProviderProof(metadata)}
                  <div><dt>${presentation.bindingLabel}</dt><dd>${externalTimestampMatchLabel(metadata.providerDigestMatch)}</dd></div>
                  ${providerRequestDetails(metadata)}
                  ${providerVerificationDetails(metadata, presentation)}
                  ${providerIdentityDetails(metadata)}
                </dl>`;
}

function qualificationFallback(value: TimestampQualificationStatus | undefined, legacy: boolean): string {
  return timestampQualificationStatusLabel(value ?? (legacy ? "not_documented" : "not_checked"));
}

function trustedListDetails(qualification: TimestampQualificationRecord | undefined): string {
  const trustedList = qualification?.trustedList;
  if (!trustedList) return "";
  return `<div><dt>Trusted List source / validation</dt><dd>${escapeHtml(trustedList.source)} · ${escapeHtml(trustedList.validationStatus.toUpperCase())}</dd></div><div><dt>Trusted List identity</dt><dd>${escapeHtml(trustedList.territory || "Not documented")} · Version ${escapeHtml(trustedList.version || "Not documented")} · Sequence ${escapeHtml(trustedList.sequenceNumber || "Not documented")} · <code>${escapeHtml(trustedList.sha256)}</code></dd></div><div><dt>Trusted List period / validation time</dt><dd>${escapeHtml(trustedList.issuedAt || "Not documented")} – ${escapeHtml(trustedList.nextUpdate || "Not documented")} · ${escapeHtml(trustedList.validatedAt || "Not documented")}</dd></div>`;
}

function qualificationServiceDetails(qualification: TimestampQualificationRecord | undefined): string {
  if (!qualification) return "";
  const matchedService = qualification.trustServiceName
    ? `<div><dt>Matched service</dt><dd>${escapeHtml(qualification.trustServiceProvider)} · ${escapeHtml(qualification.trustServiceName)}</dd></div>`
    : "";
  const serviceType = qualification.serviceType
    ? `<div><dt>Service type</dt><dd>${escapeHtml(qualification.serviceType)}</dd></div>`
    : "";
  const serviceStatus = qualification.serviceStatus
    ? `<div><dt>Service status at timestamp</dt><dd>${escapeHtml(qualification.serviceStatus)}</dd></div>`
    : "";
  const currentServiceStatus = qualification.currentServiceStatus
    ? `<div><dt>Current service status</dt><dd>${escapeHtml(qualification.currentServiceStatus)}</dd></div>`
    : "";
  const identifier = qualification.serviceIdentifier
    ? `<div><dt>Service identifier</dt><dd><code>${escapeHtml(qualification.serviceIdentifier)}</code></dd></div>`
    : "";
  const type = qualification.qualificationType
    ? `<div><dt>Qualification type</dt><dd>${escapeHtml(qualification.qualificationType)}</dd></div>`
    : "";
  return `${matchedService}${serviceType}${serviceStatus}${currentServiceStatus}${identifier}${type}`;
}

function qualificationPeriodDetails(qualification: TimestampQualificationRecord | undefined): string {
  if (!qualification) return "";
  const timestampPeriod =
    qualification.statusValidFrom || qualification.statusValidUntil
      ? `<div><dt>Timestamp-time service-status period</dt><dd>${escapeHtml(qualification.statusValidFrom || "Not documented")} – ${escapeHtml(qualification.statusValidUntil || "open")}</dd></div>`
      : "";
  const currentPeriod =
    qualification.currentStatusValidFrom || qualification.currentStatusValidUntil
      ? `<div><dt>Current service-status period</dt><dd>${escapeHtml(qualification.currentStatusValidFrom || "Not documented")} – ${escapeHtml(qualification.currentStatusValidUntil || "open")}</dd></div>`
      : "";
  const identity = qualification.identity?.certificateSha256
    ? `<div><dt>Matched identity certificate</dt><dd><code>${escapeHtml(qualification.identity.certificateSha256)}</code> · ${escapeHtml(qualification.identity.certificateSubject || "Not documented")}</dd></div>`
    : "";
  return `${timestampPeriod}${currentPeriod}${identity}`;
}

function qualificationAuditDetails(
  qualification: TimestampQualificationRecord | undefined,
  context: TimestampSectionContext
): string {
  if (!qualification) return "";
  const checkedAt = qualification.checkedAt
    ? `<div><dt>Checked at</dt><dd>${formatDate(qualification.checkedAt, true, context.language)}</dd></div>`
    : "";
  const message = qualification.message
    ? `<div><dt>Qualification detail</dt><dd>${escapeHtml(qualification.message)}</dd></div>`
    : "";
  return `${checkedAt}${message}`;
}

function qualificationMarkup(
  record: ExternalTimestampRecord,
  legacy: boolean,
  context: TimestampSectionContext
): string {
  const qualification = record.providerMetadata?.qualification;
  const verified =
    qualification?.eidasQualificationStatus === "qualified_service_verified"
      ? `<p><strong>${escapeHtml(context.translate("eIDAS QUALIFIED TRUST SERVICE – VERIFIED"))}</strong></p>`
      : "";
  const status = (value: TimestampQualificationStatus | undefined): string =>
    escapeHtml(context.translate(qualificationFallback(value, legacy)));
  return `<h4>Provider-Vertrauen und Qualifizierung</h4>
                ${verified}
                <dl>
                  <div><dt>Provider identity</dt><dd>${status(qualification?.providerIdentityStatus)}</dd></div>
                  <div><dt>Trust Service</dt><dd>${status(qualification?.trustServiceStatus)}</dd></div>
                  <div><dt>eIDAS qualification</dt><dd>${status(qualification?.eidasQualificationStatus)}</dd></div>
                  <div><dt>Qualification at timestamp</dt><dd>${status(qualification?.qualificationAtTimestamp)}</dd></div>
                  <div><dt>Current qualification</dt><dd>${status(qualification?.currentQualificationStatus)}</dd></div>
                  ${trustedListDetails(qualification)}
                  ${qualificationServiceDetails(qualification)}${qualificationPeriodDetails(qualification)}
                  ${qualificationAuditDetails(qualification, context)}
                </dl>`;
}

function timestampRecordFooter(
  record: ExternalTimestampRecord,
  presentation: TimestampPresentation,
  issues: string[],
  context: TimestampSectionContext
): string {
  const reference = record.externalReferenceId
    ? `<p>External reference ID: ${escapeHtml(record.externalReferenceId)}</p>`
    : "";
  const url = record.providerVerificationUrl
    ? `<p>${escapeHtml(context.translate(presentation.endpointLabel))}: ${escapeHtml(record.providerVerificationUrl)}</p>`
    : "";
  const integrityIssues = issues.length
    ? `<p>${escapeHtml(issues.map((issue) => context.systemText(issue)).join(" · "))}</p>`
    : "";
  return `${reference}
                ${url}
                ${integrityIssues}`;
}

function timestampRecordMarkup(
  record: ExternalTimestampRecord,
  summary: ExternalTimestampSummary,
  context: TimestampSectionContext
): string {
  const integrity = externalTimestampIntegrityPresentation(record);
  const presentation = externalTimestampRecordPresentation(record);
  const legacy = /user-confirmed|manually recorded/i.test(record.provenance);
  const qualification = record.providerMetadata?.qualification;
  const qualificationStatus = qualification?.status ?? (legacy ? "not_documented" : "not_checked");
  return `<article class="timestamp-record ${record.integrityVerified ? "" : "is-integrity-invalid"}">
              <header><div><strong>${escapeHtml(summary.provider || record.provider)}</strong><small>${legacy ? "Legacy manually recorded timestamp evidence" : "Provider-derived metadata"}</small></div><span class="verification ${integrity.label === "VERIFIED" ? "is-valid" : ""}">Addendum files and anchor integrity: ${integrity.label}</span></header>
              <dl>
                <div><dt>${presentation.timestampLabel}</dt><dd>${escapeHtml(presentation.timestampValue)}</dd></div>
                <div><dt>Referenced artifact</dt><dd>${escapeHtml(timestampArtifactLabel(record.referencedArtifact))}</dd></div>
                <div><dt>Referenced SHA-256</dt><dd><code>${escapeHtml(record.referencedSha256)}</code></dd></div>
                <div><dt>Current provider configuration</dt><dd>${escapeHtml(context.translate(timestampProviderStatusLabel(context.settings.status)))}</dd></div>
                <div><dt>Technical timestamp status</dt><dd>${escapeHtml(context.translate(externalTimestampStatusLabel(summary.status)))}</dd></div>
                <div><dt>Protocol</dt><dd>${escapeHtml(record.providerMetadata?.protocol || "Not documented")}</dd></div>
                <div><dt>Provider qualification</dt><dd>${escapeHtml(context.translate(timestampQualificationStatusLabel(qualificationStatus)))}</dd></div>
              </dl>
              <details><summary>Details anzeigen</summary>
                <h4>Technischer Zeitstempel</h4>
                <dl>
                  <div><dt>Anchor path</dt><dd>${escapeHtml(record.referencedArtifactPath)}</dd></div>
                  <div><dt>Timestamp evidence</dt><dd>${escapeHtml(record.evidenceFileName)} · <code>${escapeHtml(record.evidenceSha256)}</code></dd></div>
                  <div><dt>Imported at</dt><dd>${formatDate(record.importedAt, true, context.language)}</dd></div>
                  <div><dt>Provenance</dt><dd>${escapeHtml(record.provenance)}</dd></div>
                </dl>
                ${providerMetadataMarkup(record, presentation)}
                ${qualificationMarkup(record, legacy, context)}
                ${timestampRecordFooter(record, presentation, integrity.issues, context)}
              </details>
            </article>`;
}

function unrecordedTimestampNotice(summary: ExternalTimestampSummary, providerReady: boolean): string {
  if (summary.status !== "not_recorded") return "";
  return `<p class="timestamp-summary-copy">Der Track ist technisch vollständig finalisiert. Ein externer Zeitstempel wurde für diesen Zertifikatssnapshot noch nicht hinterlegt.</p>${providerReady ? "" : `<p class="timestamp-summary-copy">Kein externer Timestamp-Dienst eingerichtet.</p>`}`;
}

export function renderExternalTimestampSection(track: TrackDetail, context: TimestampSectionContext): string {
  const summary = externalTimestampSummaryFor(track);
  const record = timestampRecordFor(track, summary);
  const providerReady = timestampProviderIsReady(context.settings);
  const action = timestampSectionAction(track, summary, record, context);
  const recordMarkup = record ? timestampRecordMarkup(record, summary, context) : "";
  return `<section class="panel external-timestamp-section">
        <div class="panel-heading"><div><p class="overline">Finalization snapshot and later addenda</p><h3>External Timestamp Evidence</h3><p>Das finale Zertifikat hält den tatsächlichen Timestamp- und Qualification-Zustand seiner Erzeugung fest. Spätere Wiederholungen ergänzen unveränderliche Addenda, ohne das PDF umzuschreiben.</p></div>${action}</div>
        <div class="timestamp-summary">${icon("lock")}<div><strong>Finales Zertifikat: Timestamp-Zustand bei einmaliger Erzeugung erfasst</strong><span>Der Manifest-Anchor wird zuerst festgelegt; anschließend wird der automatische Provider- und Qualification-Versuch ausgewertet und erst danach das finale PDF genau einmal gerendert. Spätere Wiederholungen bleiben separate Addenda.</span></div></div>
        <div class="timestamp-summary ${timestampStatusClass(summary)}">${icon(timestampStatusIcon(summary))}<div><strong>${escapeHtml(context.translate(`Aktueller Timestamp-Nachweis: ${context.translate(externalTimestampStatusLabel(summary.status))}`))}</strong><span>${escapeHtml(context.systemText(summary.message))}</span></div></div>
        ${unrecordedTimestampNotice(summary, providerReady)}
        ${recordMarkup}
        <p class="certificate-disclaimer">Technical timestamp verification and provider qualification are independent. A positive eIDAS statement is shown only when backed by a validated Trusted List record for the relevant timestamp time.</p>
      </section>`;
}
