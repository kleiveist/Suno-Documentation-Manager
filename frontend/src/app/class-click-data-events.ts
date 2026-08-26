import { emptyEvidenceMetadata } from "../domain/types";
import type { EvidenceRole, StepId } from "../domain/types";
import { AppActions } from "./class-actions";
import { shouldIgnoreModalBackdropClick } from "./state";
import type { AppState, MainView, TrackTab } from "./state";

export abstract class AppClickDataEvents extends AppActions {
  protected clickButton(event: Event): HTMLElement | null {
    const target = event.target as HTMLElement;
    const button = target.closest<HTMLElement>(
      "button, [data-action], [data-view], [data-track-open], [data-step-open], [data-track-tab]"
    );
    if (!button) return null;
    const modalBackdrop = button.matches('.modal-backdrop[data-action="close-modal"]');
    return shouldIgnoreModalBackdropClick(modalBackdrop, target === button) ? null : button;
  }

  protected async handleAlbumCreation(event: Event, button: HTMLElement): Promise<boolean> {
    if (button.dataset.action !== "create-album") return false;
    event.preventDefault();
    event.stopPropagation();
    const title = window.prompt(this.t("Name des neuen Albumordners:"))?.trim();
    if (!title) return true;
    const albums = await this.withBusy("Albumordner wird angelegt …", () => this.api.createAlbum(title));
    if (albums) {
      this.state.albums = albums;
      this.showToast(
        "success",
        "Albumordner angelegt",
        `${title} wurde erstellt. Tracks können diesem Album jetzt zugeordnet werden.`
      );
      this.render();
    }
    return true;
  }

  protected handleSettingsCategoryClick(event: Event, button: HTMLElement): boolean {
    const settingsCategory = button.dataset.settingsCategory;
    if (settingsCategory !== "global" && settingsCategory !== "external" && settingsCategory !== "files") {
      return false;
    }
    event.preventDefault();
    this.selectSettingsCategory(settingsCategory);
    return true;
  }

  protected async handleAlbumRename(event: Event, button: HTMLElement): Promise<boolean> {
    const albumTitle = button.dataset.renameAlbum;
    if (!albumTitle) return false;
    event.preventDefault();
    event.stopPropagation();
    const newTitle = window.prompt(this.t("Neuer Name des Albumordners:"), albumTitle)?.trim();
    if (!newTitle || newTitle === albumTitle) return true;
    const result = await this.withBusy("Albumordner wird umbenannt …", async () => {
      const tracks = await this.api.renameAlbum(albumTitle, newTitle);
      const albums = await this.api.listAlbums();
      return { tracks, albums };
    });
    if (result) {
      this.state.tracks = result.tracks;
      this.state.albums = result.albums;
      const currentSummary = this.state.track
        ? result.tracks.find((track) => track.id === this.state.track!.id)
        : undefined;
      if (this.state.track && currentSummary) {
        this.state.track.library = structuredClone(currentSummary.library);
        this.state.track.relativePath = currentSummary.relativePath;
      }
      this.showToast("success", "Albumordner umbenannt", `${albumTitle} wurde in ${newTitle} umbenannt.`);
      this.render();
    }
    return true;
  }

  protected async handleNavigationDataset(button: HTMLElement): Promise<boolean> {
    const view = button.dataset.view as MainView | undefined;
    if (view) {
      if (!(await this.flushDraft())) return true;
      this.state.view = view;
      this.state.sidebarOpen = false;
      this.state.activeStep = null;
      this.render();
      return true;
    }
    const trackId = button.dataset.trackOpen;
    if (trackId) {
      if (!(await this.flushDraft())) return true;
      const track = await this.withBusy("Track wird geladen …", () => this.api.loadTrack(trackId));
      if (track) {
        this.applyTrack(track);
        this.state.view = "current";
        this.state.trackTab = "overview";
        this.state.activeStep = null;
      }
      return true;
    }
    const stepId = button.dataset.stepOpen as StepId | undefined;
    if (stepId) {
      if (!(await this.flushDraft())) return true;
      this.state.activeStep = stepId;
      this.state.view = "current";
      this.render();
      return true;
    }
    const tab = button.dataset.trackTab as TrackTab | undefined;
    if (tab) {
      if (!(await this.flushDraft())) return true;
      this.state.trackTab = tab;
      this.state.activeStep = null;
      this.render();
      return true;
    }
    const filter = button.dataset.trackFilter as AppState["trackFilter"] | undefined;
    if (!filter) return false;
    this.state.trackFilter = filter;
    this.render();
    return true;
  }

  protected async handleTrackEvidenceDataset(button: HTMLElement): Promise<boolean> {
    const importRole = button.dataset.importRole as EvidenceRole | undefined;
    if (importRole) {
      await this.importEvidence(importRole, button.dataset.replaceEvidence);
      return true;
    }
    const previewEvidenceId = button.dataset.previewEvidence;
    if (previewEvidenceId) {
      await this.previewEvidence(previewEvidenceId);
      return true;
    }
    if (button.dataset.verifyEvidence) {
      await this.trackMutation(
        "Evidence wird geprüft …",
        () => this.api.verifyEvidence(this.requireTrack().id, button.dataset.verifyEvidence),
        "Evidence verifiziert",
        true
      );
      return true;
    }
    if (!button.dataset.removeEvidence) return false;
    const selected = this.requireTrack().evidence.find((item) => item.id === button.dataset.removeEvidence);
    const prompt =
      selected?.provenance === "indexed_legacy"
        ? "Historisch indexierte Evidence entfernen? Die Datei wird nachvollziehbar unter .archive/removals gesichert und nicht gelöscht."
        : "Importierte Evidence-Kopie aus dem Track entfernen? Die Originaldatei am Quellort bleibt erhalten.";
    if (window.confirm(this.t(prompt))) {
      await this.trackMutation(
        "Evidence wird entfernt …",
        () => this.api.removeEvidence(this.requireTrack().id, button.dataset.removeEvidence!),
        "Evidence entfernt"
      );
    }
    return true;
  }

  protected async handleGlobalEvidenceDataset(button: HTMLElement): Promise<boolean> {
    if (button.dataset.removeGlobalEvidence) {
      const confirmation = this.t(
        "Global registrierten Nachweis entfernen? Bereits in Tracks kopierte Evidence bleibt bestehen."
      );
      if (window.confirm(confirmation)) {
        await this.withBusy("Nachweis wird entfernt …", () =>
          this.api.removeGlobalEvidence(button.dataset.removeGlobalEvidence!)
        );
        this.state.globalEvidence = await this.api.listGlobalEvidence();
        this.showToast("success", "Nachweis entfernt", "Bereits zugeordnete Track-Kopien wurden nicht verändert.");
      }
      return true;
    }
    if (button.dataset.editGlobalTerms) {
      const item = this.state.globalEvidence.find(
        (entry) => entry.id === button.dataset.editGlobalTerms && entry.role === "suno_terms_rights"
      );
      if (!item) return true;
      this.state.termsMetadataDialog = {
        evidenceId: item.id,
        metadata: { ...emptyEvidenceMetadata(), ...structuredClone(item.metadata) }
      };
      this.render();
      return true;
    }
    if (!button.dataset.attachGlobal) return false;
    const global = this.state.globalEvidence.find((item) => item.id === button.dataset.attachGlobal);
    const terms = global?.role === "suno_terms_rights";
    await this.trackMutation(
      terms ? "Nutzungsbedingungen werden in das Projekt kopiert …" : "Abo-Nachweis wird in den Track kopiert …",
      () => this.api.attachGlobalEvidence(this.requireTrack().id, button.dataset.attachGlobal!),
      terms ? "Nutzungsbedingungen im Projekt hinterlegt" : "Abo-Nachweis zugeordnet"
    );
    return true;
  }

  protected async handleWorkflowDataset(button: HTMLElement): Promise<boolean> {
    if (button.dataset.resolveDeviation) {
      await this.trackMutation(
        "Abweichung wird gelöst …",
        () => this.api.resolveDeviation(this.requireTrack().id, button.dataset.resolveDeviation!),
        "Abweichung gelöst"
      );
      return true;
    }
    if (button.dataset.markNa) {
      if (!(await this.flushDraft())) return true;
      const reason = window.prompt(
        this.t("Warum ist dieser Schritt für den Track nicht anwendbar? Eine konkrete Begründung ist erforderlich.")
      );
      if (!reason?.trim()) return true;
      await this.trackMutation(
        "N/A-Begründung wird gespeichert …",
        () => this.api.setStepStatus(this.requireTrack().id, button.dataset.markNa as StepId, "N_A", reason.trim()),
        "N/A dokumentiert"
      );
      return true;
    }
    if (button.dataset.resetNa) {
      await this.trackMutation(
        "Schrittstatus wird zurückgesetzt …",
        () => this.api.setStepStatus(this.requireTrack().id, button.dataset.resetNa as StepId, "NOT_RUN"),
        "Schrittstatus zurückgesetzt"
      );
      return true;
    }
    if (!button.dataset.removeDeviation) return false;
    await this.trackMutation(
      "Abweichung wird entfernt …",
      () => this.api.removeDeviation(this.requireTrack().id, button.dataset.removeDeviation!),
      "Abweichung entfernt"
    );
    return true;
  }

  protected async routeDatasetClick(event: Event, button: HTMLElement): Promise<boolean> {
    if (await this.handleAlbumCreation(event, button)) return true;
    if (this.handleSettingsCategoryClick(event, button)) return true;
    if (await this.handleAlbumRename(event, button)) return true;
    if (await this.handleNavigationDataset(button)) return true;
    if (await this.handleTrackEvidenceDataset(button)) return true;
    if (await this.handleGlobalEvidenceDataset(button)) return true;
    return this.handleWorkflowDataset(button);
  }
}
