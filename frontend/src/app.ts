import type { DesktopApi } from "./api/desktop";
import { toUserMessage } from "./api/desktop";
import {
  calculateMissingRequirements,
  evaluateRequirements,
  evidenceRoleLabel,
  finalizationGate,
  statusLabel,
  stepStatuses,
  visibleConditionalFields,
  WORKFLOW_STEPS
} from "./domain/workflow";
import {
  emptyProfile,
  type GlobalEvidenceItem,
  type EvidenceRole,
  type GlobalProfile,
  type ScanResult,
  type StepId,
  type TrackDetail,
  type TrackFields,
  type TrackSummary,
  type WorkflowDefinitionDto,
  type WorkspaceSummary
} from "./domain/types";
import { escapeHtml, formatBytes, formatDate, titleInitials } from "./ui/format";
import { icon } from "./ui/icons";

type MainView = "dashboard" | "tracks" | "current" | "workspace" | "settings";
type TrackTab = "overview" | "suno" | "artwork" | "release" | "evidence" | "certificate";
type ToastKind = "success" | "error" | "info";

interface ToastState {
  kind: ToastKind;
  title: string;
  message: string;
}

interface AppState {
  workspace: WorkspaceSummary | null;
  profile: GlobalProfile;
  tracks: TrackSummary[];
  track: TrackDetail | null;
  workflow: WorkflowDefinitionDto | null;
  globalEvidence: GlobalEvidenceItem[];
  view: MainView;
  trackTab: TrackTab;
  activeStep: StepId | null;
  trackDraft: TrackFields | null;
  scanResult: ScanResult | null;
  query: string;
  trackFilter: "all" | "open" | "ready" | "finalized";
  busy: boolean;
  busyLabel: string;
  sidebarOpen: boolean;
  showNewTrack: boolean;
  toast: ToastState | null;
}

export const MAIN_NAVIGATION: ReadonlyArray<{ id: MainView; label: string; iconName: "dashboard" | "tracks" | "current" | "workspace" | "settings" }> = [
  { id: "dashboard", label: "Dashboard", iconName: "dashboard" },
  { id: "tracks", label: "Tracks", iconName: "tracks" },
  { id: "current", label: "Aktueller Track", iconName: "current" },
  { id: "workspace", label: "Workspace", iconName: "workspace" },
  { id: "settings", label: "Einstellungen", iconName: "settings" }
];

export function missingProfileFields(profile: GlobalProfile): string[] {
  const fields: Array<[keyof GlobalProfile, string]> = [
    ["artistName", "Künstlername"], ["sunoProfileName", "Suno-Profilname"],
    ["sunoHandle", "Suno-Benutzername"], ["sunoPlan", "Suno-Tarif"],
    ["subscriptionStartDate", "Abo-Startdatum"], ["defaultAiImageService", "Standard-KI-Bilddienst"],
    ["artworkTransparencyPolicy", "Artwork-Transparenzrichtlinie"], ["disclosureText", "Standard-Hinweistext"]
  ];
  return fields.filter(([key]) => typeof profile[key] !== "string" || String(profile[key]).trim() === "").map(([, label]) => label);
}

const evidenceRoles: EvidenceRole[] = [
  "suno_final_export", "suno_project_zip", "suno_screenshot",
  "release_wav", "release_mp3", "release_mp4", "release_artwork", "ai_artwork_original",
  "ai_artwork_edited", "human_edited_artwork", "final_artwork", "external_audio_file",
  "external_audio_license", "own_audio_file", "third_party_sample_file",
  "third_party_sample_license", "other"
];

const evidenceProvenanceLabel = (value: TrackDetail["evidence"][number]["provenance"]): string => ({
  managed_copy: "Verwaltete Kopie",
  global_copy: "Globale portable Kopie",
  generated_disclosure: "Lokal erzeugter Disclosure-Nachweis",
  indexed_legacy: "Historisch indexiert"
}[value ?? "managed_copy"]);

export class SunoDocumentationApp {
  private readonly state: AppState = {
    workspace: null,
    profile: { ...emptyProfile },
    tracks: [],
    track: null,
    workflow: null,
    globalEvidence: [],
    view: "dashboard",
    trackTab: "overview",
    activeStep: null,
    trackDraft: null,
    scanResult: null,
    query: "",
    trackFilter: "all",
    busy: false,
    busyLabel: "",
    sidebarOpen: false,
    showNewTrack: false,
    toast: null
  };

  private toastTimer: number | undefined;
  private draftDirty = false;

  constructor(
    private readonly root: HTMLElement,
    private readonly api: DesktopApi
  ) {}

  start(): void {
    this.root.addEventListener("click", (event) => void this.handleClick(event));
    this.root.addEventListener("submit", (event) => void this.handleSubmit(event));
    this.root.addEventListener("change", (event) => this.handleChange(event));
    this.root.addEventListener("input", (event) => this.handleInput(event));
    this.render();
  }

  private async withBusy<T>(label: string, action: () => Promise<T>): Promise<T | undefined> {
    this.state.busy = true;
    this.state.busyLabel = label;
    this.render();
    try {
      return await action();
    } catch (error) {
      this.showToast("error", "Aktion nicht abgeschlossen", toUserMessage(error));
      return undefined;
    } finally {
      this.state.busy = false;
      this.state.busyLabel = "";
      this.render();
    }
  }

  private showToast(kind: ToastKind, title: string, message: string): void {
    this.state.toast = { kind, title, message };
    window.clearTimeout(this.toastTimer);
    this.toastTimer = window.setTimeout(() => {
      this.state.toast = null;
      this.render();
    }, kind === "error" ? 8000 : 4500);
  }

  private async enterWorkspace(workspace: WorkspaceSummary): Promise<void> {
    this.state.workspace = workspace;
    const loaded = await this.withBusy("Workspace wird eingelesen …", async () => {
      const [profile, tracks, workflow, globalEvidence] = await Promise.all([
        this.api.getProfile(), this.api.listTracks(), this.api.getWorkflow(), this.api.listGlobalEvidence()
      ]);
      return { profile, tracks, workflow, globalEvidence };
    });
    if (!loaded) {
      this.state.workspace = null;
      return;
    }
    this.state.profile = loaded.profile;
    this.state.tracks = loaded.tracks;
    this.state.workflow = loaded.workflow;
    this.state.globalEvidence = loaded.globalEvidence;
    this.state.view = "dashboard";
  }

  private async refreshTracks(): Promise<void> {
    this.state.tracks = await this.api.listTracks();
    if (this.state.track) {
      this.state.track = await this.api.loadTrack(this.state.track.id);
      this.state.trackDraft = structuredClone(this.state.track.fields);
    }
  }

  private applyTrack(track: TrackDetail): void {
    this.state.track = track;
    this.state.trackDraft = structuredClone(track.fields);
    this.draftDirty = false;
    const summaryIndex = this.state.tracks.findIndex((item) => item.id === track.id);
    const summary: TrackSummary = {
      id: track.id,
      title: track.title,
      relativePath: track.relativePath,
      status: track.status,
      updatedAt: track.updatedAt,
      progress: track.progress,
      missingCount: track.missingCount,
      certificateValid: track.certificate.valid,
      legacy: track.legacy
    };
    if (summaryIndex >= 0) this.state.tracks[summaryIndex] = summary;
    else this.state.tracks.unshift(summary);
  }

  private render(): void {
    if (!this.state.workspace) {
      this.root.innerHTML = this.renderWelcome();
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
        ${this.state.showNewTrack ? this.renderNewTrackDialog() : ""}
        ${this.renderToast()}
        ${this.state.busy ? `<div class="busy-layer" role="status" aria-live="polite"><span class="spinner"></span><span>${escapeHtml(this.state.busyLabel)}</span></div>` : ""}
      </div>`;
  }

  private renderWelcome(): string {
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
      <footer class="welcome-footer"><span>Version 0.1</span><span>•</span><span>Offline by design</span>${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : ""}</footer>
      ${this.renderToast()}
      ${this.state.busy ? `<div class="busy-layer" role="status"><span class="spinner"></span><span>${escapeHtml(this.state.busyLabel)}</span></div>` : ""}
    </main>`;
  }

  private renderSidebar(): string {
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

  private renderTopbar(): string {
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
        ${this.api.mode === "demo" ? '<span class="demo-badge">Browser-Demo</span>' : '<span class="offline-pill"><i></i> Offline</span>'}
        <button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button>
      </div>
    </header>`;
  }

  private renderCurrentView(): string {
    switch (this.state.view) {
      case "dashboard": return this.renderDashboard();
      case "tracks": return this.renderTracks();
      case "current": return this.renderTrack();
      case "workspace": return this.renderWorkspace();
      case "settings": return this.renderSettings();
    }
  }

  private renderToast(): string {
    const toast = this.state.toast;
    if (!toast) return "";
    return `<div class="toast toast--${toast.kind}" role="alert">${icon(toast.kind === "success" ? "check" : toast.kind === "error" ? "alert" : "info")}<div><strong>${escapeHtml(toast.title)}</strong><span>${escapeHtml(toast.message)}</span></div><button data-action="dismiss-toast" aria-label="Hinweis schließen">${icon("close")}</button></div>`;
  }

  private renderNewTrackDialog(): string {
    return `<div class="modal-backdrop" data-action="close-modal"><section class="modal" role="dialog" aria-modal="true" aria-labelledby="new-track-title" data-modal-panel>
      <div class="modal-head"><div><p class="overline">Neues Projekt</p><h2 id="new-track-title">Track anlegen</h2></div><button class="icon-button" data-action="close-modal" aria-label="Dialog schließen">${icon("close")}</button></div>
      <form id="new-track-form" class="form-stack">
        ${this.textField("title", "Track-Titel", "z. B. Cosmic Pulse", "", true)}
        ${this.dateField("productionStartDate", "Produktionsstart", new Date().toISOString().slice(0, 10), true)}
        <label class="toggle-row"><span><strong>Kommerzielle Nutzung vorgesehen</strong><small>Wird als Track-Snapshot gespeichert.</small></span><input type="checkbox" name="commercialUseIntended" ${this.state.profile.defaultCommercialUse ? "checked" : ""}><i></i></label>
        <div class="modal-actions"><button type="button" class="button button--secondary" data-action="close-modal">Abbrechen</button><button class="button button--primary" type="submit">${icon("plus")} Track anlegen</button></div>
      </form>
    </section></div>`;
  }

  private renderDashboard(): string {
    const active = this.state.tracks.filter((track) => track.status === "ACTIVE" || track.status === "DRAFT").length;
    const ready = this.state.tracks.filter((track) => track.status === "READY").length;
    const finalized = this.state.tracks.filter((track) => track.status === "FINALIZED" && track.certificateValid !== false).length;
    const recent = [...this.state.tracks].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 4);
    const next = recent.find((track) => track.status !== "FINALIZED" || track.certificateValid === false) ?? recent[0];
    return `<div class="page-content dashboard-page">
      <section class="dashboard-welcome">
        <div><p class="overline">Lokaler Workspace</p><h2>Guten Tag, ${escapeHtml(this.state.profile.artistName || "Artist")}.</h2><p>${next ? `Bei <strong>${escapeHtml(next.title)}</strong> sind noch ${next.missingCount} Pflichtpunkte offen.` : "Lege deinen ersten Track an, um den Workflow zu starten."}</p></div>
        <div class="workspace-seal">${icon("shield")}<span>Workspace geschützt</span></div>
      </section>
      <section class="metric-grid" aria-label="Track-Status">
        ${this.metricCard("tracks", "Tracks gesamt", this.state.tracks.length, "Im aktuellen Workspace", "ink")}
        ${this.metricCard("current", "In Bearbeitung", active, "Dokumentation offen", "amber")}
        ${this.metricCard("check", "Bereit", ready, "Bereit zur Finalisierung", "blue")}
        ${this.metricCard("certificate", "Finalisiert", finalized, "Mit gültigem Snapshot", "green")}
      </section>
      <div class="dashboard-columns">
        <section class="panel recent-panel">
          <div class="panel-heading"><div><p class="overline">Zuletzt bearbeitet</p><h3>Deine Tracks</h3></div><button class="text-button" data-view="tracks">Alle anzeigen ${icon("arrow")}</button></div>
          ${recent.length ? `<div class="track-list compact">${recent.map((track) => this.renderTrackRow(track)).join("")}</div>` : this.emptyState("tracks", "Noch keine Tracks", "Lege deinen ersten Track an und dokumentiere nur, was wirklich relevant ist.", "new-track", "Track anlegen")}
        </section>
        <aside class="panel attention-panel">
          <div class="panel-heading"><div><p class="overline">Nächster Schritt</p><h3>Aufmerksamkeit</h3></div><span class="attention-count">${active}</span></div>
          ${next ? `<div class="attention-track"><div class="track-cover track-cover--large">${escapeHtml(titleInitials(next.title))}<span></span></div><div><span class="status-chip status-chip--${next.status.toLowerCase()}">${statusLabel(next.status)}</span><h4>${escapeHtml(next.title)}</h4><p>${next.missingCount > 0 ? `${next.missingCount} erforderliche Angaben oder Nachweise fehlen noch.` : "Alle Pflichtpunkte sind erfüllt."}</p></div></div>
          <div class="progress-block"><div><span>Dokumentationsfortschritt</span><strong>${next.progress}%</strong></div><progress class="progress-track" max="100" value="${next.progress}" aria-label="Dokumentationsfortschritt ${next.progress} Prozent"></progress></div>
          <button class="button button--dark button--wide" data-track-open="${escapeHtml(next.id)}">Dokumentation fortsetzen ${icon("arrow")}</button>` : `<p class="muted">Keine offenen Tracks.</p>`}
        </aside>
      </div>
      <section class="principle-strip"><div class="principle-icon">${icon("shield")}</div><div><strong>Show what is missing.</strong><span>Ask only what is necessary.</span></div><p>Die App speichert ausschließlich lokal und erzeugt portable Track-Ordner, die ohne diese App verständlich bleiben.</p></section>
    </div>`;
  }

  private metricCard(iconName: "tracks" | "current" | "check" | "certificate", label: string, value: number, note: string, color: string): string {
    return `<article class="metric-card"><span class="metric-icon metric-icon--${color}">${icon(iconName)}</span><div><span>${label}</span><strong>${value.toLocaleString("de-DE")}</strong><small>${note}</small></div></article>`;
  }

  private renderTracks(): string {
    const query = this.state.query.trim().toLocaleLowerCase("de");
    const filtered = this.state.tracks.filter((track) => {
      const matchesQuery = !query || track.title.toLocaleLowerCase("de").includes(query) || track.relativePath.toLocaleLowerCase("de").includes(query);
      const matchesFilter = this.state.trackFilter === "all"
        || (this.state.trackFilter === "open" && ["DRAFT", "ACTIVE"].includes(track.status))
        || (this.state.trackFilter === "ready" && track.status === "READY")
        || (this.state.trackFilter === "finalized" && track.status === "FINALIZED");
      return matchesQuery && matchesFilter;
    });
    return `<div class="page-content tracks-page">
      <div class="page-lead"><div><p class="overline">Bibliothek</p><h2>Alle Tracks</h2><p>Öffne einen Track und sieh sofort, was dokumentiert ist und was noch fehlt.</p></div><button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button></div>
      <section class="panel tracks-panel">
        <div class="tracks-toolbar">
          <label class="search-field"><span class="sr-only">Tracks durchsuchen</span>${icon("scan")}<input type="search" data-track-search placeholder="Tracks durchsuchen …" value="${escapeHtml(this.state.query)}"></label>
          <div class="filter-tabs" role="group" aria-label="Statusfilter">
            ${([['all','Alle'],['open','Offen'],['ready','Bereit'],['finalized','Finalisiert']] as const).map(([id, label]) => `<button class="${this.state.trackFilter === id ? "is-active" : ""}" data-track-filter="${id}">${label}</button>`).join("")}
          </div>
          <button class="button button--secondary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button>
        </div>
        ${filtered.length ? `<div class="track-table-head"><span>Track</span><span>Status</span><span>Fortschritt</span><span>Aktualisiert</span><span></span></div><div class="track-list">${filtered.map((track) => this.renderTrackRow(track, true)).join("")}</div>` : this.emptyState("tracks", "Keine passenden Tracks", "Passe Suche oder Statusfilter an.")}
      </section>
    </div>`;
  }

  private renderTrackRow(track: TrackSummary, detailed = false): string {
    return `<button class="track-row ${detailed ? "track-row--detailed" : ""}" data-track-open="${escapeHtml(track.id)}">
      <span class="track-cover">${escapeHtml(titleInitials(track.title))}<i></i></span>
      <span class="track-identity"><strong>${escapeHtml(track.title)}</strong><small>${escapeHtml(track.relativePath)}${track.legacy ? " · Legacy-Import" : ""}</small></span>
      <span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span>
      <span class="row-progress"><progress max="100" value="${track.progress}" aria-label="${track.progress} Prozent"></progress><b>${track.progress}%</b></span>
      ${detailed ? `<time>${formatDate(track.updatedAt)}</time>` : `<span class="missing-hint">${track.missingCount ? `${track.missingCount} offen` : "Vollständig"}</span>`}
      <span class="row-arrow">${icon("arrow")}</span>
    </button>`;
  }

  private renderTrack(): string {
    const track = this.state.track;
    if (!track) return this.emptyState("current", "Kein Track ausgewählt", "Wähle einen Track aus deiner Bibliothek.", "go-tracks", "Tracks öffnen");
    const tabs: Array<[TrackTab, string]> = [["overview", "Übersicht"], ["suno", "Suno"], ["artwork", "Artwork"], ["release", "Release"], ["evidence", "Evidence"], ["certificate", "Zertifikat"]];
    return `<div class="track-page">
      <section class="track-hero">
        <div class="track-cover track-cover--hero">${escapeHtml(titleInitials(track.title))}<i></i></div>
        <div class="track-hero-copy"><div><span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span><span class="workflow-version">Workflow ${escapeHtml(track.workflowVersion)}</span></div><h2>${escapeHtml(track.title)}</h2><p>${escapeHtml(track.relativePath)}</p></div>
        <div class="hero-progress"><strong>${track.progress}%</strong><span>dokumentiert</span><progress class="progress-track" max="100" value="${track.progress}" aria-label="Dokumentationsfortschritt ${track.progress} Prozent"></progress></div>
      </section>
      <nav class="track-tabs" aria-label="Track-Ansichten">${tabs.map(([id, label]) => `<button class="${this.state.trackTab === id ? "is-active" : ""}" data-track-tab="${id}">${label}</button>`).join("")}</nav>
      <div class="track-content">${this.renderTrackTab(track)}</div>
    </div>`;
  }

  private renderTrackTab(track: TrackDetail): string {
    if (this.state.activeStep) return this.renderWorkflowEditor(track, this.state.activeStep);
    switch (this.state.trackTab) {
      case "overview": return this.renderTrackOverview(track);
      case "suno": return this.renderWorkflowEditor(track, "suno");
      case "artwork": return this.renderWorkflowEditor(track, "artwork");
      case "release": return this.renderWorkflowEditor(track, "release");
      case "evidence": return this.renderEvidence(track);
      case "certificate": return this.renderCertificate(track);
    }
  }

  private renderTrackOverview(track: TrackDetail): string {
    const missing = calculateMissingRequirements(track, track.profileSnapshot);
    const evaluatedStatuses = stepStatuses(track, track.profileSnapshot);
    const statuses = this.runtimeSteps().map((step) => evaluatedStatuses.find((state) => state.id === step.id) ?? { id: step.id, status: "NOT_RUN" as const });
    return `<div class="workflow-layout">
      <section class="workflow-main">
        ${track.status === "FINALIZED" && !track.certificate.valid ? this.renderInvalidCertificateNotice() : ""}
        ${track.legacy && missingProfileFields(track.profileSnapshot).length ? `<div class="policy-card">${icon("info")}<div><p class="overline">Legacy-Track</p><h4>Historische Stammdaten ausdrücklich bestätigen</h4><p>Der Scan hat keine fehlenden Fakten erfunden. Übernimm die aktuellen Workspace-Stammdaten nur, wenn sie für diesen Track tatsächlich zutreffen; danach kannst du weitere Angaben prüfen und speichern.</p></div><button class="button button--secondary" data-action="adopt-legacy-profile">Stammdaten als Snapshot bestätigen</button></div>` : ""}
        <div class="panel missing-panel ${missing.length === 0 ? "is-complete" : ""}">
          <div class="panel-heading"><div><p class="overline">Finalisierungs-Gate</p><h3>${missing.length ? "Was fehlt noch?" : "Bereit zur Finalisierung"}</h3></div><span class="missing-total">${missing.length}</span></div>
          ${missing.length ? `<ul class="missing-list">${missing.slice(0, 8).map((item) => `<li><span>${icon("alert")}</span><div><strong>${escapeHtml(item.label)}</strong><small>${escapeHtml(this.runtimeSteps().find((step) => step.id === item.stepId)?.title ?? item.stepId)}</small></div><button data-step-open="${item.stepId}">${item.evidenceRole ? "Nachweis" : "Öffnen"} ${icon("arrow")}</button></li>`).join("")}</ul>${missing.length > 8 ? `<button class="text-button" data-track-tab="evidence">Alle ${missing.length} offenen Punkte anzeigen</button>` : ""}` : `<div class="success-message">${icon("check")}<div><strong>Alle lokalen Vorprüfungen sind erfüllt.</strong><span>Rust validiert den Track vor der Finalisierung nochmals vollständig.</span></div></div>`}
        </div>
        <div class="panel workflow-panel"><div class="panel-heading"><div><p class="overline">Geführter Ablauf</p><h3>10 Dokumentationsschritte</h3></div><span class="workflow-id">${escapeHtml(this.state.workflow?.id ?? track.workflowId)} · v${escapeHtml(this.state.workflow?.version ?? track.workflowVersion)}</span></div>
          <div class="step-list">${statuses.map((step, index) => this.renderStepRow(step.id, step.status, index)).join("")}</div>
        </div>
      </section>
      <aside class="workflow-side">
        <div class="panel quick-actions"><p class="overline">Schnellaktionen</p><h3>Dokumentsatz</h3>
          ${this.actionRow("file", "Dokumente", track.documents.current ? "Aktuell" : track.documents.generated ? "Veraltet" : "Nicht erzeugt", "generate-documents")}
          ${this.actionRow("hash", "SHA-256", track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Nicht erzeugt", "calculate-hashes")}
          ${this.actionRow("shield", "Verifikation", track.integrity.verified ? "Bestanden" : "Offen", "verify-hashes")}
        </div>
        <div class="panel snapshot-card"><p class="overline">Track-Snapshot</p><dl><div><dt>Evidence</dt><dd>${track.evidence.length}</dd></div><div><dt>Dokumente</dt><dd>${track.documents.files.length}</dd></div><div><dt>Verifiziert</dt><dd>${track.integrity.verifiedCount}/${track.integrity.fileCount}</dd></div><div><dt>Abweichungen</dt><dd>${track.blockingDeviations?.filter((item) => item.blocking && !item.resolved).length ?? track.integrity.mismatchFiles.length}</dd></div></dl></div>
      </aside>
    </div>`;
  }

  private renderStepRow(stepId: StepId, status: string, index: number): string {
    const nativeStep = this.state.workflow?.steps.find((item) => item.id === stepId);
    const fallback = WORKFLOW_STEPS.find((item) => item.id === stepId)!;
    const number = nativeStep?.number ?? fallback.number;
    const label = nativeStep?.title ?? nativeStep?.label ?? fallback.title;
    const description = nativeStep?.description ?? fallback.description;
    return `<button class="step-row" data-step-open="${stepId}"><span class="step-number">${escapeHtml(number)}</span><span class="step-state step-state--${status.toLowerCase().replace("_", "-")}">${status === "PASS" || status === "N_A" ? icon("check") : status === "FAIL" || status === "BLOCKED" ? icon("alert") : index + 1}</span><span class="step-copy"><strong>${escapeHtml(label)}</strong><small>${escapeHtml(description)}</small></span><span class="step-status">${statusLabel(status as Parameters<typeof statusLabel>[0])}</span>${icon("arrow")}</button>`;
  }

  private runtimeSteps(): Array<{ id: StepId; number: string; title: string; description: string }> {
    if (this.state.workflow) {
      return this.state.workflow.steps.map((step) => ({
        id: step.id,
        number: step.number,
        title: step.title ?? step.label,
        description: step.description
      }));
    }
    // Explicit browser-demo/test fallback. Packaged Tauri builds use get_workflow().
    return WORKFLOW_STEPS.map((step) => ({ id: step.id, number: step.number, title: step.title, description: step.description }));
  }

  private actionRow(iconName: "file" | "hash" | "shield", label: string, state: string, action: string): string {
    return `<button class="action-row" data-action="${action}"><span>${icon(iconName)}</span><div><strong>${label}</strong><small>${state}</small></div>${icon("arrow")}</button>`;
  }

  private renderInvalidCertificateNotice(): string {
    return `<div class="danger-banner">${icon("alert")}<div><strong>Dokumentation nach Finalisierung geändert.</strong><span>Das Zertifikat stimmt nicht mehr mit dem aktuellen Track-Zustand überein.</span></div><button class="button button--danger" data-action="create-revision">Neue Revision anlegen</button></div>`;
  }

  private renderWorkflowEditor(track: TrackDetail, stepId: StepId): string {
    const runtimeSteps = this.runtimeSteps();
    const index = runtimeSteps.findIndex((step) => step.id === stepId);
    const definition = this.state.workflow?.steps.find((step) => step.id === stepId);
    const fallback = runtimeSteps[index];
    const label = definition?.title ?? definition?.label ?? fallback.title;
    const description = definition?.description ?? fallback.description;
    const statuses = stepStatuses(track, track.profileSnapshot);
    const currentStatus = statuses.find((step) => step.id === stepId)?.status ?? "NOT_RUN";
    const naEligible = !evaluateRequirements(track, track.profileSnapshot).some((requirement) => requirement.stepId === stepId);
    return `<div class="editor-layout">
      <aside class="workflow-rail">
        <button class="rail-back" data-action="back-overview">${icon("arrow")} Track-Übersicht</button>
        <div class="rail-progress"><span>${index + 1} / 10</span><progress max="10" value="${index + 1}" aria-label="Schritt ${index + 1} von 10"></progress></div>
        <nav aria-label="Workflow-Schritte">${runtimeSteps.map((runtimeStep) => {
          const step = statuses.find((entry) => entry.id === runtimeStep.id) ?? { id: runtimeStep.id, status: "NOT_RUN" as const };
          const item = this.state.workflow?.steps.find((entry) => entry.id === step.id);
          const itemFallback = runtimeStep;
          return `<button class="rail-step ${step.id === stepId ? "is-active" : ""} ${step.status === "PASS" || step.status === "N_A" ? "is-complete" : ""}" data-step-open="${step.id}"><span>${step.status === "PASS" || step.status === "N_A" ? icon("check") : item?.number ?? itemFallback.number}</span><strong>${escapeHtml(item?.title ?? item?.label ?? itemFallback.title)}</strong></button>`;
        }).join("")}</nav>
      </aside>
      <section class="editor-main">
        <header class="editor-head"><div><p class="overline">Schritt ${escapeHtml(definition?.number ?? fallback.number)}</p><h3>${escapeHtml(label)}</h3><p>${escapeHtml(description)}</p></div><span class="status-chip step-status-chip step-status-chip--${currentStatus.toLowerCase().replace("_", "-")}">${statusLabel(currentStatus)}</span></header>
        ${naEligible ? `<div class="na-control">${icon("info")}<div><strong>Dieser Schritt hat für den aktuellen Track keine anwendbaren Pflichtpunkte.</strong><span>N/A wird nur mit einer gespeicherten sachlichen Begründung akzeptiert.</span></div>${currentStatus === "N_A" ? `<button class="button button--secondary" data-reset-na="${stepId}">N/A zurücksetzen</button>` : `<button class="button button--secondary" data-mark-na="${stepId}">Als N/A dokumentieren</button>`}</div>` : ""}
        ${this.renderStepContent(track, stepId)}
        <footer class="editor-footer">
          <button class="button button--secondary" ${index === 0 ? "disabled" : ""} data-step-open="${runtimeSteps[Math.max(index - 1, 0)].id}">${icon("arrow")} Zurück</button>
          ${index < runtimeSteps.length - 1 ? `<button class="button button--dark" data-step-open="${runtimeSteps[index + 1].id}">Weiter: ${escapeHtml(runtimeSteps[index + 1].title)} ${icon("arrow")}</button>` : ""}
        </footer>
      </section>
    </div>`;
  }

  private renderStepContent(track: TrackDetail, stepId: StepId): string {
    const draft = this.state.trackDraft ?? track.fields;
    const conditional = visibleConditionalFields(draft, track.profileSnapshot);
    if (stepId === "evidence_licenses") return this.renderEvidence(track, true);
    if (stepId === "integrity") return this.renderIntegrity(track);
    if (stepId === "finalize") return this.renderFinalization(track);

    let body = "";
    switch (stepId) {
      case "track":
        body = `<div class="field-grid two-col">${this.textField("title", "Track-Titel", "Name des Tracks", draft.title, true)}${this.dateField("productionStartDate", "Produktionsstart", draft.productionStartDate, true)}${this.dateField("productionEndDate", "Produktionsende", draft.productionEndDate, true)}</div>
          <div class="form-section">${this.boolQuestion("commercialUseIntended", "Kommerzielle Nutzung vorgesehen?", "Der tatsächlich für diesen Track verwendete Wert wird im Dokument-Snapshot gespeichert.", draft.commercialUseIntended)}</div>`;
        break;
      case "source":
        body = `${this.boolQuestion("externalAudioUploaded", "Externes Audio hochgeladen?", "Audio außerhalb der eigenen Produktion, das Suno als Quelle erhalten hat.", draft.externalAudioUploaded)}
          ${conditional.has("externalAudioSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.textField("externalAudioSource", "Quelle", "Woher stammt die Datei?", draft.externalAudioSource, true)}${this.textField("externalAudioOwnership", "Rechtezuordnung", "Eigentum / Lizenz", draft.externalAudioOwnership, true)}</div>${this.inlineEvidenceActions(track, [["external_audio_file", "Audiodatei importieren"], ["external_audio_license", "Lizenznachweis importieren"]])}</div>` : ""}
          ${this.boolQuestion("ownAudioUploaded", "Eigene Audiodatei hochgeladen?", "Eine von dir erstellte Aufnahme oder Instrumentalspur.", draft.ownAudioUploaded)}
          ${conditional.has("ownAudioSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.textField("ownAudioSource", "Quelle", "z. B. eigene Aufnahme", draft.ownAudioSource, true)}${this.textField("ownAudioOwnership", "Rechtezuordnung", "Eigene Produktion / Mitwirkende", draft.ownAudioOwnership, true)}</div>${this.inlineEvidenceActions(track, [["own_audio_file", "Eigene Audiodatei importieren"]])}</div>` : ""}
          ${this.boolQuestion("thirdPartySamplesUploaded", "Fremde Samples hochgeladen?", "Samples oder Loops, die von Dritten stammen.", draft.thirdPartySamplesUploaded)}
          ${conditional.has("thirdPartySampleSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.textField("thirdPartySampleSource", "Sample-Quelle", "Bibliothek oder Anbieter", draft.thirdPartySampleSource, true)}${this.textField("thirdPartySampleOwnership", "Lizenz / Rechte", "Lizenzmodell oder Rechteinhaber", draft.thirdPartySampleOwnership, true)}</div>${this.inlineEvidenceActions(track, [["third_party_sample_file", "Sample-Datei importieren"], ["third_party_sample_license", "Sample-Lizenz importieren"]])}</div>` : ""}`;
        break;
      case "suno":
        body = `<div class="field-grid two-col">${this.textField("sunoModel", "Suno-Modell", "z. B. v4.5", draft.sunoModel, true)}${this.textField("sunoPlanAtCreation", "Tarif bei Erstellung", "z. B. Premier", draft.sunoPlanAtCreation, true)}${this.textField("sunoProjectUrl", "Suno-Projekt-URL", "https://suno.com/song/…", draft.sunoProjectUrl, true, "url")}${this.dateField("finalExportDate", "Finaler Export", draft.finalExportDate, true)}</div>
          <div class="form-section"><p class="field-label">Suno-Projektnachweis</p>${this.inlineEvidenceActions(track, [["suno_screenshot", "Screenshot importieren"], ["suno_project_zip", "Projekt-ZIP importieren"], ["suno_final_export", "Suno-Export importieren"]])}</div>`;
        break;
      case "human_work":
        body = `<div class="field-grid two-col">${this.selectField("lyricsSource", "Lyrics-Quelle", draft.lyricsSource, [["", "Bitte auswählen"], ["instrumental", "Instrumental – keine Lyrics"], ["human", "Menschlich geschrieben"], ["suno", "Von Suno erzeugt"], ["mixed", "Gemischt"]], true)}</div>
          ${conditional.has("lyricsText") ? this.textArea("lyricsText", "Menschliche Lyrics", "Nur die tatsächlich verwendete Fassung dokumentieren.", draft.lyricsText, true) : ""}
          ${this.boolQuestion("humanEditingPerformed", "Menschliche Bearbeitung durchgeführt?", "Nur bestätigen, wenn sie tatsächlich stattgefunden hat.", draft.humanEditingPerformed)}
          ${conditional.has("humanEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.textArea("humanEditingDetails", "Bestätigte Schritte", "z. B. Cuts, Track Editing, EQ-Preset", draft.humanEditingDetails, true)}</div>` : ""}
          ${this.boolQuestion("postExportEditingPerformed", "Nach dem Suno-Export weiter bearbeitet?", "Bearbeitung der exportierten Audiodatei.", draft.postExportEditingPerformed)}
          ${conditional.has("postExportEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.textArea("postExportEditingDetails", "Nachbearbeitung", "z. B. Lautheitsanpassung und finaler Schnitt", draft.postExportEditingDetails, true)}</div>` : ""}`;
        break;
      case "artwork":
        body = `<div class="field-grid two-col">${this.selectField("artworkOrigin", "Entstehung des Artworks", draft.artworkOrigin, [["", "Bitte auswählen"], ["none", "Kein Artwork"], ["human", "Menschlich erstellt"], ["ai_generated", "KI-generiert"], ["ai_assisted", "KI-assistiert"]], true)}${conditional.has("aiImageService") ? this.textField("aiImageService", "KI-Bilddienst", "Verwendeter Dienst", draft.aiImageService, true) : ""}</div>
          ${conditional.has("humanArtworkModifications") ? this.textArea("humanArtworkModifications", "Menschliche Änderungen", "Nur tatsächlich ausgeführte Änderungen", draft.humanArtworkModifications, true) : ""}
          ${conditional.has("aiArtworkOriginal") ? `<div class="form-section"><p class="field-label">Originale KI-Ausgabe</p>${this.inlineEvidenceActions(track, [["ai_artwork_original", "KI-Original importieren"], ["ai_artwork_edited", "KI-bearbeitete Version importieren"], ["human_edited_artwork", "Menschlich bearbeitete Version importieren"]])}</div>` : ""}
          ${draft.artworkOrigin && draft.artworkOrigin !== "none" ? `<div class="question-group"><div><p class="overline">Kurzer Content-Check</p><h4>Nur relevante Angaben</h4><p>Die App dokumentiert deine Bestätigung und trifft keine rechtliche Entscheidung.</p></div>${this.boolQuestion("depictsRealPerson", "Zeigt das Artwork absichtlich eine reale Person?", "", draft.depictsRealPerson)}${conditional.has("realPersonNotes") ? this.textArea("realPersonNotes", "Notiz zur realen Person", "Darstellung und Kontext", draft.realPersonNotes, true) : ""}${this.boolQuestion("depictsRealEvent", "Stellt es ein reales Ereignis als authentisch dar?", "", draft.depictsRealEvent)}${conditional.has("realEventNotes") ? this.textArea("realEventNotes", "Notiz zum realen Ereignis", "Darstellung und Kontext", draft.realEventNotes, true) : ""}${this.boolQuestion("containsTrademark", "Reproduziert es eine Marke oder ein Firmenlogo?", "", draft.containsTrademark)}${conditional.has("trademarkNotes") ? this.textArea("trademarkNotes", "Notiz zur Marke / zum Logo", "Darstellung und Kontext", draft.trademarkNotes, true) : ""}</div>` : ""}`;
        break;
      case "ai_transparency":
        body = `<div class="policy-card">${icon("info")}<div><p class="overline">Projektinterne Transparenzrichtlinie · Track-Snapshot</p><h4>${this.policyLabel(track.profileSnapshot.artworkTransparencyPolicy)}</h4><p>Dies ist die beim Anlegen gespeicherte Projektregel – keine pauschale gesetzliche Aussage. Spätere globale Änderungen verändern diesen Track nicht.</p></div></div>
          ${conditional.has("disclosure") ? `<div class="field-grid two-col">${this.textField("disclosureText", "Sichtbarer Hinweis", "AI-assisted", draft.disclosureText, true)}${track.profileSnapshot.artworkTransparencyPolicy === "per_artwork" ? this.boolQuestion("disclosureApplied", "Sichtbaren Hinweis anwenden?", "Bei Ja muss die gekennzeichnete Fassung lokal erzeugt werden.", draft.disclosureApplied) : `<div class="read-only-field"><span>Status</span><strong>${draft.disclosureApplied ? "Lokal erzeugt" : "Noch nicht erzeugt"}</strong></div>`}</div><button type="button" class="button button--accent" data-action="generate-disclosure">${icon("certificate")} Sichtbaren Hinweis lokal erzeugen</button>` : `<div class="neutral-message">${icon("check")}<div><strong>Kein automatischer Transparenzschritt erforderlich.</strong><span>Grundlage: Artwork-Angabe und aktive Workspace-Policy.</span></div></div>`}`;
        break;
      case "release":
        body = `<div class="field-grid two-col">${this.dateField("finalExportDate", "Finaler Export", draft.finalExportDate, true)}${this.textArea("releaseNotes", "Release-Notizen", "Optional: Format, Version oder Ziel", draft.releaseNotes)}</div>
          <div class="form-section"><p class="field-label">Finale Release-Dateien</p><p class="field-help">WAV ist für die Finalisierung erforderlich. Wenn die KI-Disclosure aktiv ist, wähle als finales Artwork exakt die lokal erzeugte <code>_AI_EDITED.png</code>; die App prüft die Byte-Identität.</p>${this.inlineEvidenceActions(track, [["release_wav", "Release-WAV importieren"], ["release_mp3", "MP3 importieren"], ["release_mp4", "MP4 importieren"], ["final_artwork", "Finales Artwork importieren"]])}</div>`;
        break;
    }
    return `<form id="track-step-form" class="workflow-form" data-step="${stepId}">${body}<div class="form-save"><span>${icon("shield")} Änderungen bleiben lokal im Workspace.</span><button class="button button--primary" type="submit">${icon("check")} Schritt speichern</button></div></form>`;
  }

  private renderEvidence(track: TrackDetail, embedded = false): string {
    const missing = calculateMissingRequirements(track, track.profileSnapshot).filter((item) => item.evidenceRole);
    return `<div class="${embedded ? "embedded-content" : "evidence-page"}">
      <div class="section-intro"><div><p class="overline">Lokale Nachweise</p><h3>Evidence & Lizenzen</h3><p>Originale bleiben am Quellort. Importierte Kopien werden gehasht und niemals still überschrieben.</p></div><button class="button button--primary" data-action="import-evidence">${icon("upload")} Evidence importieren</button></div>
      ${missing.length ? `<div class="evidence-needed"><strong>${missing.length} erforderliche Nachweise fehlen</strong><div>${missing.map((item) => item.evidenceRole === "subscription_payment" ? `<span class="evidence-reminder">${icon("info")} ${escapeHtml(item.label)} – unten aus globaler Evidence zuordnen</span>` : `<button data-import-role="${item.evidenceRole}">${icon("plus")} ${escapeHtml(item.label)}</button>`).join("")}</div></div>` : ""}
      ${track.fields.commercialUseIntended ? this.renderGlobalEvidencePicker(track) : ""}
      <div class="panel evidence-table-panel"><div class="evidence-table-head"><span>Datei</span><span>Rolle</span><span>Integrität</span><span>Größe</span><span></span></div>
        ${track.evidence.length ? `<div class="evidence-list">${track.evidence.map((item) => `<div class="evidence-row"><span class="file-icon">${icon("file")}</span><span class="evidence-name"><strong>${escapeHtml(item.fileName)}</strong><small>${escapeHtml(item.relativePath)} · ${escapeHtml(evidenceProvenanceLabel(item.provenance))}</small></span><span>${escapeHtml(evidenceRoleLabel(item.role))}</span><span class="verification ${item.verified ? "is-valid" : ""}">${item.verified ? icon("check") + " Verifiziert" : "Nicht verifiziert"}</span><span>${formatBytes(item.sizeBytes)}</span><span class="row-actions"><button class="icon-button" data-verify-evidence="${item.id}" aria-label="Evidence prüfen">${icon("shield")}</button><button class="icon-button danger" data-remove-evidence="${item.id}" aria-label="Evidence entfernen">${icon("trash")}</button></span></div>`).join("")}</div>` : this.emptyState("file", "Noch keine Evidence", "Importiere echte lokale Dateien über den nativen Dateidialog.")}
      </div>
      <div class="deviation-section"><div class="section-intro compact"><div><p class="overline">Abweichungen</p><h3>Offene Hinweise & Blocker</h3></div><button class="button button--secondary" data-action="add-deviation">${icon("plus")} Abweichung erfassen</button></div>${this.renderDeviations(track)}</div>
    </div>`;
  }

  private renderDeviations(track: TrackDetail): string {
    const deviations = track.blockingDeviations ?? [];
    if (!deviations.length) return `<div class="neutral-message">${icon("check")}<div><strong>Keine Abweichungen erfasst.</strong><span>Ungeklärte blockierende Abweichungen verhindern die Finalisierung.</span></div></div>`;
    return `<div class="deviation-list">${deviations.map((item) => `<article class="deviation ${item.resolved ? "is-resolved" : ""}">${icon(item.resolved ? "check" : "alert")}<div><strong>${escapeHtml(item.title)}</strong><p>${escapeHtml(item.description)}</p><small>${item.resolved ? `Gelöst ${formatDate(item.resolvedAt, true)}` : `Erfasst ${formatDate(item.createdAt, true)}`}</small></div><div>${!item.resolved ? `<button class="button button--small button--secondary" data-resolve-deviation="${item.id}">Als gelöst markieren</button>` : ""}<button class="icon-button danger" data-remove-deviation="${item.id}" aria-label="Abweichung entfernen">${icon("trash")}</button></div></article>`).join("")}</div>`;
  }

  private renderGlobalEvidencePicker(track: TrackDetail): string {
    const attached = track.evidence.some((item) => item.role === "subscription_payment" && item.verified && Boolean(item.sha256)
      && Boolean(item.coverageStart) && Boolean(item.coverageEnd)
      && item.coverageStart! <= track.fields.productionStartDate && item.coverageEnd! >= track.fields.productionEndDate);
    return `<section class="global-picker"><div><p class="overline">Global registriert</p><h4>Abo-Nachweis für Produktionszeitraum</h4><p>Beim Zuordnen kopiert der native Dienst den Nachweis in den Track-Ordner, damit der finale Ordner eigenständig bleibt.</p></div>${this.state.globalEvidence.length ? `<div>${this.state.globalEvidence.map((item) => {
      const covers = Boolean(item.coverageStart) && Boolean(item.coverageEnd)
        && Boolean(track.fields.productionStartDate) && Boolean(track.fields.productionEndDate)
        && item.coverageStart! <= track.fields.productionStartDate
        && item.coverageEnd! >= track.fields.productionEndDate;
      return `<article class="${covers ? "is-covering" : ""}">${icon("file")}<span><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart)} – ${formatDate(item.coverageEnd)} · ${covers ? "Zeitraum passend" : "Deckt den Track-Zeitraum nicht ab"}</small></span><button class="button button--small button--secondary" data-attach-global="${item.id}" ${attached || !covers ? "disabled" : ""}>${attached ? "Zugeordnet" : covers ? "Diesem Track zuordnen" : "Nicht passend"}</button></article>`;
    }).join("")}</div>` : `<p class="empty-inline">Noch keine globale Abo-Evidence. Registriere sie unter Einstellungen.</p>`}</section>`;
  }

  private renderIntegrity(track: TrackDetail): string {
    const mismatches = track.integrity.mismatchFiles;
    return `<div class="integrity-page">
      <div class="integrity-hero ${track.integrity.verified && !mismatches.length ? "is-valid" : ""}"><span>${icon("shield")}</span><div><p class="overline">SHA-256 Integrität</p><h3>${track.integrity.verified ? "Dateien erfolgreich verifiziert" : "Integritätsprüfung ausstehend"}</h3><p>${track.integrity.fileCount} Dateien gehasht · ${track.integrity.verifiedCount} Dateien verifiziert</p></div><strong>${track.integrity.verified && !mismatches.length ? "PASS" : track.integrity.generated ? "NICHT VERIFIZIERT" : "NICHT ERZEUGT"}</strong></div>
      ${mismatches.length ? `<div class="danger-banner">${icon("alert")}<div><strong>${mismatches.length} Integritätsabweichungen</strong><span>${mismatches.map(escapeHtml).join(", ")}</span></div></div>` : ""}
      <div class="integrity-actions"><article class="action-card">${icon("file")}<div><h4>1. Dokumente erzeugen</h4><p>Versionierte Markdown- und Textdokumente aus den aktuellen Angaben erstellen.</p><span>${track.documents.current ? "Aktuell · " + formatDate(track.documents.generatedAt, true) : "Ausstehend oder veraltet"}</span></div><button class="button button--secondary" data-action="generate-documents">Erzeugen</button></article>
      <article class="action-card">${icon("hash")}<div><h4>2. SHA-256 berechnen</h4><p>Alle relevanten Dateien in einer extern prüfbaren Hashliste erfassen.</p><span>${track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Ausstehend"}</span></div><button class="button button--secondary" data-action="calculate-hashes" ${!track.documents.current ? "disabled" : ""}>Berechnen</button></article>
      <article class="action-card">${icon("shield")}<div><h4>3. Prüfsummen verifizieren</h4><p>Hashliste erneut lesen und jede erfasste Datei nativ überprüfen.</p><span>${track.integrity.verified ? formatDate(track.integrity.verifiedAt, true) : "Ausstehend"}</span></div><button class="button button--primary" data-action="verify-hashes" ${!track.integrity.generated ? "disabled" : ""}>Verifizieren</button></article></div>
      <div class="technical-note">${icon("info")}<p><strong>Unabhängig prüfbar.</strong> SHA256SUMS.txt bleibt möglichst mit <code>sha256sum -c</code> kompatibel. Zertifikat, Archiv und interne Verwaltungsdaten werden nicht in dieselbe Hashliste aufgenommen.</p></div>
    </div>`;
  }

  private renderFinalization(track: TrackDetail): string {
    const localGate = finalizationGate(track, track.profileSnapshot);
    const blockers = [...localGate.missingItems, ...localGate.blockingItems];
    return `<div class="finalize-page">
      <div class="finalize-mark ${localGate.valid ? "is-ready" : ""}">${icon(localGate.valid ? "certificate" : "lock")}</div>
      <p class="overline">Track Documentation Completion Certificate</p><h3>${localGate.valid ? "Bereit für den Abschluss" : "Finalisierung noch blockiert"}</h3><p>${localGate.valid ? "Die UI-Vorprüfung ist vollständig. Der native Dienst validiert vor dem Erzeugen des Zertifikats nochmals alle Pflichtschritte, Evidence und Hashes." : `${blockers.length} Punkte müssen vor dem Abschluss geklärt werden.`}</p>
      ${blockers.length ? `<ul class="gate-list">${blockers.map((item) => `<li>${icon("alert")}<span>${escapeHtml(item)}</span></li>`).join("")}</ul>` : `<ul class="gate-list gate-list--success"><li>${icon("check")}<span>Pflichtschritte erfüllt</span></li><li>${icon("check")}<span>Evidence vollständig</span></li><li>${icon("check")}<span>Dokumente aktuell</span></li><li>${icon("check")}<span>SHA-256 vollständig verifiziert</span></li></ul>`}
      <button class="button button--finalize" data-action="finalize-track" ${!localGate.valid || track.status === "FINALIZED" ? "disabled" : ""}>${icon("certificate")} Dokumentation finalisieren</button>
      <p class="certificate-disclaimer">Das Zertifikat bestätigt ausschließlich den Abschluss des konfigurierten Dokumentations- und Integritätsworkflows. Es ist keine behördliche Zertifizierung, Rechtsberatung oder unabhängige Feststellung von Urheberschaft oder Rechtskonformität.</p>
    </div>`;
  }

  private renderCertificate(track: TrackDetail): string {
    if (!track.certificate.certificateId) return `<div class="certificate-empty">${icon("certificate")}<p class="overline">Zertifikat</p><h3>Noch kein Completion Certificate</h3><p>Das Zertifikat wird erst nach erfolgreicher nativer Finalisierungsprüfung erzeugt.</p><button class="button button--dark" data-step-open="finalize">Finalisierungs-Gate öffnen ${icon("arrow")}</button></div>`;
    const deviations = (track.blockingDeviations ?? []).filter((item) => item.blocking && !item.resolved);
    return `<div class="certificate-view ${track.certificate.valid ? "is-valid" : "is-invalid"}">
      <div class="certificate-paper"><header><div class="certificate-seal">${icon("certificate")}</div><div><p>Suno Documentation Manager</p><h3>Track Documentation<br>Completion Certificate</h3></div><span class="certificate-result">${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "CERTIFICATE INVALID"}</span></header>
      <div class="certificate-rule"></div><dl><div><dt>Certificate ID</dt><dd>${escapeHtml(track.certificate.certificateId)}</dd></div><div><dt>Track</dt><dd>${escapeHtml(track.title)}</dd></div><div><dt>Artist</dt><dd>${escapeHtml(track.profileSnapshot.artistName)}</dd></div><div><dt>Workflow</dt><dd>${escapeHtml(track.workflowId)} · ${escapeHtml(track.certificate.workflowVersion ?? track.workflowVersion)}</dd></div><div><dt>Finalisierung</dt><dd>${formatDate(track.certificate.finalizedAt, true)}</dd></div><div><dt>Evidence-Dateien</dt><dd>${track.evidence.length}</dd></div><div><dt>Blockierende Abweichungen</dt><dd>${deviations.length}</dd></div><div><dt>Finales Ergebnis</dt><dd>${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "INVALID"}</dd></div></dl>
      <footer>This certificate confirms completion of the configured documentation workflow and integrity checks. It does not constitute governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.</footer></div>
      <div class="certificate-actions">${track.certificate.valid ? `<button class="button button--danger-soft" data-action="invalidate-certificate">Zertifikat invalidieren</button>` : `<button class="button button--primary" data-action="create-revision">Neue Revision anlegen</button>`}</div>
    </div>`;
  }

  private renderWorkspace(): string {
    const scan = this.state.scanResult;
    return `<div class="page-content workspace-page">
      <div class="page-lead"><div><p class="overline">Lokaler Projektordner</p><h2>${escapeHtml(this.state.workspace?.name)}</h2><p class="path-display">${icon("workspace")} ${escapeHtml(this.state.workspace?.path)}</p></div><button class="button button--primary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button></div>
      <section class="workspace-stats"><article><span>${icon("tracks")}</span><div><strong>${this.state.tracks.length}</strong><small>indexierte Tracks</small></div></article><article><span>${icon("scan")}</span><div><strong>${formatDate(this.state.workspace?.lastScannedAt)}</strong><small>zuletzt gescannt</small></div></article><article><span>${icon("shield")}</span><div><strong>Lokal</strong><small>SQLite + Track-Ordner</small></div></article></section>
      <section class="panel workspace-scan"><div class="panel-heading"><div><p class="overline">Bestehende Projekte</p><h3>Legacy-Track-Import</h3><p>Der Scan erkennt bekannte Ordner, Evidence und Hashlisten. Bestehende Dateien werden dabei niemals verändert.</p></div></div>
        ${scan ? `<div class="scan-summary"><span><strong>${scan.discovered}</strong> erkannt</span><span><strong>${scan.indexed}</strong> indexiert</span><span><strong>${scan.unchanged}</strong> unverändert</span></div>${scan.warnings.length ? `<ul class="warning-list">${scan.warnings.map((warning) => `<li>${icon("alert")} ${escapeHtml(warning)}</li>`).join("")}</ul>` : ""}${scan.candidates?.length ? `<div class="candidate-list">${scan.candidates.map((candidate) => `<article><span class="file-icon">${icon("workspace")}</span><div><strong>${escapeHtml(candidate.name)}</strong><small>${escapeHtml(candidate.relativePath)}</small><p>${candidate.missingItems.length ? `Fehlt: ${candidate.missingItems.map(escapeHtml).join(", ")}` : "Bekannte Struktur erkannt"}</p></div><span class="status-chip">${candidate.status === "NOT_VERIFIED" ? "Nicht verifiziert" : candidate.status === "INCOMPLETE" ? "Unvollständig" : "Indexiert"}</span>${candidate.hasManagedDocumentCollision ? `<span class="collision-note">${icon("alert")} Bestehendes Dokument – keine Übernahme ohne Bestätigung und Sicherung</span>` : ""}</article>`).join("")}</div>` : ""}` : `<div class="scan-placeholder">${icon("scan")}<div><strong>Noch kein Scan in dieser Sitzung</strong><span>Legacy-Projekte beginnen als „Nicht verifiziert“. Fehlende historische Angaben werden nie erfunden.</span></div></div>`}
      </section>
      <section class="panel safety-panel"><div>${icon("lock")}<div><h3>Dateisystem-Sicherheit</h3><p>Alle Dateizugriffe laufen über enge native Commands. Pfade werden kanonisiert, Traversal und Symlink-Escapes abgewiesen und Schreibvorgänge atomar ausgeführt.</p></div></div><ul><li>${icon("check")} Kein stilles Überschreiben</li><li>${icon("check")} Relative Track-Pfade</li><li>${icon("check")} Originale bleiben erhalten</li></ul></section>
      <div class="workspace-actions"><button class="button button--secondary" data-action="open-workspace">Anderen Workspace öffnen</button><button class="button button--secondary" data-action="create-workspace">Neuen Workspace anlegen</button></div>
    </div>`;
  }

  private renderSettings(): string {
    const profile = this.state.profile;
    return `<div class="page-content settings-page">
      <div class="page-lead"><div><p class="overline">Workspace-Stammdaten</p><h2>Globale Angaben</h2><p>Diese Werte werden einmal gespeichert und als tatsächlicher Snapshot in jedes Track-Dokument übernommen.</p></div></div>
      <form id="profile-form" class="panel settings-form">
        <div class="settings-section"><div class="settings-section-copy"><span>01</span><div><h3>Artist & Suno</h3><p>Nur produktionsrelevante Profildaten – keine privaten Kontaktdaten.</p></div></div><div class="field-grid two-col">${this.textField("artistName", "Künstlername", "Künstlername", profile.artistName, true)}${this.textField("sunoProfileName", "Suno-Profilname", "Profilname", profile.sunoProfileName, true)}${this.textField("sunoHandle", "Suno-Benutzername", "@handle", profile.sunoHandle, true)}${this.textField("sunoPlan", "Suno-Tarif", "z. B. Premier", profile.sunoPlan, true)}${this.dateField("subscriptionStartDate", "Abo-Startdatum", profile.subscriptionStartDate, true)}</div></div>
        <div class="settings-section"><div class="settings-section-copy"><span>02</span><div><h3>Standards</h3><p>Vorbelegte Werte können pro Track angepasst werden.</p></div></div><div class="field-grid two-col">${this.textField("defaultAiImageService", "Standard-KI-Bilddienst", "z. B. OpenAI", profile.defaultAiImageService)}${this.boolQuestion("defaultCommercialUse", "Kommerzielle Nutzung standardmäßig vorgesehen?", "", profile.defaultCommercialUse)}</div></div>
        <div class="settings-section"><div class="settings-section-copy"><span>03</span><div><h3>Artwork-Transparenz</h3><p>Projektinterne Richtlinie; keine pauschale gesetzliche Kennzeichnungspflicht.</p></div></div><div>${this.radioCards("artworkTransparencyPolicy", profile.artworkTransparencyPolicy, [["always", "Immer sichtbaren KI-Hinweis hinzufügen", "Empfohlener Projektstandard"], ["per_artwork", "Pro Artwork entscheiden", "Entscheidung wird je Track dokumentiert"], ["none", "Kein automatischer sichtbarer Hinweis", "Nur Prozessdokumentation"]])}${this.textField("disclosureText", "Standard-Hinweistext", "AI-assisted", profile.disclosureText, true)}</div></div>
        <div class="form-save settings-save"><span>${icon("shield")} Stammdaten verbleiben in der lokalen Workspace-Datenbank.</span><button class="button button--primary" type="submit">${icon("check")} Einstellungen speichern</button></div>
      </form>
      <section class="panel global-evidence-panel"><div class="panel-heading"><div><p class="overline">Wiederverwendbare Nachweise</p><h3>Suno-Abo-Evidence</h3><p>Registriere Rechnungen einmal und ordne den passenden Produktionszeitraum einem Track zu.</p></div><button class="button button--secondary" data-action="import-global-evidence">${icon("upload")} Abo-Nachweis registrieren</button></div>
        ${this.state.globalEvidence.length ? `<div class="global-evidence-list">${this.state.globalEvidence.map((item) => `<article><span class="file-icon">${icon("file")}</span><div><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart)} – ${formatDate(item.coverageEnd)}</small></div><span class="verification is-valid">${icon("check")} Gehasht</span><button class="icon-button danger" data-remove-global-evidence="${item.id}" aria-label="Globalen Nachweis entfernen">${icon("trash")}</button></article>`).join("")}</div>` : `<p class="empty-inline">Noch kein globaler Abo-Nachweis registriert.</p>`}
      </section>
    </div>`;
  }

  private inlineEvidenceActions(track: TrackDetail, actions: Array<[EvidenceRole, string]>): string {
    return `<div class="inline-evidence">${actions.map(([role, label]) => {
      const present = track.evidence.some((item) => item.role === role && item.verified);
      return `<button type="button" class="evidence-button ${present ? "is-present" : ""}" data-import-role="${role}">${present ? icon("check") : icon("upload")}<span><strong>${escapeHtml(label)}</strong><small>${present ? "Vorhanden und verifiziert" : evidenceRoleLabel(role)}</small></span></button>`;
    }).join("")}</div>`;
  }

  private textField(name: string, label: string, placeholder: string, value: string, required = false, type = "text"): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><input type="${type}" name="${name}" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${required ? "required" : ""}></label>`;
  }

  private dateField(name: string, label: string, value: string, required = false): string {
    return this.textField(name, label, "", value, required, "date");
  }

  private textArea(name: string, label: string, placeholder: string, value: string, required = false): string {
    return `<label class="field field--wide"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><textarea name="${name}" placeholder="${escapeHtml(placeholder)}" ${required ? "required" : ""}>${escapeHtml(value)}</textarea></label>`;
  }

  private selectField(name: string, label: string, value: string, options: Array<[string, string]>, required = false): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><select name="${name}" ${required ? "required" : ""}>${options.map(([id, text]) => `<option value="${escapeHtml(id)}" ${value === id ? "selected" : ""}>${escapeHtml(text)}</option>`).join("")}</select></label>`;
  }

  private boolQuestion(name: string, label: string, help: string, value: boolean | null): string {
    return `<fieldset class="boolean-field"><legend>${escapeHtml(label)}</legend>${help ? `<p>${escapeHtml(help)}</p>` : ""}<div><label><input type="radio" name="${name}" value="true" ${value === true ? "checked" : ""}><span>${icon("check")} Ja</span></label><label><input type="radio" name="${name}" value="false" ${value === false ? "checked" : ""}><span>${icon("close")} Nein</span></label></div></fieldset>`;
  }

  private radioCards(name: string, value: string, options: Array<[string, string, string]>): string {
    return `<div class="radio-cards">${options.map(([id, label, help]) => `<label><input type="radio" name="${name}" value="${id}" ${id === value ? "checked" : ""}><span><i></i><strong>${escapeHtml(label)}</strong><small>${escapeHtml(help)}</small></span></label>`).join("")}</div>`;
  }

  private policyLabel(policy: GlobalProfile["artworkTransparencyPolicy"]): string {
    return policy === "always" ? "Immer sichtbaren KI-Hinweis hinzufügen" : policy === "per_artwork" ? "Pro Artwork entscheiden" : "Kein automatischer sichtbarer Hinweis";
  }

  private emptyState(iconName: "tracks" | "current" | "file", title: string, copy: string, action?: string, actionLabel?: string): string {
    return `<div class="empty-state">${icon(iconName)}<h3>${escapeHtml(title)}</h3><p>${escapeHtml(copy)}</p>${action ? `<button class="button button--secondary" data-action="${action}">${escapeHtml(actionLabel)}</button>` : ""}</div>`;
  }

  private async handleClick(event: Event): Promise<void> {
    const target = event.target as HTMLElement;
    const button = target.closest<HTMLElement>("button, [data-action], [data-view], [data-track-open], [data-step-open], [data-track-tab]");
    if (!button) return;
    if (button.closest("[data-modal-panel]") && target === button.closest("[data-modal-panel]")) return;

    const view = button.dataset.view as MainView | undefined;
    if (view) {
      this.state.view = view;
      this.state.sidebarOpen = false;
      this.state.activeStep = null;
      this.render();
      return;
    }
    const trackId = button.dataset.trackOpen;
    if (trackId) {
      if (!(await this.flushDraft())) return;
      const track = await this.withBusy("Track wird geladen …", () => this.api.loadTrack(trackId));
      if (track) {
        this.applyTrack(track);
        this.state.view = "current";
        this.state.trackTab = "overview";
        this.state.activeStep = null;
      }
      return;
    }
    const stepId = button.dataset.stepOpen as StepId | undefined;
    if (stepId) {
      this.state.activeStep = stepId;
      this.state.view = "current";
      this.render();
      return;
    }
    const tab = button.dataset.trackTab as TrackTab | undefined;
    if (tab) {
      this.state.trackTab = tab;
      this.state.activeStep = null;
      this.render();
      return;
    }
    const filter = button.dataset.trackFilter as AppState["trackFilter"] | undefined;
    if (filter) {
      this.state.trackFilter = filter;
      this.render();
      return;
    }
    const importRole = button.dataset.importRole as EvidenceRole | undefined;
    if (importRole) {
      await this.importEvidence(importRole);
      return;
    }
    if (button.dataset.verifyEvidence) {
      await this.trackMutation("Evidence wird geprüft …", () => this.api.verifyEvidence(this.requireTrack().id, button.dataset.verifyEvidence), "Evidence verifiziert");
      return;
    }
    if (button.dataset.removeEvidence) {
      const selected = this.requireTrack().evidence.find((item) => item.id === button.dataset.removeEvidence);
      const prompt = selected?.provenance === "indexed_legacy"
        ? "Historisch indexierte Evidence entfernen? Die Datei wird nachvollziehbar unter .archive/removals gesichert und nicht gelöscht."
        : "Importierte Evidence-Kopie aus dem Track entfernen? Die Originaldatei am Quellort bleibt erhalten.";
      if (window.confirm(prompt)) {
        await this.trackMutation("Evidence wird entfernt …", () => this.api.removeEvidence(this.requireTrack().id, button.dataset.removeEvidence!), "Evidence entfernt");
      }
      return;
    }
    if (button.dataset.removeGlobalEvidence) {
      if (window.confirm("Global registrierten Nachweis entfernen? Bereits in Tracks kopierte Evidence bleibt bestehen.")) {
        await this.withBusy("Nachweis wird entfernt …", () => this.api.removeGlobalEvidence(button.dataset.removeGlobalEvidence!));
        this.state.globalEvidence = await this.api.listGlobalEvidence();
        this.showToast("success", "Nachweis entfernt", "Bereits zugeordnete Track-Kopien wurden nicht verändert.");
      }
      return;
    }
    if (button.dataset.attachGlobal) {
      await this.trackMutation("Abo-Nachweis wird in den Track kopiert …", () => this.api.attachGlobalEvidence(this.requireTrack().id, button.dataset.attachGlobal!), "Abo-Nachweis zugeordnet");
      return;
    }
    if (button.dataset.resolveDeviation) {
      await this.trackMutation("Abweichung wird gelöst …", () => this.api.resolveDeviation(this.requireTrack().id, button.dataset.resolveDeviation!), "Abweichung gelöst");
      return;
    }
    if (button.dataset.markNa) {
      if (!(await this.flushDraft())) return;
      const reason = window.prompt("Warum ist dieser Schritt für den Track nicht anwendbar? Eine konkrete Begründung ist erforderlich.");
      if (!reason?.trim()) return;
      await this.trackMutation("N/A-Begründung wird gespeichert …", () => this.api.setStepStatus(this.requireTrack().id, button.dataset.markNa as StepId, "N_A", reason.trim()), "N/A dokumentiert");
      return;
    }
    if (button.dataset.resetNa) {
      await this.trackMutation("Schrittstatus wird zurückgesetzt …", () => this.api.setStepStatus(this.requireTrack().id, button.dataset.resetNa as StepId, "NOT_RUN"), "Schrittstatus zurückgesetzt");
      return;
    }
    if (button.dataset.removeDeviation) {
      await this.trackMutation("Abweichung wird entfernt …", () => this.api.removeDeviation(this.requireTrack().id, button.dataset.removeDeviation!), "Abweichung entfernt");
      return;
    }

    switch (button.dataset.action) {
      case "open-workspace": await this.chooseWorkspace("open"); break;
      case "create-workspace": await this.chooseWorkspace("create"); break;
      case "new-track": {
        if (!(await this.flushDraft())) break;
        const missing = missingProfileFields(this.state.profile);
        if (missing.length) {
          this.state.view = "settings";
          this.showToast("info", "Zuerst Stammdaten vervollständigen", `Für einen unveränderlichen Track-Snapshot fehlen: ${missing.join(", ")}.`);
          this.render();
        } else {
          this.state.showNewTrack = true;
          this.render();
        }
        break;
      }
      case "close-modal": this.state.showNewTrack = false; this.render(); break;
      case "open-sidebar": this.state.sidebarOpen = true; this.render(); break;
      case "close-sidebar": this.state.sidebarOpen = false; this.render(); break;
      case "dismiss-toast": this.state.toast = null; this.render(); break;
      case "go-tracks": this.state.view = "tracks"; this.render(); break;
      case "back-overview": this.state.activeStep = null; this.state.trackTab = "overview"; this.render(); break;
      case "scan-workspace": await this.scanWorkspace(); break;
      case "adopt-legacy-profile":
        if (window.confirm("Treffen die aktuellen Workspace-Stammdaten auf diesen historischen Track zu? Sie werden als Track-Snapshot übernommen.")) await this.trackMutation("Stammdaten werden als Legacy-Snapshot übernommen …", () => this.api.adoptLegacyProfile(this.requireTrack().id), "Legacy-Snapshot übernommen");
        break;
      case "import-evidence": await this.chooseEvidenceRole(); break;
      case "import-global-evidence": await this.importGlobalEvidence(); break;
      case "add-deviation": await this.addDeviation(); break;
      case "generate-documents": await this.generateDocumentsSafely(); break;
      case "generate-disclosure": await this.runAction("KI-Hinweis wird lokal erzeugt …", () => this.api.generateArtworkDisclosure(this.requireTrack().id, this.state.trackDraft?.disclosureText)); break;
      case "calculate-hashes": await this.runAction("SHA-256 wird berechnet …", () => this.api.calculateHashes(this.requireTrack().id)); break;
      case "verify-hashes": await this.runAction("Prüfsummen werden verifiziert …", () => this.api.verifyHashes(this.requireTrack().id)); break;
      case "finalize-track": await this.finalizeTrack(); break;
      case "invalidate-certificate":
        if (window.confirm("Zertifikat als ungültig markieren? Der finalisierte Snapshot wird nicht still überschrieben.")) await this.runAction("Zertifikat wird invalidiert …", () => this.api.invalidateCertificate(this.requireTrack().id));
        break;
      case "create-revision":
        if (window.confirm("Neue Revision anlegen? Der bisherige Certificate-/Manifest-Snapshot wird zuerst unter .archive/revisions gesichert.")) await this.runAction("Neue Revision wird angelegt …", () => this.api.createRevision(this.requireTrack().id));
        break;
    }
  }

  private async handleSubmit(event: SubmitEvent): Promise<void> {
    const form = event.target as HTMLFormElement;
    event.preventDefault();
    if (form.id === "new-track-form") {
      const profileMissing = missingProfileFields(this.state.profile);
      if (profileMissing.length) {
        this.state.showNewTrack = false;
        this.state.view = "settings";
        this.showToast("error", "Track nicht angelegt", `Vervollständige zuerst: ${profileMissing.join(", ")}.`);
        this.render();
        return;
      }
      const data = new FormData(form);
      const track = await this.withBusy("Track-Struktur wird angelegt …", () => this.api.createTrack({
        title: String(data.get("title") ?? ""),
        productionStartDate: String(data.get("productionStartDate") ?? ""),
        commercialUseIntended: data.get("commercialUseIntended") === "on"
      }));
      if (track) {
        this.applyTrack(track);
        this.state.showNewTrack = false;
        this.state.view = "current";
        this.state.activeStep = "track";
        this.showToast("success", "Track angelegt", "Die portable Track-Struktur wurde lokal erstellt.");
      }
      return;
    }
    if (form.id === "profile-form") {
      const data = new FormData(form);
      const profile: GlobalProfile = {
        artistName: String(data.get("artistName") ?? ""), sunoProfileName: String(data.get("sunoProfileName") ?? ""),
        sunoHandle: String(data.get("sunoHandle") ?? ""), sunoPlan: String(data.get("sunoPlan") ?? ""),
        subscriptionStartDate: String(data.get("subscriptionStartDate") ?? ""), defaultCommercialUse: data.get("defaultCommercialUse") === "true",
        defaultAiImageService: String(data.get("defaultAiImageService") ?? ""), artworkTransparencyPolicy: String(data.get("artworkTransparencyPolicy")) as GlobalProfile["artworkTransparencyPolicy"],
        disclosureText: String(data.get("disclosureText") ?? "AI-assisted")
      };
      const saved = await this.withBusy("Stammdaten werden gespeichert …", () => this.api.updateProfile(profile));
      if (saved) { this.state.profile = saved; this.showToast("success", "Einstellungen gespeichert", "Neue Tracks verwenden diese Werte als Vorbelegung."); }
      return;
    }
    if (form.id === "track-step-form") {
      await this.saveTrackDraft();
    }
  }

  private handleChange(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    if (!input.closest("#track-step-form") || !this.state.trackDraft) return;
    const key = input.name as keyof TrackFields;
    if (!key) return;
    let value: string | boolean | null = input.value;
    if (input instanceof HTMLInputElement && input.type === "radio" && (input.value === "true" || input.value === "false")) value = input.value === "true";
    (this.state.trackDraft as unknown as Record<string, unknown>)[key] = value;
    this.draftDirty = true;
    this.render();
  }

  private handleInput(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLTextAreaElement;
    if (input.matches("[data-track-search]")) {
      this.state.query = input.value;
      this.render();
      const next = this.root.querySelector<HTMLInputElement>("[data-track-search]");
      next?.focus();
      next?.setSelectionRange(next.value.length, next.value.length);
      return;
    }
    if (input.closest("#track-step-form") && this.state.trackDraft && input.name) {
      (this.state.trackDraft as unknown as Record<string, unknown>)[input.name] = input.value;
      this.draftDirty = true;
    }
  }

  private requireTrack(): TrackDetail {
    if (!this.state.track) throw new Error("Wähle zuerst einen Track aus.");
    return this.state.track;
  }

  private async chooseWorkspace(kind: "open" | "create"): Promise<void> {
    if (!(await this.flushDraft())) return;
    const workspace = await this.withBusy(kind === "open" ? "Ordnerdialog wird geöffnet …" : "Workspace wird angelegt …", () => kind === "open" ? this.api.openWorkspace() : this.api.createWorkspace());
    if (workspace) await this.enterWorkspace(workspace);
  }

  private async scanWorkspace(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const scan = await this.withBusy("Workspace wird sicher gescannt …", () => this.api.scanWorkspace());
    if (!scan) return;
    this.state.scanResult = scan;
    await this.refreshTracks();
    this.state.view = "workspace";
    this.showToast("success", "Scan abgeschlossen", `${scan.discovered} Track-Ordner erkannt. Es wurden keine bestehenden Dateien überschrieben.`);
  }

  private async importEvidence(role: EvidenceRole): Promise<void> {
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const imported = await this.withBusy("Nativer Dateidialog wird geöffnet …", () => this.api.importEvidence(track.id, role));
    if (imported) {
      this.applyTrack(imported);
      this.showToast("success", "Evidence importiert", `${evidenceRoleLabel(role)} wurde kopiert, gehasht und dem Track zugeordnet.`);
    }
  }

  private async chooseEvidenceRole(): Promise<void> {
    const labels = evidenceRoles.map((role, index) => `${index + 1}: ${evidenceRoleLabel(role)}`).join("\n");
    const choice = window.prompt(`Rolle der Evidence wählen:\n\n${labels}\n\nNummer eingeben:`);
    if (!choice) return;
    const role = evidenceRoles[Number(choice) - 1];
    if (!role) { this.showToast("error", "Ungültige Rolle", "Wähle eine Nummer aus der angezeigten Liste."); return; }
    await this.importEvidence(role);
  }

  private async importGlobalEvidence(): Promise<void> {
    const coverageStart = window.prompt("Beginn des abgedeckten Zeitraums (JJJJ-MM-TT):", new Date().toISOString().slice(0, 8) + "01");
    if (!coverageStart) return;
    const coverageEnd = window.prompt("Ende des abgedeckten Zeitraums (JJJJ-MM-TT):", new Date().toISOString().slice(0, 10));
    if (!coverageEnd) return;
    const item = await this.withBusy("Abo-Nachweis wird registriert …", () => this.api.importGlobalEvidence("subscription_payment", coverageStart, coverageEnd));
    if (item) { this.state.globalEvidence = await this.api.listGlobalEvidence(); this.showToast("success", "Abo-Nachweis registriert", "Er kann nun passenden Track-Zeiträumen zugeordnet werden."); }
  }

  private async addDeviation(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const description = window.prompt("Abweichung sachlich beschreiben:");
    if (!description?.trim()) return;
    const blocking = window.confirm("Soll diese Abweichung die Finalisierung blockieren?");
    await this.trackMutation("Abweichung wird gespeichert …", () => this.api.addDeviation(this.requireTrack().id, description, blocking), "Abweichung gespeichert");
  }

  private async saveTrackDraft(): Promise<void> {
    if (!this.state.trackDraft) return;
    const updated = await this.withBusy("Track-Angaben werden gespeichert …", () => this.api.updateTrack(this.requireTrack().id, this.state.trackDraft!));
    if (updated) { this.applyTrack(updated); this.showToast("success", "Schritt gespeichert", "Der Dokumentationsstatus wurde neu bewertet."); }
  }

  private async flushDraft(): Promise<boolean> {
    if (!this.draftDirty || !this.state.trackDraft || !this.state.track) return true;
    const updated = await this.withBusy("Ungespeicherte Angaben werden zuerst gesichert …", () => this.api.updateTrack(this.state.track!.id, this.state.trackDraft!));
    if (!updated) return false;
    this.applyTrack(updated);
    return true;
  }

  private async trackMutation(label: string, action: () => Promise<TrackDetail>, success: string): Promise<void> {
    if (!(await this.flushDraft())) return;
    const track = await this.withBusy(label, action);
    if (track) { this.applyTrack(track); this.showToast("success", success, "Der Track-Status wurde neu bewertet."); }
  }

  private async runAction(label: string, action: () => Promise<{ message: string; track?: TrackDetail }>): Promise<void> {
    if (!(await this.flushDraft())) return;
    const result = await this.withBusy(label, action);
    if (!result) return;
    if (result.track) this.applyTrack(result.track);
    else await this.refreshTracks();
    this.showToast("success", "Aktion abgeschlossen", result.message);
  }

  private async finalizeTrack(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const validation = await this.withBusy("Finalisierungs-Gate wird nativ geprüft …", () => this.api.validateTrack(track.id));
    if (!validation) return;
    if (!validation.valid) { this.showToast("error", "Finalisierung blockiert", [...validation.missingItems, ...validation.blockingItems].join(" · ")); return; }
    const result = await this.withBusy("Unveränderlicher Snapshot und Zertifikat werden erzeugt …", () => this.api.finalizeTrack(track.id));
    if (result) {
      if (result.track) this.applyTrack(result.track); else await this.refreshTracks();
      this.state.trackTab = "certificate"; this.state.activeStep = null;
      this.showToast("success", "Dokumentation finalisiert", result.message);
    }
  }

  private async generateDocumentsSafely(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const preview = await this.withBusy("Dokumentgenerierung wird sicher vorbereitet …", () => this.api.previewDocumentGeneration(track.id));
    if (!preview) return;
    let adoptExisting = false;
    if (preview.adoptionRequired || preview.collisions.length > 0) {
      const collisionList = preview.collisions.join("\n");
      adoptExisting = window.confirm(`Bestehende verwaltete Dokumente erkannt:\n\n${collisionList}\n\nDie native Anwendung sichert den vorhandenen Zustand unter .archive, bevor neue verwaltete Dokumente geschrieben werden. Fortfahren?`);
      if (!adoptExisting) {
        this.showToast("info", "Dokumentgenerierung abgebrochen", "Bestehende Dateien wurden nicht verändert.");
        return;
      }
    }
    await this.runAction("Dokumente werden atomar erzeugt …", () => this.api.generateDocuments(track.id, adoptExisting));
  }
}
