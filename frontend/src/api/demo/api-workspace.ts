import type {
  AudioScreeningProviderTestResult,
  AudioScreeningSecretInput,
  AudioScreeningSettings,
  EvidenceMetadata,
  EvidenceRole,
  GlobalProfile,
  GlobalEvidenceItem,
  ScanResult,
  SubscriptionBillingCycle,
  TimestampSettings,
  WorkflowDefinitionDto
} from "../../domain/types";
import { WORKFLOW_ID, WORKFLOW_VERSION } from "../../domain/workflow";
import { subscriptionCoverageEnd } from "../../domain/subscription";
import { emptyEvidenceMetadata } from "../../domain/types";
import { clone, evidence, now, wait } from "./helpers";
import { makeTrack, refresh, sameTrackDocumentationProfile } from "./fixtures";
import {
  configuredTimestampProviderStatus,
  normalizeTimestampSettings,
  timestampProviderCapabilities
} from "./timestamp";
import { configuredAudioScreeningStatus, normalizeAudioScreeningSettings } from "./audio-screening";
import { DemoApiBase } from "./api-base";

export abstract class DemoWorkspaceApi extends DemoApiBase {
  async getWorkflow(): Promise<WorkflowDefinitionDto> {
    return {
      schemaVersion: 1,
      id: WORKFLOW_ID,
      version: WORKFLOW_VERSION,
      name: "Suno Track Documentation",
      steps: [
        { id: "track", number: "01", label: "Track", description: "Titel und Produktionszeitraum", required: true },
        {
          id: "source",
          number: "02",
          label: "Quelle",
          description: "Audioquellen und Rechtezuordnung",
          required: true
        },
        {
          id: "suno",
          number: "03",
          label: "Suno",
          description: "Projekt, Modell und Erstellungstarif",
          required: true
        },
        {
          id: "human_work",
          number: "04",
          label: "Menschliche Arbeit",
          description: "Lyrics und bestätigte Bearbeitungen",
          required: true
        },
        {
          id: "artwork",
          number: "05",
          label: "Artwork",
          description: "Entstehung und Content-Check",
          required: true
        },
        {
          id: "ai_transparency",
          number: "06",
          label: "KI-Transparenz",
          description: "Projektinterne Disclosure-Policy",
          required: true
        },
        {
          id: "release",
          number: "07",
          label: "Release",
          description: "Letzte Bearbeitung und Release-Dateien",
          required: true
        },
        {
          id: "evidence_licenses",
          number: "08",
          label: "Evidence & Lizenzen",
          description: "Nachweise vollständig zuordnen",
          required: true
        },
        {
          id: "integrity",
          number: "09",
          label: "Integrität",
          description: "Dokumente, SHA-256 und Verifikation",
          required: true
        },
        {
          id: "finalize",
          number: "10",
          label: "Finalisieren",
          description: "Gate prüfen und Zertifikat erzeugen",
          required: true
        }
      ]
    };
  }

  async openWorkspace() {
    await wait();
    this.state.workspace = {
      id: "demo-workspace",
      name: "Music Projects",
      path: "Beispiel/Workspace",
      trackCount: 2,
      lastScannedAt: now()
    };
    if (this.state.tracks.size === 0) {
      const demoAlbum = { section: "album", albumTitle: "Event Horizon" } as const;
      this.rememberAlbum(demoAlbum);
      this.state.tracks.set("gravity", makeTrack("gravity", "Gravity", this.state.profile, true, demoAlbum));
      this.state.tracks.set("cosmic-pulse", makeTrack("cosmic-pulse", "Cosmic Pulse", this.state.profile));
    }
    return clone(this.state.workspace);
  }

  async createWorkspace() {
    return this.openWorkspace();
  }

  async restoreWorkspace(_path: string) {
    void _path;
    const opened = await this.openWorkspace();
    if (!opened) throw new Error("Demo-Workspace konnte nicht geöffnet werden.");
    return opened;
  }

  async scanWorkspace(): Promise<ScanResult> {
    await wait();
    if (!this.state.workspace) throw new Error("Öffne zuerst einen Workspace.");
    this.state.workspace.lastScannedAt = now();
    return {
      discovered: this.state.tracks.size,
      indexed: this.state.tracks.size,
      unchanged: this.state.tracks.size,
      warnings: []
    };
  }

  async getProfile() {
    await wait();
    return clone(this.state.profile);
  }

  async updateProfile(next: GlobalProfile) {
    await wait();
    this.state.profile = clone(next);
    for (const track of this.state.tracks.values()) {
      if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
      if (sameTrackDocumentationProfile(track.profileSnapshot, this.state.profile)) continue;
      track.profileSnapshot = clone(this.state.profile);
      track.documents.current = false;
      refresh(track);
    }
    return clone(this.state.profile);
  }

  async getTimestampSettings() {
    await wait();
    return clone(this.state.timestampSettings);
  }

  async updateTimestampSettings(next: TimestampSettings) {
    await wait();
    this.state.timestampSettings = normalizeTimestampSettings(next, this.state.timestampSecretConfigured);
    return clone(this.state.timestampSettings);
  }

  async updateTimestampSecret(secret: string | null) {
    await wait();
    this.state.timestampSecretConfigured = Boolean(secret?.trim());
    const status = configuredTimestampProviderStatus(
      this.state.timestampSettings,
      this.state.timestampSecretConfigured
    );
    this.state.timestampSettings = { ...this.state.timestampSettings, ...status };
  }

  async testTimestampProvider() {
    await wait();
    const status = configuredTimestampProviderStatus(
      this.state.timestampSettings,
      this.state.timestampSecretConfigured
    );
    const testedAt = now();
    this.state.timestampSettings = {
      ...this.state.timestampSettings,
      ...status,
      lastTestedAt: testedAt
    };
    return {
      provider: this.state.timestampSettings.provider,
      status: this.state.timestampSettings.status,
      message:
        this.state.timestampSettings.status === "ready"
          ? this.state.timestampSettings.provider === "open_timestamps"
            ? "OpenTimestamps calendar reachable. This is not RFC 3161; proof verification or upgrade remains pending."
            : "Provider reachable. RFC 3161 timestamp service ready."
          : this.state.timestampSettings.statusMessage,
      testedAt,
      capabilities: timestampProviderCapabilities(this.state.timestampSettings.provider)
    };
  }

  async getAudioScreeningSettings() {
    await wait();
    return clone(this.state.audioScreeningSettings);
  }

  async updateAudioScreeningSettings(next: AudioScreeningSettings) {
    await wait();
    const credentialsConfigured = this.state.audioAccessKeyConfigured && this.state.audioAccessSecretConfigured;
    this.state.audioScreeningSettings = normalizeAudioScreeningSettings(next, credentialsConfigured);
    return clone(this.state.audioScreeningSettings);
  }

  async updateAudioScreeningSecret(input: AudioScreeningSecretInput) {
    await wait();
    if (typeof input.accessKey === "string" && input.accessKey.trim()) this.state.audioAccessKeyConfigured = true;
    if (typeof input.accessSecret === "string" && input.accessSecret.trim())
      this.state.audioAccessSecretConfigured = true;
    const credentialsConfigured = this.state.audioAccessKeyConfigured && this.state.audioAccessSecretConfigured;
    this.state.audioScreeningSettings = {
      ...this.state.audioScreeningSettings,
      credentialsConfigured,
      ...configuredAudioScreeningStatus(this.state.audioScreeningSettings, credentialsConfigured)
    };
  }

  async testAudioScreeningProvider(): Promise<AudioScreeningProviderTestResult> {
    await wait();
    const credentialsConfigured = this.state.audioAccessKeyConfigured && this.state.audioAccessSecretConfigured;
    const configured = configuredAudioScreeningStatus(this.state.audioScreeningSettings, credentialsConfigured);
    const testedAt = now();
    const unavailable = configured.status === "ready";
    const status = unavailable ? "provider_unavailable" : configured.status;
    const message = unavailable
      ? "Browser demo: no ACRCloud connection is made. Test the configured provider in the desktop app."
      : configured.statusMessage;
    this.state.audioScreeningSettings = {
      ...this.state.audioScreeningSettings,
      credentialsConfigured,
      status,
      statusMessage: message,
      lastTestedAt: testedAt
    };
    return { status, message, testedAt };
  }

  async listGlobalEvidence() {
    await wait();
    return clone(this.state.globalEvidence);
  }

  async importGlobalEvidence(role: EvidenceRole, coverageStart: string, billingCycle: SubscriptionBillingCycle) {
    await wait();
    const coverageEnd =
      coverageStart && billingCycle ? (subscriptionCoverageEnd(coverageStart, billingCycle) ?? undefined) : undefined;
    const item = {
      ...evidence(role, `subscription_${new Date().toISOString().slice(0, 7)}.pdf`),
      coverageStart,
      coverageEnd
    };
    this.state.globalEvidence.push(item);
    return clone(item);
  }

  async importGlobalTermsEvidence(metadata: Partial<EvidenceMetadata>) {
    await wait();
    const normalized: EvidenceMetadata = {
      ...emptyEvidenceMetadata(),
      ...clone(metadata),
      originalFileName: "suno_terms.pdf"
    };
    if (!normalized.documentTitle.trim() || !normalized.provider.trim() || !normalized.retrievalDate.trim()) {
      throw new Error("Dokumenttitel, Anbieter und Abrufdatum sind für den Terms-Nachweis erforderlich.");
    }
    const item: GlobalEvidenceItem = {
      ...evidence("suno_terms_rights", "suno_terms.pdf"),
      relativePath: ".suno-doc/global-evidence/suno_terms.pdf",
      metadata: normalized
    };
    this.state.globalEvidence.push(item);
    for (const track of this.state.tracks.values()) {
      if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
      this.attachGlobalToTrack(track, item);
    }
    return clone(item);
  }

  async updateGlobalTermsEvidenceMetadata(evidenceId: string, metadata: Partial<EvidenceMetadata>) {
    await wait();
    const item = this.state.globalEvidence.find(
      (entry) => entry.id === evidenceId && entry.role === "suno_terms_rights"
    );
    if (!item) throw new Error("Der globale Terms-Nachweis wurde nicht gefunden.");
    const normalized: EvidenceMetadata = {
      ...emptyEvidenceMetadata(),
      ...item.metadata,
      ...clone(metadata),
      originalFileName: item.metadata?.originalFileName || item.fileName
    };
    if (!normalized.documentTitle.trim() || !normalized.provider.trim() || !normalized.retrievalDate.trim()) {
      throw new Error("Dokumenttitel, Anbieter und Abrufdatum sind für den Terms-Nachweis erforderlich.");
    }
    item.metadata = normalized;
    for (const track of this.state.tracks.values()) {
      if (track.status === "FINALIZED" || track.status === "SUPERSEDED") continue;
      let changed = false;
      for (const copy of track.evidence.filter(
        (entry) => entry.sourceGlobalEvidenceId === evidenceId && entry.provenance === "global_copy"
      )) {
        copy.metadata = clone(normalized);
        changed = true;
      }
      if (changed) {
        track.documents.current = false;
        refresh(track);
      }
    }
    return clone(item);
  }

  async removeGlobalEvidence(evidenceId: string) {
    await wait();
    this.state.globalEvidence = this.state.globalEvidence.filter((item) => item.id !== evidenceId);
  }

  async attachGlobalEvidence(trackId: string, evidenceId: string) {
    await wait();
    const track = this.mutableTrack(trackId);
    const item = this.state.globalEvidence.find((entry) => entry.id === evidenceId);
    if (!item) throw new Error("Der globale Nachweis wurde nicht gefunden.");
    this.attachGlobalToTrack(track, item);
    return clone(track);
  }
}
