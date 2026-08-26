import { subscriptionCoverageEnd } from "../domain/subscription";
import type {
  FolderImportProposal,
  TrackDetail,
  TrackLibraryAssignment,
  TrackSummary,
  WorkspaceSummary
} from "../domain/types";
import { evidenceRoleLabel } from "../domain/workflow";
import { escapeHtml, formatBytes, formatDate, titleInitials } from "../ui/format";
import { translateRenderedUi, translateUiText } from "../ui/i18n";
import { icon } from "../ui/icons";
import { AppBase } from "./class-base";
import { MAIN_NAVIGATION } from "./navigation";
import { resetWorkspaceScopedUiState } from "./state";
import type { MainView } from "./state";
import { trackSummaryFromDetail } from "./track-presentation";

export abstract class AppWorkspaceShell extends AppBase {
  protected async enterWorkspace(workspace: WorkspaceSummary): Promise<void> {
    this.state.workspace = workspace;
    this.trackCoverGeneration += 1;
    this.trackCoverCache.clear();
    // The native command has already switched its authoritative workspace.
    // Clear every workspace-scoped selection before loading the new index so a
    // track from the previous workspace can never be rendered or mutated here.
    const { draftDirty, ...resetState } = resetWorkspaceScopedUiState({
      track: this.state.track,
      trackDraft: this.state.trackDraft,
      activeStep: this.state.activeStep,
      trackTab: this.state.trackTab,
      scanResult: this.state.scanResult,
      albums: this.state.albums,
      showNewTrack: this.state.showNewTrack,
      folderImport: this.state.folderImport,
      showTrackLibrary: this.state.showTrackLibrary,
      showSubscriptionEvidence: this.state.showSubscriptionEvidence,
      evidencePreview: this.state.evidencePreview,
      showCertificatePopup: this.state.showCertificatePopup,
      termsMetadataDialog: this.state.termsMetadataDialog,
      timestampSettings: this.state.timestampSettings,
      timestampProviderTest: this.state.timestampProviderTest,
      audioScreeningSettings: this.state.audioScreeningSettings,
      audioScreeningProviderTest: this.state.audioScreeningProviderTest,
      query: this.state.query,
      trackFilter: this.state.trackFilter,
      draftDirty: this.draftDirty
    });
    Object.assign(this.state, resetState);
    this.draftDirty = draftDirty;
    const loaded = await this.withBusy("Workspace wird eingelesen …", async () => {
      const [profile, timestampSettings, audioScreeningSettings, tracks, albums, workflow, globalEvidence] =
        await Promise.all([
          this.api.getProfile(),
          this.api.getTimestampSettings(),
          this.api.getAudioScreeningSettings(),
          this.api.listTracks(),
          this.api.listAlbums(),
          this.api.getWorkflow(),
          this.api.listGlobalEvidence()
        ]);
      return { profile, timestampSettings, audioScreeningSettings, tracks, albums, workflow, globalEvidence };
    });
    if (!loaded) {
      this.state.workspace = null;
      return;
    }
    this.saveWorkspacePathToStorage(workspace.path);
    this.state.profile = loaded.profile;
    this.state.timestampSettings = loaded.timestampSettings;
    this.state.audioScreeningSettings = loaded.audioScreeningSettings;
    this.state.tracks = loaded.tracks;
    this.state.albums = loaded.albums;
    this.state.workflow = loaded.workflow;
    this.state.globalEvidence = loaded.globalEvidence;
    this.state.view = "dashboard";
    void this.hydrateTrackCovers(loaded.tracks);
  }

  protected async refreshTracks(): Promise<void> {
    const [tracks, albums] = await Promise.all([this.api.listTracks(), this.api.listAlbums()]);
    this.state.tracks = tracks;
    this.state.albums = albums;
    if (this.state.track) {
      this.state.track = await this.api.loadTrack(this.state.track.id);
      this.state.trackDraft = structuredClone(this.state.track.fields);
    }
    void this.hydrateTrackCovers(tracks);
  }

  protected applyTrack(track: TrackDetail): void {
    this.state.track = track;
    this.state.trackDraft = structuredClone(track.fields);
    this.draftDirty = false;
    const albumTitle = track.library.section === "album" ? track.library.albumTitle?.trim() : "";
    if (
      albumTitle &&
      !this.state.albums.some(
        (title) =>
          title.normalize("NFKC").localeCompare(albumTitle.normalize("NFKC"), "de", { sensitivity: "base" }) === 0
      )
    ) {
      this.state.albums.push(albumTitle);
      this.state.albums.sort((left, right) => left.localeCompare(right, "de", { sensitivity: "base", numeric: true }));
    }
    const summaryIndex = this.state.tracks.findIndex((item) => item.id === track.id);
    const summary = trackSummaryFromDetail(track);
    const cachedCover = this.trackCoverCache.get(track.id);
    if (!summary.coverEvidenceId || cachedCover?.evidenceId !== summary.coverEvidenceId) {
      this.trackCoverCache.delete(track.id);
    }
    if (summaryIndex >= 0) this.state.tracks[summaryIndex] = summary;
    else this.state.tracks.unshift(summary);
    void this.hydrateTrackCovers([summary]);
  }

  protected async hydrateTrackCovers(tracks: TrackSummary[]): Promise<void> {
    const generation = this.trackCoverGeneration;
    const workspaceId = this.state.workspace?.id;
    const queue = tracks.filter((track) => {
      if (!track.coverEvidenceId) {
        this.trackCoverCache.delete(track.id);
        return false;
      }
      return this.trackCoverCache.get(track.id)?.evidenceId !== track.coverEvidenceId;
    });
    const worker = async (): Promise<void> => {
      let track: TrackSummary | undefined;
      while ((track = queue.shift())) {
        const expectedEvidenceId = track.coverEvidenceId;
        if (!expectedEvidenceId) continue;
        try {
          const cover = await this.api.loadTrackCover(track.id);
          const current = this.state.tracks.find((item) => item.id === track!.id);
          if (
            generation !== this.trackCoverGeneration ||
            workspaceId !== this.state.workspace?.id ||
            !cover ||
            cover.evidenceId !== expectedEvidenceId ||
            current?.coverEvidenceId !== expectedEvidenceId
          ) {
            continue;
          }
          this.trackCoverCache.set(track.id, cover);
          this.revealTrackCover(track.id, cover.dataUrl);
        } catch {
          // A cover is supplemental presentation. Keep the stable initials fallback
          // when its managed image cannot be decoded without interrupting workspace use.
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(queue.length, 3) }, () => worker()));
  }

  protected revealTrackCover(trackId: string, dataUrl: string): void {
    this.root.querySelectorAll<HTMLElement>("[data-track-cover]").forEach((cover) => {
      if (cover.dataset.trackCover !== trackId) return;
      const image = cover.querySelector<HTMLImageElement>(".track-cover__image");
      const fallback = cover.querySelector<HTMLElement>(".track-cover__fallback");
      if (!image || !fallback) return;
      image.src = dataUrl;
      image.hidden = false;
      fallback.hidden = true;
      cover.classList.add("has-artwork");
    });
  }

  protected render(): void {
    this.syncDocumentLanguage();
    if (!this.state.workspace) {
      this.root.innerHTML = this.renderWelcome();
      translateRenderedUi(this.root, this.language, this.protectedRenderedValues());
      return;
    }
    const view = this.renderCurrentView();
    this.root.innerHTML = `
        <div class="app-shell ${this.state.sidebarOpen ? "sidebar-is-open" : ""}">
          ${this.renderSidebar()}
          <div class="sidebar-scrim" data-action="close-sidebar"></div>
          <main class="main-shell">
            ${this.renderTopbar()}
            <div class="view-shell">${view}</div>
          </main>
          ${
            this.state.showCertificatePopup
              ? this.renderCertificatePopupDialog()
              : this.state.termsMetadataDialog
                ? this.renderTermsMetadataDialog()
                : this.state.showNewTrack
                  ? this.renderNewTrackDialog()
                  : this.state.showTrackLibrary
                    ? this.renderTrackLibraryDialog()
                    : this.state.showSubscriptionEvidence
                      ? this.renderSubscriptionEvidenceDialog()
                      : this.state.evidencePreview
                        ? this.renderEvidencePreviewDialog()
                        : ""
          }
          ${this.renderToast()}
          ${this.renderBusyLayer()}
        </div>`;
    translateRenderedUi(this.root, this.language, this.protectedRenderedValues());
  }

  protected renderWelcome(): string {
    return `<main class="welcome-shell">
        <div class="welcome-grid"></div>
        <section class="welcome-card" aria-labelledby="welcome-title">
          <div class="brand-mark brand-mark--large" aria-hidden="true"><span></span><span></span><span></span><span></span></div>
          <p class="overline">Suno Documentation Manager</p>
          <h1 id="welcome-title">Deine Musik.<br><em>Sauber dokumentiert.</em></h1>
          <p class="welcome-copy">Ein lokaler Arbeitsbereich für nachvollziehbare Evidence, klare Integritätsprüfungen und portable Track-Dokumentation.</p>
          <div class="welcome-actions">
            <button class="button button--primary button--large" data-action="open-workspace">${icon("workspace")} Workspace auswählen</button>
            <button class="button button--secondary button--large" data-action="create-workspace">${icon("plus")} Neuen Workspace anlegen</button>
          </div>
          <div class="local-promise">${icon("shield")} <span><strong>Vollständig lokal.</strong> Keine Cloud, kein Login, keine Telemetrie.</span></div>
        </section>
        <footer class="welcome-footer"><span>Version 0.1</span><span>•</span><span>Offline by design</span>${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : ""}<button class="welcome-theme-toggle" data-action="toggle-theme" aria-label="${this.state.theme === "dark" ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren"}" aria-pressed="${this.state.theme === "dark"}" title="${this.state.theme === "dark" ? "Heller Modus" : "Dunkler Modus"}">${icon(this.state.theme === "dark" ? "sun" : "moon")}</button></footer>
        ${this.renderToast()}
        ${this.renderBusyLayer()}
      </main>`;
  }

  protected renderSidebar(): string {
    return `<aside class="sidebar">
        <div class="sidebar-brand"><div class="brand-mark"><span></span><span></span><span></span><span></span></div><div><strong>SUNO</strong><small>Documentation Manager</small></div></div>
        <nav class="main-nav" aria-label="Hauptnavigation">
          ${MAIN_NAVIGATION.map((item) => `<button class="nav-item ${this.state.view === item.id ? "is-active" : ""}" data-view="${item.id}" ${item.id === "current" && !this.state.track ? "disabled" : ""}>${icon(item.iconName)}<span>${item.label}</span>${item.id === "tracks" ? `<b>${this.state.tracks.length}</b>` : ""}</button>`).join("")}
        </nav>
        <div class="sidebar-bottom">
          <div class="offline-card">${icon("shield")}<div><strong>Lokaler Modus</strong><span>Keine Daten verlassen dieses Gerät</span></div></div>
          <div class="workspace-mini"><span class="workspace-avatar">${escapeHtml(titleInitials(this.state.workspace?.name ?? "WS"))}</span><div><strong>${escapeHtml(this.state.workspace?.name)}</strong><span title="${escapeHtml(this.state.workspace?.path)}">${escapeHtml(this.state.workspace?.path)}</span></div><button class="icon-button" data-view="workspace" aria-label="Workspace öffnen">${icon("arrow")}</button></div>
        </div>
      </aside>`;
  }

  protected renderTopbar(): string {
    const titles: Record<MainView, [string, string]> = {
      dashboard: ["Dashboard", "Dokumentationsstatus auf einen Blick"],
      tracks: ["Tracks", "Alle lokalen Musikprojekte"],
      current: [this.state.track?.title ?? "Aktueller Track", "Geführter Dokumentationsworkflow"],
      workspace: ["Workspace", "Lokaler Projektordner und Import"],
      settings: ["Einstellungen", "Globale Stammdaten und Richtlinien"]
    };
    const [title, subtitle] = titles[this.state.view];
    return `<header class="topbar">
        <button class="icon-button mobile-menu" data-action="open-sidebar" aria-label="Navigation öffnen">${icon("menu")}</button>
        <div class="topbar-title"><h1>${escapeHtml(title)}</h1><p>${escapeHtml(subtitle)}</p></div>
        <div class="topbar-actions">
          <button class="theme-toggle icon-button" data-action="toggle-theme" aria-label="${this.state.theme === "dark" ? "Hellen Modus aktivieren" : "Dunklen Modus aktivieren"}" aria-pressed="${this.state.theme === "dark"}" title="${this.state.theme === "dark" ? "Heller Modus" : "Dunkler Modus"}">${icon(this.state.theme === "dark" ? "sun" : "moon")}</button>
          ${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : '<span class="offline-pill"><i></i> Offline</span>'}
          <button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button>
        </div>
      </header>`;
  }

  protected renderCurrentView(): string {
    switch (this.state.view) {
      case "dashboard":
        return this.renderDashboard();
      case "tracks":
        return this.renderTracks();
      case "current":
        return this.renderTrack();
      case "workspace":
        return this.renderWorkspace();
      case "settings":
        return this.renderSettings();
    }
  }

  protected renderToast(): string {
    const toast = this.state.toast;
    if (!toast) return "";
    return `<div class="toast toast--${toast.kind}" role="alert">${icon(toast.kind === "success" ? "check" : toast.kind === "error" ? "alert" : "info")}<div><strong>${escapeHtml(translateUiText(toast.title, this.language))}</strong><span>${escapeHtml(translateUiText(toast.message, this.language))}</span></div><button data-action="dismiss-toast" aria-label="Hinweis schließen">${icon("close")}</button></div>`;
  }

  protected renderNewTrackDialog(): string {
    const proposal = this.state.folderImport;
    const importedTrack = proposal?.tracks[0];
    const library: TrackLibraryAssignment =
      proposal?.kind === "album" ? { section: "album", albumTitle: proposal.albumTitle } : { section: "single" };
    const submitLabel = proposal
      ? `${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} importieren`
      : "Track anlegen";
    const creationFields =
      proposal?.kind === "album"
        ? `<div class="read-only-field"><span>Ziel der Bibliothek</span><strong>${escapeHtml(proposal.albumTitle ?? "Unbenanntes Album")}</strong><small>Alle erkannten Tracks erhalten die normale Album-/Track-Struktur. Der Produktionsstart bleibt je Track offen.</small></div>`
        : `${this.textField("title", "Track-Titel", "z. B. Cosmic Pulse", importedTrack?.title ?? "", true)}
          ${this.dateField("productionStartDate", "Produktionsstart", proposal ? "" : new Date().toISOString().slice(0, 10), !proposal)}
          ${this.renderTrackLibraryFields(library, "new-track")}`;
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal track-library-modal" role="dialog" aria-modal="true" aria-labelledby="new-track-title" data-modal-panel>
        <div class="modal-head"><div><p class="overline">Neues Projekt</p><h2 id="new-track-title">Track anlegen</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
        <form id="new-track-form" class="form-stack">
          ${proposal ? this.renderFolderImportPreview(proposal) : ""}
          ${creationFields}
          <label class="toggle-row"><span><strong>Kommerzielle Nutzung vorgesehen</strong><small>Wird als Track-Snapshot gespeichert.</small></span><input type="checkbox" name="commercialUseIntended" ${this.state.profile.defaultCommercialUse ? "checked" : ""}><i></i></label>
          <div class="modal-actions"><button type="button" class="button button--secondary modal-import-button" data-action="scan-folder-import">${icon("upload")} Ordner importieren</button><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("plus")} ${submitLabel}</button></div>
        </form>
      </section></div>`;
  }

  protected renderFolderImportPreview(proposal: FolderImportProposal): string {
    const heading =
      proposal.kind === "album"
        ? `Album: ${escapeHtml(proposal.albumTitle ?? "Unbenannt")}`
        : "Einzelner Track erkannt";
    return `<section class="folder-import-preview"><p class="overline">Import erkannt</p><strong>${heading}</strong><small>${proposal.tracks.length} ${proposal.tracks.length === 1 ? "Track" : "Tracks"} erkannt · Produktionsstart bleibt offen, sofern er nicht dokumentiert ist.</small><div class="folder-import-tracks">${proposal.tracks
      .map((track) => {
        const recognised = track.files
          .filter((file) => file.selected)
          .map((file) => `${file.fileName} (${file.roles.join(", ")})`);
        return `<div><strong>${escapeHtml(track.title)}</strong>${recognised.length ? `<small>${escapeHtml(recognised.join(" · "))}</small>` : "<small>Keine eindeutig zuordenbare Evidence erkannt.</small>"}${track.ambiguities.length ? `<small class="warning">⚠ ${escapeHtml(track.ambiguities.map((message) => this.systemText(message)).join(" · "))} – Auswahl bleibt offen</small>` : ""}${track.unassignedFiles.length ? `<small>Nicht zugeordnet: ${escapeHtml(track.unassignedFiles.join(", "))}</small>` : ""}</div>`;
      })
      .join(
        ""
      )}</div>${proposal.unassignedFiles.length ? `<small>Nicht zugeordnet: ${escapeHtml(proposal.unassignedFiles.join(", "))}</small>` : ""}</section>`;
  }

  protected renderTrackLibraryDialog(): string {
    const track = this.state.track;
    if (!track) return "";
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal track-library-modal" role="dialog" aria-modal="true" aria-labelledby="track-library-title" data-modal-panel>
        <div class="modal-head"><div><p class="overline">Bibliothekszuordnung</p><h2 id="track-library-title">${escapeHtml(track.title)} einordnen</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
        <form id="track-library-form" class="form-stack">
          ${this.renderTrackLibraryFields(track.library, "track-library")}
          <div class="library-safety-note">${icon("shield")}<p>Beim Speichern wird der vollständige Track-Ordner sicher in den gewählten Album- oder Singles-Ordner verschoben. Dateien, interne Prüfsummen und Zertifikat bleiben dabei unverändert.</p></div>
          <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("check")} Zuordnung speichern</button></div>
        </form>
      </section></div>`;
  }

  protected renderTrackLibraryFields(library: TrackLibraryAssignment, idPrefix: string): string {
    const albumSelected = library.section === "album";
    const albumFieldId = `${idPrefix}-album-field`;
    const albumListId = `${idPrefix}-album-titles`;
    const albumTitles = this.albumTitles();
    return `<fieldset class="track-library-field"><legend>Bereich der Track-Bibliothek *</legend><div class="track-library-choices">
        <label><input type="radio" name="librarySection" value="single" aria-controls="${albumFieldId}" ${albumSelected ? "" : "checked"} required><span>${icon("tracks")}<strong>Single</strong><small>Unter Singles einordnen</small></span></label>
        <label><input type="radio" name="librarySection" value="album" aria-controls="${albumFieldId}" ${albumSelected ? "checked" : ""} required><span>${icon("workspace")}<strong>Album-Track</strong><small>Einem Album zuordnen</small></span></label>
      </div></fieldset>
      <label class="field library-album-field" id="${albumFieldId}" data-library-album-field ${albumSelected ? "" : "hidden"}><span class="field-label">Albumtitel *</span><input type="text" name="albumTitle" list="${albumListId}" placeholder="Bestehendes oder neues Album" value="${escapeHtml(library.albumTitle ?? "")}" autocomplete="off" ${albumSelected ? "required" : "disabled"}><small>Wird als echter Ordnername verwendet; maximal 200 Zeichen, keine Pfadtrenner oder reservierten Namen.</small></label>
      <datalist id="${albumListId}">${albumTitles.map((title) => `<option value="${escapeHtml(title)}"></option>`).join("")}</datalist>`;
  }

  protected albumTitles(): string[] {
    const titles = new Map<string, string>();
    for (const title of this.state.albums) {
      const normalized = title.trim();
      if (normalized) titles.set(normalized.normalize("NFKC").toLocaleLowerCase("de-DE"), normalized);
    }
    for (const track of this.state.tracks) {
      const title = track.library?.section === "album" ? track.library.albumTitle?.trim() : "";
      if (title) titles.set(title.normalize("NFKC").toLocaleLowerCase("de-DE"), title);
    }
    return [...titles.values()].sort((left, right) =>
      left.localeCompare(right, "de", { sensitivity: "base", numeric: true })
    );
  }

  protected renderSubscriptionEvidenceDialog(): string {
    const coverageStart = new Date().toISOString().slice(0, 8) + "01";
    const coverageEnd = subscriptionCoverageEnd(coverageStart, "monthly") ?? "";
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal subscription-evidence-modal" role="dialog" aria-modal="true" aria-labelledby="subscription-evidence-title" data-modal-panel>
        <div class="modal-head"><div><p class="overline">Wiederverwendbarer Nachweis</p><h2 id="subscription-evidence-title">Suno-Abo-Nachweis registrieren</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
        <form id="subscription-evidence-form" class="form-stack">
          <fieldset class="billing-cycle-field"><legend>Bezahlrhythmus *</legend><div>
            <label><input type="radio" name="billingCycle" value="monthly" checked><span><strong>Monatlich</strong><small>Ein Kalendermonat ab dem Startdatum</small></span></label>
            <label><input type="radio" name="billingCycle" value="annual"><span><strong>Jährlich</strong><small>Zwölf Kalendermonate ab dem Startdatum</small></span></label>
          </div></fieldset>
          <div class="field-grid two-col">
            ${this.dateField("coverageStart", "Beginn laut Rechnung", coverageStart, true)}
            <label class="field"><span class="field-label">Automatisch abgedeckt bis</span><input type="date" name="coverageEnd" value="${coverageEnd}" readonly aria-readonly="true"></label>
          </div>
          <div class="evidence-guidance">${icon("info")}<p>Übernimm den tatsächlichen Beginn vom Beleg. Das Enddatum wird bis zum Tag vor der nächsten Zahlung berechnet; der Inhalt der Datei wird nicht automatisch ausgelesen. Pro Registrierung wird genau eine Rechnung oder ein Beleg ausgewählt.</p></div>
          <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("upload")} Datei auswählen und registrieren</button></div>
        </form>
      </section></div>`;
  }

  protected renderTermsMetadataDialog(): string {
    const dialog = this.state.termsMetadataDialog;
    if (!dialog) return "";
    const metadata = dialog.metadata;
    const editing = Boolean(dialog.evidenceId);
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal evidence-metadata-modal" role="dialog" aria-modal="true" aria-labelledby="terms-metadata-title" data-modal-panel>
        <div class="modal-head"><div><p class="overline">Terms-/Rights-Evidence</p><h2 id="terms-metadata-title">${editing ? "Beschreibende Metadaten bearbeiten" : "Nutzungsbedingungen registrieren"}</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
        <form id="terms-metadata-form" class="form-stack">
          <div class="evidence-guidance">${icon("info")}<p>Dokumentiere nur bekannte Fakten zur lokal archivierten Fassung. SunoDM ruft keine Internetdaten ab und bewertet weder Rechte noch rechtliche Wirksamkeit.</p></div>
          <div class="field-grid two-col">
            ${this.textField("documentTitle", "Dokumenttitel", "z. B. Suno Terms of Service", metadata.documentTitle, true)}
            ${this.textField("provider", "Provider / Source", "Vom Benutzer dokumentierter Anbieter", metadata.provider, true)}
            ${this.dateField("retrievalDate", "Abrufdatum", metadata.retrievalDate, true)}
            ${this.textField("sourceUrl", "Source URL", "Optional", metadata.sourceUrl, false, "url")}
            ${this.dateField("effectiveDate", "Effective Date", metadata.effectiveDate)}
            ${this.textField("applicableProductionPeriod", "Anwendbarer Produktionszeitraum", "Optional; nur dokumentieren", metadata.applicableProductionPeriod)}
          </div>
          ${this.textArea("factualNote", "Sachlicher Hinweis", "Optionaler Kontext ohne rechtliche Schlussfolgerung", metadata.factualNote)}
          <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon(editing ? "check" : "upload")} ${editing ? "Metadaten speichern" : "PDF auswählen und registrieren"}</button></div>
        </form>
      </section></div>`;
  }

  protected renderEvidencePreviewDialog(): string {
    const preview = this.state.evidencePreview;
    if (!preview) return "";
    const metadata = this.state.track?.evidence.find((item) => item.id === preview.evidenceId)?.metadata;
    const technicalMetadata = metadata
      ? [
          metadata.mimeType ? `<div><dt>Medientyp</dt><dd>${escapeHtml(metadata.mimeType)}</dd></div>` : "",
          metadata.audioFormat ? `<div><dt>Audioformat</dt><dd>${escapeHtml(metadata.audioFormat)}</dd></div>` : "",
          typeof metadata.audioChannels === "number"
            ? `<div><dt>Kanäle</dt><dd>${metadata.audioChannels}</dd></div>`
            : "",
          typeof metadata.audioSampleRateHz === "number"
            ? `<div><dt>Sample Rate</dt><dd>${metadata.audioSampleRateHz.toLocaleString(this.numberLocale)} Hz</dd></div>`
            : "",
          typeof metadata.audioDurationMilliseconds === "number"
            ? `<div><dt>Dauer</dt><dd>${(metadata.audioDurationMilliseconds / 1000).toLocaleString(this.numberLocale, { maximumFractionDigits: 3 })} s</dd></div>`
            : "",
          typeof metadata.audioBitDepth === "number"
            ? `<div><dt>Bit-Tiefe</dt><dd>${metadata.audioBitDepth} Bit</dd></div>`
            : "",
          metadata.sunoStudioDetected
            ? `<div><dt>Suno Studio</dt><dd>Erkannt · Evidence-derived metadata</dd></div>`
            : "",
          metadata.sunoCreatedTimestamp
            ? `<div><dt>Suno-created</dt><dd>${escapeHtml(metadata.sunoCreatedTimestamp)}</dd></div>`
            : "",
          metadata.sunoId ? `<div><dt>Technische Suno-ID</dt><dd>${escapeHtml(metadata.sunoId)}</dd></div>` : ""
        ]
          .filter(Boolean)
          .join("")
      : "";
    const content = preview.dataUrl
      ? `<div class="evidence-preview-stage"><img src="${escapeHtml(preview.dataUrl)}" alt="Vorschau von ${escapeHtml(preview.fileName)}"></div>`
      : preview.textContent !== undefined && preview.textContent !== null
        ? `<pre class="evidence-preview-text">${escapeHtml(preview.textContent)}</pre>`
        : `<div class="evidence-preview-unavailable">${icon("file")}<p>${escapeHtml(this.systemText(preview.message ?? "Für diese Datei ist keine Vorschau verfügbar."))}</p></div>`;
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal evidence-preview-modal" role="dialog" aria-modal="true" aria-labelledby="evidence-preview-title" data-modal-panel>
        <div class="modal-head"><div><p class="overline">Evidence-Vorschau</p><h2 id="evidence-preview-title">${escapeHtml(preview.fileName)}</h2></div><button class="icon-button" data-action="close-modal" aria-label="Vorschau schließen">${icon("close")}</button></div>
        ${content}
        <dl class="evidence-preview-meta"><div><dt>Rolle</dt><dd>${escapeHtml(evidenceRoleLabel(preview.role))}</dd></div><div><dt>Größe</dt><dd>${escapeHtml(formatBytes(preview.sizeBytes, this.language))}</dd></div><div><dt>Pfad</dt><dd>${escapeHtml(preview.relativePath)}</dd></div>${technicalMetadata}</dl>
        <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Schließen</button></div>
      </section></div>`;
  }

  protected renderCertificatePopupDialog(): string {
    const track = this.state.track;
    if (!track?.certificate.valid || !track.certificate.certificateId) return "";
    const openBlockingDeviations = (track.blockingDeviations ?? []).filter(
      (item) => item.blocking && !item.resolved
    ).length;
    return `<div class="modal-backdrop certificate-popup-backdrop" data-action="close-modal"><section class="modal certificate-popup-modal" role="dialog" aria-modal="true" aria-labelledby="certificate-popup-title" data-modal-panel>
        <button class="icon-button certificate-popup-close" data-action="close-modal" aria-label="Zertifikat schließen">${icon("close")}</button>
        <div class="certificate-popup-celebration" aria-hidden="true"><i></i><i></i><i></i><span>${icon("certificate")}</span></div>
        <p class="overline">Track Documentation Completion Certificate</p>
        <h2 id="certificate-popup-title">Dokumentation erfolgreich finalisiert</h2>
        <span class="certificate-popup-result">${icon("check")} DOCUMENTATION COMPLETE</span>
        <p class="certificate-popup-meaning"><strong>${this.t("Meaning:")}</strong> ${this.t("Configured documentation requirements completed.")}</p>
        <dl class="certificate-popup-facts">
          <div><dt>Certificate ID</dt><dd>${escapeHtml(track.certificate.certificateId)}</dd></div>
          <div><dt>Track</dt><dd>${escapeHtml(track.title)}</dd></div>
          <div><dt>Artist</dt><dd>${escapeHtml(track.profileSnapshot.artistName)}</dd></div>
          <div><dt>Finalisiert</dt><dd>${formatDate(track.certificate.finalizedAt, true, this.language)}</dd></div>
          <div><dt>Workflow</dt><dd>${escapeHtml(track.workflowId)} · ${escapeHtml(track.certificate.workflowVersion ?? track.workflowVersion)}</dd></div>
          <div><dt>Integrität</dt><dd>${track.integrity.verifiedCount} / ${track.integrity.fileCount} Dateien verifiziert</dd></div>
          <div><dt>Evidence</dt><dd>${track.evidence.length} Dateien</dd></div>
          <div><dt>Blockierende Abweichungen</dt><dd>${openBlockingDeviations}</dd></div>
        </dl>
        <p class="certificate-popup-note">Der lokale Zertifikatssatz wurde erzeugt und verifiziert. PASS bedeutet ausschließlich: Configured documentation requirements for this step were satisfied. Dies ist keine behördliche oder rechtliche Zertifizierung.</p>
        <div class="modal-actions certificate-popup-actions"><button type="button" class="button button--secondary" data-action="close-modal">Schließen</button><button type="button" class="button button--primary" data-action="open-certificate-tab">${icon("certificate")} Vollständiges Zertifikat öffnen</button></div>
      </section></div>`;
  }

  protected renderTrackCover(
    track: Pick<TrackSummary, "id" | "title" | "coverEvidenceId">,
    modifier = "",
    element: "span" | "div" = "span"
  ): string {
    const cached = this.trackCoverCache.get(track.id);
    const dataUrl = track.coverEvidenceId && cached?.evidenceId === track.coverEvidenceId ? cached.dataUrl : undefined;
    const classes = `track-cover${modifier ? ` ${modifier}` : ""}${dataUrl ? " has-artwork" : ""}`;
    return `<${element} class="${classes}" data-track-cover="${escapeHtml(track.id)}">
        <img class="track-cover__image" ${dataUrl ? `src="${escapeHtml(dataUrl)}"` : ""} alt="" ${dataUrl ? "" : "hidden"}>
        <span class="track-cover__fallback" ${dataUrl ? "hidden" : ""}>${escapeHtml(titleInitials(track.title))}<i></i></span>
      </${element}>`;
  }
}
