import { emptyEvidenceMetadata } from "../domain/types";
import { AppClickDataEvents } from "./class-click-data-events";
import { missingProfileFields } from "./navigation";
import { canCreateTrackRevision } from "./state";
import { externalTimestampStatusLabel, externalTimestampSummaryFor } from "./timestamp-presentation";

export abstract class AppClickEvents extends AppClickDataEvents {
  protected async handleLibraryAction(action: string): Promise<boolean> {
    switch (action) {
      case "toggle-theme":
        this.toggleTheme();
        return true;
      case "open-workspace":
        await this.chooseWorkspace("open");
        return true;
      case "create-workspace":
        await this.chooseWorkspace("create");
        return true;
      case "new-track": {
        if (!(await this.flushDraft())) return true;
        const missing = missingProfileFields(this.state.profile);
        if (missing.length) {
          this.state.view = "settings";
          this.state.settingsCategory = "global";
          this.showToast(
            "info",
            "Zuerst Stammdaten vervollständigen",
            this.t(`Für einen unveränderlichen Track-Snapshot fehlen: ${this.localizedLabels(missing)}.`)
          );
          this.render();
        } else {
          this.state.showTrackLibrary = false;
          this.state.showSubscriptionEvidence = false;
          this.state.evidencePreview = null;
          this.state.folderImport = null;
          this.state.showNewTrack = true;
          this.render();
        }
        return true;
      }
      case "scan-folder-import": {
        const proposal = await this.withBusy("Ordner wird analysiert …", () =>
          this.api.scanImportFolder(this.language)
        );
        if (proposal) {
          this.state.folderImport = proposal;
          this.showToast(
            "info",
            "Ordner analysiert",
            `${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} erkannt. Nur eindeutige Dateien werden übernommen.`
          );
          this.render();
        }
        return true;
      }
      case "edit-track-library":
        if (this.rejectLockedContentMutation()) return true;
        if (!(await this.flushDraft())) return true;
        this.state.showNewTrack = false;
        this.state.showSubscriptionEvidence = false;
        this.state.evidencePreview = null;
        this.state.showTrackLibrary = true;
        this.render();
        return true;
      case "close-modal":
        this.state.showNewTrack = false;
        this.state.folderImport = null;
        this.state.showTrackLibrary = false;
        this.state.showSubscriptionEvidence = false;
        this.state.evidencePreview = null;
        this.state.showCertificatePopup = false;
        this.state.termsMetadataDialog = null;
        this.render();
        return true;
      case "remove-legacy-lyrics": {
        if (this.rejectLockedContentMutation()) return true;
        const confirmed = window.confirm(
          this.t(
            "Historische Lyrics-Angaben aus diesem bearbeitbaren Track entfernen? Die neuen Lyrics-/Structure-Felder bleiben unverändert."
          )
        );
        if (!confirmed) return true;
        const updated = await this.withBusy("Historische Lyrics-Angaben werden entfernt …", () =>
          this.api.updateTrack(this.requireTrack().id, { legacyLyricsSource: "", legacyLyricsText: "" })
        );
        if (updated) {
          this.applyTrack(updated);
          this.showToast(
            "success",
            "Historische Lyrics-Angaben entfernt",
            "Der Legacy-Hinweis wurde gelöscht; aktuelle Lyrics-/Structure-Angaben blieben unverändert."
          );
        }
        return true;
      }
      default:
        return false;
    }
  }

  protected async handleShellAction(action: string): Promise<boolean> {
    switch (action) {
      case "show-certificate-popup":
        if (this.requireTrack().certificate.valid && this.requireTrack().certificate.certificateId) {
          this.state.showCertificatePopup = true;
          this.render();
        }
        return true;
      case "open-certificate-tab":
        this.state.showCertificatePopup = false;
        this.state.view = "current";
        this.state.activeStep = null;
        this.state.trackTab = "certificate";
        this.render();
        return true;
      case "open-sidebar":
        this.state.sidebarOpen = true;
        this.render();
        return true;
      case "close-sidebar":
        this.state.sidebarOpen = false;
        this.render();
        return true;
      case "dismiss-toast":
        this.state.toast = null;
        this.render();
        return true;
      case "go-tracks":
        if (await this.flushDraft()) {
          this.state.view = "tracks";
          this.render();
        }
        return true;
      case "back-overview":
        if (await this.flushDraft()) {
          this.state.activeStep = null;
          this.state.trackTab = "overview";
          this.render();
        }
        return true;
      case "scan-workspace":
        await this.scanWorkspace();
        return true;
      case "adopt-legacy-profile":
        if (
          window.confirm(
            this.t(
              "Treffen die aktuellen Workspace-Stammdaten auf diesen historischen Track zu? Sie werden als Track-Snapshot übernommen."
            )
          )
        ) {
          await this.trackMutation(
            "Stammdaten werden als Legacy-Snapshot übernommen …",
            () => this.api.adoptLegacyProfile(this.requireTrack().id),
            "Legacy-Snapshot übernommen"
          );
        }
        return true;
      case "import-evidence":
        await this.chooseEvidenceRole();
        return true;
      default:
        return false;
    }
  }

  protected async testTimestampProvider(button: HTMLElement): Promise<void> {
    const form = button.closest<HTMLFormElement>("#profile-form");
    if (!form) return;
    const outcome = await this.withBusy("Timestamp-Provider wird geprüft …", () =>
      this.updateTimestampSettingsFromForm(form, true)
    );
    if (!outcome) return;
    this.state.timestampSettings = outcome.settings;
    this.state.timestampProviderTest = outcome.test;
    const successful = outcome.test?.status === "ready";
    this.showToast(
      successful ? "success" : "info",
      "Verbindungstest",
      this.systemText(outcome.test?.message ?? outcome.settings.statusMessage)
    );
  }

  protected async testAudioScreeningProvider(button: HTMLElement): Promise<void> {
    const form = button.closest<HTMLFormElement>("#profile-form");
    if (!form) return;
    const outcome = await this.withBusy("ACRCloud-Konfiguration wird geprüft …", () =>
      this.updateAudioScreeningSettingsFromForm(form, true)
    );
    if (!outcome) return;
    this.state.audioScreeningSettings = outcome.settings;
    this.state.audioScreeningProviderTest = outcome.test;
    const successful = outcome.test?.status === "ready";
    this.showToast(
      successful ? "success" : "info",
      "ACRCloud-Verbindung",
      this.systemText(outcome.test?.message ?? outcome.settings.statusMessage)
    );
  }

  protected async handleSettingsAction(action: string, button: HTMLElement): Promise<boolean> {
    switch (action) {
      case "import-global-evidence":
        this.state.showNewTrack = false;
        this.state.showTrackLibrary = false;
        this.state.evidencePreview = null;
        this.state.showSubscriptionEvidence = true;
        this.render();
        return true;
      case "import-global-terms":
        this.state.termsMetadataDialog = { evidenceId: null, metadata: emptyEvidenceMetadata() };
        this.render();
        return true;
      case "open-timestamp-settings":
      case "open-audio-screening-settings":
        if (await this.flushDraft()) {
          this.state.view = "settings";
          this.state.settingsCategory = "external";
          this.state.activeStep = null;
          this.render();
        }
        return true;
      case "test-timestamp-provider":
        await this.testTimestampProvider(button);
        return true;
      case "test-audio-screening-provider":
        await this.testAudioScreeningProvider(button);
        return true;
      default:
        return false;
    }
  }

  protected async handleTimestampAction(action: string): Promise<boolean> {
    if (action !== "attach-external-timestamp") return false;
    const track = this.requireTrack();
    const updated = await this.withBusy(
      "Externer Zeitstempel wird an den finalisierten Manifest-Anchor angehängt …",
      () => this.api.attachExternalTimestamp(track.id)
    );
    if (!updated) return true;
    this.applyTrack(updated);
    const summary = externalTimestampSummaryFor(updated);
    const successful = summary.status === "verified" || summary.status === "attached";
    this.showToast(
      successful ? "success" : "info",
      successful ? "Externer Zeitstempel angehängt" : externalTimestampStatusLabel(summary.status),
      this.systemText(summary.message)
    );
    return true;
  }

  protected async handleWorkflowAction(action: string): Promise<boolean> {
    switch (action) {
      case "add-deviation":
        await this.addDeviation();
        return true;
      case "generate-documents":
        await this.generateDocumentsSafely();
        return true;
      case "generate-disclosure":
        await this.runAction("KI-Hinweis wird lokal erzeugt …", () =>
          this.api.generateArtworkDisclosure(this.requireTrack().id, this.state.trackDraft?.disclosureText)
        );
        return true;
      case "run-local-audio-screening":
        await this.runProgressAction(
          "audio_screening",
          "Lokaler Chromaprint-Fingerprint wird erzeugt …",
          (onProgress) => this.api.runLocalAudioScreening(this.requireTrack().id, onProgress)
        );
        return true;
      case "run-external-audio-screening":
        await this.runProgressAction("audio_screening", "ACRCloud-Prüfung wird vorbereitet …", (onProgress) =>
          this.api.runExternalAudioScreening(this.requireTrack().id, onProgress)
        );
        return true;
      case "calculate-hashes":
        await this.runProgressAction("hashes", "SHA-256 wird berechnet …", (onProgress) =>
          this.api.calculateHashes(this.requireTrack().id, onProgress)
        );
        return true;
      case "verify-hashes":
        await this.runProgressAction(
          "verification",
          "Prüfsummen werden verifiziert …",
          (onProgress) => this.api.verifyHashes(this.requireTrack().id, onProgress),
          true
        );
        return true;
      case "finalize-track":
        await this.finalizeTrack();
        return true;
      default:
        return false;
    }
  }

  protected async invalidateCertificate(): Promise<void> {
    if (!canCreateTrackRevision(this.requireTrack().status)) {
      this.rejectLockedContentMutation();
      return;
    }
    if (
      !window.confirm(
        this.t("Zertifikat als ungültig markieren? Der finalisierte Snapshot wird nicht still überschrieben.")
      )
    ) {
      return;
    }
    await this.runAction(
      "Zertifikat wird invalidiert …",
      () => this.api.invalidateCertificate(this.requireTrack().id),
      true
    );
  }

  protected async createRevision(): Promise<void> {
    if (!canCreateTrackRevision(this.requireTrack().status)) {
      this.rejectLockedContentMutation();
      return;
    }
    const confirmed = window.confirm(
      this.t(
        "Neue Revision anlegen? Der bisherige Certificate-/Manifest-Snapshot wird zuerst unter .archive/revisions gesichert."
      )
    );
    if (!confirmed) return;
    await this.runAction("Neue Revision wird angelegt …", () => this.api.createRevision(this.requireTrack().id), true);
  }

  protected async reEvaluateTrack(): Promise<void> {
    if (this.requireTrack().status === "SUPERSEDED") {
      this.rejectLockedContentMutation();
      return;
    }
    const confirmed = window.confirm(
      this.t(
        "Track mit dem aktuellen Workflow neu bewerten? Ein finalisierter Snapshot wird zuerst unverändert als Revision archiviert; Dokumente, Prüfsummen und Zertifikat müssen danach neu erzeugt werden."
      )
    );
    if (!confirmed) return;
    await this.runAction(
      "Workflow wird aktualisiert und neu bewertet …",
      () => this.api.reEvaluateTrack(this.requireTrack().id),
      this.requireTrack().status === "FINALIZED"
    );
  }

  protected async handleRevisionAction(action: string): Promise<boolean> {
    switch (action) {
      case "invalidate-certificate":
        await this.invalidateCertificate();
        return true;
      case "create-revision":
        await this.createRevision();
        return true;
      case "re-evaluate-track":
        await this.reEvaluateTrack();
        return true;
      default:
        return false;
    }
  }

  protected async routeActionClick(action: string, button: HTMLElement): Promise<void> {
    if (await this.handleLibraryAction(action)) return;
    if (await this.handleShellAction(action)) return;
    if (await this.handleSettingsAction(action, button)) return;
    if (await this.handleTimestampAction(action)) return;
    if (await this.handleWorkflowAction(action)) return;
    await this.handleRevisionAction(action);
  }

  protected async handleClick(event: Event): Promise<void> {
    const button = this.clickButton(event);
    if (!button) return;
    if (await this.routeDatasetClick(event, button)) return;
    await this.routeActionClick(button.dataset.action ?? "", button);
  }
}
