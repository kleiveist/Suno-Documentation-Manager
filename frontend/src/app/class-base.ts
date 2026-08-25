import { toUserMessage } from "../api/desktop";
import type { DesktopApi } from "../api/desktop";
import { emptyAudioScreeningSettings, emptyProfile, emptyTimestampSettings } from "../domain/types";
import type {
  DocumentationAnswer,
  EvidenceRole,
  FactOrigin,
  GlobalProfile,
  OperationProgress,
  StepId,
  TrackCoverPreview,
  TrackDetail,
  WorkspaceSummary
} from "../domain/types";
import { escapeHtml, formatBytes } from "../ui/format";
import { hasUiTranslation, translateUiText } from "../ui/i18n";
import type { AppLanguage } from "../ui/i18n";
import { icon } from "../ui/icons";
import { resolveTheme, THEME_STORAGE_KEY, toggledTheme } from "../ui/theme";
import type { GuidedChoice, SingleChoiceOption } from "./choices";
import { OPERATION_PROGRESS_CONFIGURATION, operationProgressPercent, operationStageLabel } from "./progress";
import type { LongOperationKind } from "./progress";
import { collectUserText, WORKSPACE_PATH_STORAGE_KEY } from "./state";
import type { ActiveOperationProgress, AppState, SettingsCategory, ToastKind } from "./state";

export abstract class AppBase {
  protected readonly state: AppState = {
    workspace: null,
    profile: { ...emptyProfile },
    timestampSettings: { ...emptyTimestampSettings, custom: { ...emptyTimestampSettings.custom } },
    timestampProviderTest: null,
    audioScreeningSettings: { ...emptyAudioScreeningSettings },
    audioScreeningProviderTest: null,
    tracks: [],
    albums: [],
    track: null,
    workflow: null,
    globalEvidence: [],
    view: "dashboard",
    settingsCategory: "global",
    trackTab: "overview",
    activeStep: null,
    trackDraft: null,
    scanResult: null,
    query: "",
    trackFilter: "all",
    busy: false,
    busyLabel: "",
    operationProgress: null,
    sidebarOpen: false,
    showNewTrack: false,
    folderImport: null,
    showTrackLibrary: false,
    showSubscriptionEvidence: false,
    evidencePreview: null,
    showCertificatePopup: false,
    termsMetadataDialog: null,
    theme: "light",
    toast: null
  };

  protected toastTimer: number | undefined;

  protected operationTimer: number | undefined;

  protected draftDirty = false;

  protected followsSystemTheme = true;

  protected systemThemeQuery: MediaQueryList | null = null;

  protected readonly trackCoverCache = new Map<string, TrackCoverPreview>();

  protected trackCoverGeneration = 0;

  constructor(
    protected readonly root: HTMLElement,
    protected readonly api: DesktopApi
  ) {}

  protected get language(): AppLanguage {
    return this.state.profile.certificateLanguage === "de" ? "de" : "en";
  }

  protected get numberLocale(): string {
    return this.language === "de" ? "de-DE" : "en-US";
  }

  /** Localize known system copy only; free user content stays untouched. */
  protected t(value: string): string {
    return translateUiText(value, this.language);
  }

  protected localizedLabels(values: readonly string[]): string {
    return values.map((value) => this.t(value)).join(", ");
  }

  /**
   * Hashing/document operations use `currentFile` for an actual path. Native
   * audio screening alone uses it for a fixed progress sentence, which can be
   * localized safely without ever changing a user-selected filename.
   */
  protected progressCurrentFile(operation: ActiveOperationProgress): string {
    const value = operation.progress.currentFile ?? "";
    return operation.kind === "audio_screening" && value ? this.systemText(value) : value;
  }

  /**
   * Backend results are system copy, not user text. Known messages are mapped
   * exactly; an unmapped low-level diagnostic is replaced with locale-safe
   * copy so a German screen never leaks an English warning (or vice versa).
   */
  protected systemText(value: string, fallback?: string): string {
    if (hasUiTranslation(value, this.language)) return this.t(value);
    return (
      fallback ??
      (this.language === "en"
        ? "Technical status detail is not available."
        : "Technische Statusmeldung ist nicht verfügbar.")
    );
  }

  /**
   * `translateRenderedUi` is retained for legacy static templates. Supply it
   * only data-owned values so a track title, file name, lyric, or free text
   * that happens to equal a catalog entry is never localized as UI copy.
   */
  protected protectedRenderedValues(): ReadonlySet<string> {
    const values = new Set<string>();
    const track = this.state.track;

    collectUserText(
      values,
      this.state.workspace && {
        name: this.state.workspace.name,
        path: this.state.workspace.path
      }
    );
    collectUserText(values, this.state.profile);
    collectUserText(
      values,
      this.state.tracks.map(({ title, relativePath, library }) => ({ title, relativePath, library }))
    );
    collectUserText(values, this.state.albums);
    collectUserText(values, this.state.trackDraft);
    collectUserText(
      values,
      this.state.globalEvidence.map(({ fileName, relativePath, metadata }) => ({ fileName, relativePath, metadata }))
    );
    collectUserText(values, this.state.termsMetadataDialog?.metadata);
    collectUserText(values, this.state.query);
    collectUserText(values, this.state.operationProgress?.progress.currentFile);
    collectUserText(
      values,
      this.state.scanResult?.candidates?.map(({ name, relativePath }) => ({ name, relativePath }))
    );
    collectUserText(
      values,
      this.state.evidencePreview && {
        fileName: this.state.evidencePreview.fileName,
        relativePath: this.state.evidencePreview.relativePath,
        textContent: this.state.evidencePreview.textContent
      }
    );
    collectUserText(
      values,
      this.state.folderImport && {
        sourcePath: this.state.folderImport.sourcePath,
        albumTitle: this.state.folderImport.albumTitle,
        unassignedFiles: this.state.folderImport.unassignedFiles,
        tracks: this.state.folderImport.tracks.map(({ title, sourcePath, files, unassignedFiles }) => ({
          title,
          sourcePath,
          files: files.map(({ fileName }) => fileName),
          unassignedFiles
        }))
      }
    );
    collectUserText(values, {
      timestampSettings: {
        custom: this.state.timestampSettings.custom
      },
      audioScreeningSettings: {
        host: this.state.audioScreeningSettings.host,
        localEngineVersion: this.state.audioScreeningSettings.localEngineVersion
      }
    });

    if (track) {
      const { consistencyIssues: _consistencyIssues, ...automation } = track.automation;
      void _consistencyIssues;
      collectUserText(values, {
        title: track.title,
        relativePath: track.relativePath,
        library: track.library,
        profileSnapshot: track.profileSnapshot,
        fields: track.fields,
        automation,
        evidence: track.evidence.map(({ fileName, relativePath, metadata, generatedDisclosureText }) => ({
          fileName,
          relativePath,
          metadata,
          generatedDisclosureText
        })),
        documents: track.documents,
        certificate: track.certificate,
        integrityMismatchFiles: track.integrity.mismatchFiles,
        externalTimestamps: track.externalTimestamps,
        deviations: track.blockingDeviations?.map(({ title, description }) => ({ title, description })),
        audioMatches: track.audioScreening.external.matches,
        audioSampleMatches: track.audioScreening.external.samples.map(
          ({ matches, responseRelativePath, responseSha256 }) => ({
            matches,
            responseRelativePath,
            responseSha256
          })
        ),
        audioSourcePaths: {
          local: track.audioScreening.local.sourceRelativePath,
          external: track.audioScreening.external.sourceRelativePath
        }
      });
    }
    return values;
  }

  protected syncDocumentLanguage(): void {
    const document = this.root.ownerDocument;
    document.documentElement.lang = this.language;
    const description = document.querySelector<HTMLMetaElement>('meta[name="description"]');
    description?.setAttribute(
      "content",
      this.t("Lokale Desktop-App für nachvollziehbare Suno-Track-Dokumentation und Integritätsprüfung")
    );
  }

  start(): void {
    this.initializeTheme();
    this.root.addEventListener("click", (event) => void this.handleClick(event));
    this.root.addEventListener("submit", (event) => void this.handleSubmit(event));
    this.root.addEventListener("change", (event) => this.handleChange(event));
    this.root.addEventListener("input", (event) => this.handleInput(event));
    this.render();
    void this.restoreSavedWorkspace();
  }

  protected async restoreSavedWorkspace(): Promise<void> {
    const path = this.loadWorkspacePathFromStorage();
    if (!path) return;
    try {
      const workspace = await this.api.restoreWorkspace(path);
      await this.enterWorkspace(workspace);
    } catch (error) {
      const message = toUserMessage(error, this.language);
      const pathUnavailable = /not found|does not exist|existiert nicht|nicht gefunden|keine Datei|no such file/i.test(
        message
      );
      if (pathUnavailable) {
        this.clearWorkspacePathFromStorage();
      }
      this.showToast(
        pathUnavailable ? "info" : "error",
        pathUnavailable ? "Gespeicherter Workspace nicht verfügbar" : "Workspace konnte nicht geladen werden",
        pathUnavailable
          ? "Der zuletzt verwendete Workspace wurde nicht mehr gefunden. Bitte wähle einen anderen Workspace."
          : this.t(`Gespeicherter Workspace konnte nicht geladen werden: ${message}`)
      );
    }
  }

  protected loadWorkspacePathFromStorage(): string | null {
    try {
      const path = window.localStorage.getItem(WORKSPACE_PATH_STORAGE_KEY);
      return path?.trim() || null;
    } catch {
      return null;
    }
  }

  protected saveWorkspacePathToStorage(path: string): void {
    const trimmed = path.trim();
    if (!trimmed) return;
    try {
      window.localStorage.setItem(WORKSPACE_PATH_STORAGE_KEY, trimmed);
    } catch {
      // Keep the current session usable even if storage is blocked.
    }
  }

  protected clearWorkspacePathFromStorage(): void {
    try {
      window.localStorage.removeItem(WORKSPACE_PATH_STORAGE_KEY);
    } catch {
      // Keep the current session usable even if storage is blocked.
    }
  }

  protected initializeTheme(): void {
    let stored: string | null = null;
    try {
      stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    } catch {
      // The selected theme still works for this session if storage is unavailable.
    }
    this.followsSystemTheme = stored !== "light" && stored !== "dark";
    this.systemThemeQuery =
      typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
    this.state.theme = resolveTheme(stored, this.systemThemeQuery?.matches ?? false);
    this.applyTheme();
    this.systemThemeQuery?.addEventListener("change", (event) => {
      if (!this.followsSystemTheme) return;
      this.state.theme = event.matches ? "dark" : "light";
      this.applyTheme();
    });
  }

  protected applyTheme(): void {
    document.documentElement.dataset.theme = this.state.theme;
    document.documentElement.style.colorScheme = this.state.theme;
    document
      .querySelector<HTMLMetaElement>('meta[name="theme-color"]')
      ?.setAttribute("content", this.state.theme === "dark" ? "#111310" : "#f4f2ed");
    const dark = this.state.theme === "dark";
    const label = translateUiText(dark ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren", this.language);
    const title = translateUiText(dark ? "Heller Modus" : "Dunkler Modus", this.language);
    this.root.querySelectorAll<HTMLElement>('[data-action="toggle-theme"]').forEach((control) => {
      control.setAttribute("aria-label", label);
      control.setAttribute("aria-pressed", String(dark));
      control.setAttribute("title", title);
      control.innerHTML = icon(dark ? "sun" : "moon");
    });
  }

  protected toggleTheme(): void {
    this.state.theme = toggledTheme(this.state.theme);
    this.followsSystemTheme = false;
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, this.state.theme);
    } catch {
      // Keep the active session usable even if persistence is blocked.
    }
    this.applyTheme();
  }

  protected selectSettingsCategory(category: SettingsCategory): void {
    this.state.settingsCategory = category;
    this.root.querySelectorAll<HTMLElement>("[data-settings-category-panel]").forEach((panel) => {
      panel.hidden = panel.dataset.settingsCategoryPanel !== category;
    });
    this.root.querySelectorAll<HTMLButtonElement>("[data-settings-category]").forEach((control) => {
      const selected = control.dataset.settingsCategory === category;
      control.classList.toggle("is-active", selected);
      control.setAttribute("aria-selected", String(selected));
    });
    const settingsForm = this.root.querySelector<HTMLFormElement>("#profile-form");
    if (settingsForm) settingsForm.hidden = category === "files";
  }

  protected async withBusy<T>(label: string, action: () => Promise<T>): Promise<T | undefined> {
    this.state.busy = true;
    this.state.busyLabel = label;
    this.state.operationProgress = null;
    this.render();
    try {
      return await action();
    } catch (error) {
      this.showToast("error", "Aktion nicht abgeschlossen", toUserMessage(error, this.language));
      return undefined;
    } finally {
      this.state.busy = false;
      this.state.busyLabel = "";
      this.state.operationProgress = null;
      this.render();
    }
  }

  protected async withOperationProgress<T>(
    kind: LongOperationKind,
    label: string,
    action: (onProgress: (progress: OperationProgress) => void) => Promise<T>
  ): Promise<T | undefined> {
    const initialStage =
      kind === "documents"
        ? "preparing_documents"
        : kind === "hashes"
          ? "discovering_files"
          : kind === "finalization"
            ? "validating_finalization_gate"
            : kind === "audio_screening"
              ? "preparing_audio"
              : "reading_hash_list";
    this.state.busy = true;
    this.state.busyLabel = label;
    this.state.operationProgress = {
      kind,
      elapsedSeconds: 0,
      progress: { stage: initialStage, processedBytes: 0, totalBytes: 0, processedFiles: 0, totalFiles: 0 }
    };
    this.render();
    let active = true;
    const startedAt = Date.now();
    window.clearInterval(this.operationTimer);
    this.operationTimer = window.setInterval(() => {
      if (!active || !this.state.operationProgress) return;
      this.state.operationProgress.elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
      this.syncBusyLayer();
    }, 1000);
    try {
      return await action((progress) => {
        if (!active || !this.state.operationProgress) return;
        this.state.operationProgress.progress = progress;
        this.state.operationProgress.elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
        this.syncBusyLayer();
      });
    } catch (error) {
      this.showToast("error", "Aktion nicht abgeschlossen", toUserMessage(error, this.language));
      return undefined;
    } finally {
      active = false;
      window.clearInterval(this.operationTimer);
      this.operationTimer = undefined;
      this.state.busy = false;
      this.state.busyLabel = "";
      this.state.operationProgress = null;
      this.render();
    }
  }

  protected syncBusyLayer(): void {
    const operation = this.state.operationProgress;
    const current = this.root.querySelector<HTMLElement>(".busy-layer--operation");
    if (!operation || !current || current.dataset.operationKind !== operation.kind) {
      this.render();
      return;
    }

    // Keep the layer alive while progress changes. Replacing the complete
    // element here restarts every CSS keyframe and makes the artwork jump.
    const progress = operation.progress;
    const configuration = OPERATION_PROGRESS_CONFIGURATION[operation.kind];
    const percent = operationProgressPercent(operation.kind, progress);
    const detail =
      progress.totalBytes > 0
        ? this.language === "en"
          ? `${formatBytes(progress.processedBytes, this.language)} of ${formatBytes(progress.totalBytes, this.language)} · ${progress.processedFiles}/${progress.totalFiles} files`
          : `${formatBytes(progress.processedBytes, this.language)} von ${formatBytes(progress.totalBytes, this.language)} · ${progress.processedFiles}/${progress.totalFiles} Dateien`
        : progress.totalFiles > 0
          ? this.language === "en"
            ? `${progress.processedFiles} of ${progress.totalFiles} files`
            : `${progress.processedFiles} von ${progress.totalFiles} Dateien`
          : this.language === "en"
            ? "Preparing file set"
            : "Dateisatz wird vorbereitet";
    const minutes = Math.floor(operation.elapsedSeconds / 60);
    const seconds = String(operation.elapsedSeconds % 60).padStart(2, "0");
    const activeStep = configuration.thresholds.reduce<number>(
      (result, threshold, index) => (percent >= threshold ? index : result),
      0
    );
    const tip = configuration.tips[Math.floor(operation.elapsedSeconds / 5) % configuration.tips.length];

    const elapsed = current.querySelector<HTMLTimeElement>('[data-operation-value="elapsed"]');
    if (elapsed) {
      elapsed.textContent = `${minutes}:${seconds}`;
      elapsed.dateTime = `PT${operation.elapsedSeconds}S`;
    }
    const percentLabel = current.querySelector<HTMLElement>('[data-operation-value="percent"]');
    if (percentLabel) percentLabel.textContent = `${percent}%`;
    const stageLabel = current.querySelector<HTMLElement>('[data-operation-value="stage"]');
    if (stageLabel)
      stageLabel.textContent = translateUiText(operationStageLabel(progress.stage, operation.kind), this.language);
    const detailLabel = current.querySelector<HTMLElement>('[data-operation-value="detail"]');
    if (detailLabel) detailLabel.textContent = detail;

    const currentFile = current.querySelector<HTMLElement>('[data-operation-value="file"]');
    if (currentFile) {
      const fileName = progress.currentFile ?? "";
      const filePresentation = this.progressCurrentFile(operation);
      currentFile.textContent = filePresentation;
      currentFile.title = filePresentation;
      currentFile.hidden = fileName.length === 0;
    }

    const meter = current.querySelector<HTMLElement>('[data-operation-value="meter"]');
    meter?.setAttribute("aria-valuenow", String(percent));
    const meterBar = current.querySelector<HTMLElement>('[data-operation-value="meter-bar"]');
    if (meterBar) meterBar.style.width = `${percent}%`;

    current.querySelectorAll<HTMLElement>("[data-operation-step]").forEach((step, index) => {
      const complete = index < activeStep;
      step.classList.toggle("is-complete", complete);
      step.classList.toggle("is-active", index === activeStep);
      const badge = step.querySelector<HTMLElement>("[data-operation-step-badge]");
      const nextState = complete ? "complete" : "pending";
      if (badge && badge.dataset.operationStepBadge !== nextState) {
        badge.dataset.operationStepBadge = nextState;
        badge.innerHTML = complete ? icon("check") : String(index + 1);
      }
    });

    const tipLabel = current.querySelector<HTMLElement>('[data-operation-value="tip"]');
    if (tipLabel) tipLabel.textContent = translateUiText(tip, this.language);
  }

  protected renderBusyLayer(): string {
    if (!this.state.busy) return "";
    const operation = this.state.operationProgress;
    if (!operation) {
      return `<div class="busy-layer" role="status" aria-live="polite"><span class="spinner"></span><span>${escapeHtml(translateUiText(this.state.busyLabel, this.language))}</span></div>`;
    }
    const progress = operation.progress;
    const currentFile = this.progressCurrentFile(operation);
    const percent = operationProgressPercent(operation.kind, progress);
    const configuration = OPERATION_PROGRESS_CONFIGURATION[operation.kind];
    const detail =
      progress.totalBytes > 0
        ? this.language === "en"
          ? `${formatBytes(progress.processedBytes, this.language)} of ${formatBytes(progress.totalBytes, this.language)} · ${progress.processedFiles}/${progress.totalFiles} files`
          : `${formatBytes(progress.processedBytes, this.language)} von ${formatBytes(progress.totalBytes, this.language)} · ${progress.processedFiles}/${progress.totalFiles} Dateien`
        : progress.totalFiles > 0
          ? this.language === "en"
            ? `${progress.processedFiles} of ${progress.totalFiles} files`
            : `${progress.processedFiles} von ${progress.totalFiles} Dateien`
          : this.language === "en"
            ? "Preparing file set"
            : "Dateisatz wird vorbereitet";
    const minutes = Math.floor(operation.elapsedSeconds / 60);
    const seconds = String(operation.elapsedSeconds % 60).padStart(2, "0");
    const activeStep = configuration.thresholds.reduce<number>(
      (result, threshold, index) => (percent >= threshold ? index : result),
      0
    );
    const tip = configuration.tips[Math.floor(operation.elapsedSeconds / 5) % configuration.tips.length];
    return `<div class="busy-layer busy-layer--operation" data-operation-kind="${operation.kind}" data-operation-theme="${this.state.theme}" role="status" aria-live="polite" aria-busy="true">
          <section class="operation-progress operation-progress--${operation.kind}" aria-label="${escapeHtml(configuration.title)}">
            <header><div><p class="overline">${escapeHtml(configuration.eyebrow)}</p><h2>${escapeHtml(configuration.title)}</h2></div><time data-operation-value="elapsed" datetime="PT${operation.elapsedSeconds}S">${minutes}:${seconds}</time></header>
            <div class="operation-stage">
              <div class="operation-orbit" aria-hidden="true"><i></i><i></i><i></i><span>${icon(configuration.iconName)}</span><b data-operation-value="percent">${percent}%</b></div>
              <div class="operation-stream" aria-hidden="true"><i>01</i><i>a7</i><i>f3</i><i>9c</i><i>42</i><i>e8</i></div>
            </div>
            <div class="operation-status"><strong data-operation-value="stage">${escapeHtml(operationStageLabel(progress.stage, operation.kind))}</strong><span data-operation-value="detail">${escapeHtml(detail)}</span><code data-operation-value="file" title="${escapeHtml(currentFile)}"${progress.currentFile ? "" : " hidden"}>${escapeHtml(currentFile)}</code></div>
            <div class="operation-meter" data-operation-value="meter" role="progressbar" aria-label="Fortschritt" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${percent}"><i data-operation-value="meter-bar" style="width:${percent}%"></i></div>
            <ol class="operation-steps">${configuration.steps.map((step, index) => `<li data-operation-step class="${index < activeStep ? "is-complete" : index === activeStep ? "is-active" : ""}"><span data-operation-step-badge="${index < activeStep ? "complete" : "pending"}">${index < activeStep ? icon("check") : index + 1}</span><strong>${escapeHtml(step)}</strong></li>`).join("")}</ol>
            <div class="operation-tip">${icon("info")}<p><strong>Währenddessen</strong><span data-operation-value="tip">${escapeHtml(tip)}</span></p></div>
            <p class="operation-footnote">${icon("lock")} Lokal und nachvollziehbar · Bitte Workspace und Datenträger verbunden lassen.</p>
          </section>
        </div>`;
  }

  protected showToast(kind: ToastKind, title: string, message: string): void {
    this.state.toast = { kind, title, message };
    // Most mutations finish after `withBusy` has already rendered its final
    // frame. Render the result immediately so workflow ticks and messages are
    // never delayed until the next interaction.
    this.render();
    window.clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(
      () => {
        this.state.toast = null;
        this.render();
      },
      kind === "error" ? 8000 : 4500
    );
  }

  protected abstract automatedDateField(
    name: string,
    label: string,
    value: string,
    origin: FactOrigin,
    required?: boolean,
    fallbackCaption?: string
  ): string;
  protected abstract automatedTextField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    origin: FactOrigin
  ): string;
  protected abstract boolQuestion(name: string, label: string, help: string, value: boolean | null): string;
  protected abstract dateField(name: string, label: string, value: string, required?: boolean): string;
  protected abstract documentationAnswerQuestion(
    name: string,
    label: string,
    value: DocumentationAnswer | null
  ): string;
  protected abstract documentationBooleanQuestion(
    name: string,
    label: string,
    help: string,
    value: boolean | null
  ): string;
  protected abstract emptyState(
    iconName: "tracks" | "current" | "file",
    title: string,
    copy: string,
    action?: string,
    actionLabel?: string
  ): string;
  protected abstract enterWorkspace(workspace: WorkspaceSummary): Promise<void>;
  protected abstract filenameConfirmation(
    track: TrackDetail,
    role: EvidenceRole,
    field: "releaseFilenameDifferenceConfirmed" | "sunoExportFilenameDifferenceConfirmed",
    confirmed: boolean | null,
    label: string
  ): string;
  protected abstract guidedSingleChoiceField(
    name: string,
    label: string,
    value: string,
    choices: readonly GuidedChoice[],
    required?: boolean
  ): string;
  protected abstract handleChange(event: Event): void;
  protected abstract handleClick(event: Event): Promise<void>;
  protected abstract handleInput(event: Event): void;
  protected abstract handleSubmit(event: SubmitEvent): Promise<void>;
  protected abstract inlineEvidenceActions(track: TrackDetail, actions: Array<[EvidenceRole, string]>): string;
  protected abstract multiChoiceArrayField(
    name: string,
    label: string,
    value: readonly string[],
    options: readonly GuidedChoice[],
    required?: boolean
  ): string;
  protected abstract multiChoiceField(
    name: string,
    label: string,
    value: string,
    options: readonly GuidedChoice[],
    required?: boolean
  ): string;
  protected abstract policyLabel(policy: GlobalProfile["artworkTransparencyPolicy"]): string;
  protected abstract radioCards(name: string, value: string, options: Array<[string, string, string]>): string;
  protected abstract render(): void;
  protected abstract renderAutomaticSunoMetadata(track: TrackDetail): string;
  protected abstract renderCertificate(track: TrackDetail): string;
  protected abstract renderDashboard(): string;
  protected abstract renderEvidence(track: TrackDetail, embedded?: boolean): string;
  protected abstract renderFinalization(track: TrackDetail): string;
  protected abstract renderSettings(): string;
  protected abstract renderStepContent(track: TrackDetail, stepId: StepId): string;
  protected abstract renderTrack(): string;
  protected abstract renderTracks(): string;
  protected abstract renderWorkspace(): string;
  protected abstract selectField(
    name: string,
    label: string,
    value: string,
    options: readonly SingleChoiceOption[],
    required?: boolean
  ): string;
  protected abstract suggestedTextField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    suggestions: readonly string[],
    required?: boolean
  ): string;
  protected abstract textArea(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    required?: boolean
  ): string;
  protected abstract textField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    required?: boolean,
    type?: string
  ): string;
}
