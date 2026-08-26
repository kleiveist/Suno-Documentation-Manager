import { groupTrackLibrary } from "../domain/track-library";
import type { AlbumTrackGroup } from "../domain/track-library";
import type { StepId, TrackDetail, TrackSummary } from "../domain/types";
import {
  automaticConsistencyPresentation,
  calculateMissingRequirements,
  evaluateRequirements,
  humanEditedFinalArtworkStatus,
  statusLabel,
  stepStatuses,
  WORKFLOW_STEPS
} from "../domain/workflow";
import { escapeHtml, formatDate, titleInitials } from "../ui/format";
import { icon } from "../ui/icons";
import { AppWorkspaceShell } from "./class-workspace-shell";
import { missingProfileFields, workflowUpgradePresentation } from "./navigation";
import { finalizedTrackPresentation, isTrackContentLocked } from "./state";
import type { TrackTab } from "./state";
import { trackCheckSummary } from "./track-presentation";

export abstract class AppLibraryRender extends AppWorkspaceShell {
  protected renderDashboard(): string {
    const active = this.state.tracks.filter((track) => track.status === "ACTIVE" || track.status === "DRAFT").length;
    const ready = this.state.tracks.filter((track) => track.status === "READY").length;
    const finalized = this.state.tracks.filter(
      (track) => track.status === "FINALIZED" && track.certificateValid !== false
    ).length;
    const recent = [...this.state.tracks].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 4);
    const next = recent.find((track) => track.status !== "FINALIZED" || track.certificateValid === false) ?? recent[0];
    const nextDescription = next
      ? this.language === "en"
        ? `There are still ${next.missingCount} required items open for <strong>${escapeHtml(next.title)}</strong>.`
        : `Bei <strong>${escapeHtml(next.title)}</strong> sind noch ${next.missingCount} Pflichtpunkte offen.`
      : this.language === "en"
        ? "Create your first track to start the workflow."
        : "Lege deinen ersten Track an, um den Workflow zu starten.";
    return `<div class="page-content dashboard-page">
        <section class="dashboard-welcome">
          <div><p class="overline">Lokaler Workspace</p><h2>Guten Tag, ${escapeHtml(this.state.profile.artistName || "Artist")}.</h2><p>${nextDescription}</p></div>
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
            ${
              next
                ? `<div class="attention-track">${this.renderTrackCover(next, "track-cover--large", "div")}<div><span class="status-chip status-chip--${next.status.toLowerCase()}">${statusLabel(next.status)}</span><h4>${escapeHtml(next.title)}</h4><p>${next.missingCount > 0 ? `${next.missingCount} erforderliche Angaben oder Nachweise fehlen noch.` : "Alle Pflichtpunkte sind erfüllt."}</p></div></div>
            <div class="progress-block"><div><span>Dokumentationsfortschritt</span><strong>${next.progress}%</strong></div><progress class="progress-track" max="100" value="${next.progress}" aria-label="Dokumentationsfortschritt ${next.progress} Prozent"></progress></div>
            <button class="button button--dark button--wide" data-track-open="${escapeHtml(next.id)}">Dokumentation fortsetzen ${icon("arrow")}</button>`
                : `<p class="muted">Keine offenen Tracks.</p>`
            }
          </aside>
        </div>
        <section class="principle-strip"><div class="principle-icon">${icon("shield")}</div><div><strong>Show what is missing.</strong><span>Ask only what is necessary.</span></div><p>Die App speichert ausschließlich lokal und erzeugt portable Track-Ordner, die ohne diese App verständlich bleiben.</p></section>
      </div>`;
  }

  protected metricCard(
    iconName: "tracks" | "current" | "check" | "certificate",
    label: string,
    value: number,
    note: string,
    color: string
  ): string {
    return `<article class="metric-card"><span class="metric-icon metric-icon--${color}">${icon(iconName)}</span><div><span>${label}</span><strong>${value.toLocaleString(this.numberLocale)}</strong><small>${note}</small></div></article>`;
  }

  protected renderTracks(): string {
    const library = groupTrackLibrary(
      this.state.tracks,
      {
        query: this.state.query,
        status: this.state.trackFilter
      },
      this.state.albums
    );
    const albumTrackCount = library.albums.reduce((total, album) => total + album.tracks.length, 0);
    return `<div class="page-content tracks-page">
        <div class="page-lead"><div><p class="overline">Bibliothek</p><h2>Alben & Singles</h2><p>Die Ansicht entspricht der echten Ordnerstruktur im Workspace. Albumordner können direkt angelegt und umbenannt werden.</p></div><button class="button button--primary" data-action="new-track">${icon("plus")} Neuer Track</button></div>
        <section class="panel tracks-panel">
          <div class="tracks-toolbar">
            <label class="search-field"><span class="sr-only">Tracks und Alben durchsuchen</span>${icon("scan")}<input type="search" data-track-search placeholder="Tracks und Alben durchsuchen …" value="${escapeHtml(this.state.query)}"></label>
            <div class="filter-tabs" role="group" aria-label="Statusfilter">
              ${(
                [
                  ["all", "Alle"],
                  ["open", "Offen"],
                  ["ready", "Bereit"],
                  ["finalized", "Finalisiert"]
                ] as const
              )
                .map(
                  ([id, label]) =>
                    `<button class="${this.state.trackFilter === id ? "is-active" : ""}" data-track-filter="${id}">${label}</button>`
                )
                .join("")}
            </div>
            <button class="button button--secondary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button>
          </div>
          <div class="track-library-content">
            <details class="library-section" aria-labelledby="albums-library-title" open>
              <summary class="library-section-head"><span class="library-section-icon">${icon("workspace")}</span><span class="library-section-copy"><strong class="library-section-title" id="albums-library-title" role="heading" aria-level="3">Alben</strong><small>${library.albums.length} ${library.albums.length === 1 ? "Album" : "Alben"} · ${albumTrackCount} ${albumTrackCount === 1 ? "Track" : "Tracks"}</small></span><button type="button" class="album-create-button" data-action="create-album" aria-label="Neuen Albumordner anlegen" title="Neuen Albumordner anlegen">${icon("plus")} Album anlegen</button><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
              <div class="library-section-content">${
                library.albums.length
                  ? `<div class="album-group-list">${library.albums.map((album) => this.renderAlbumGroup(album)).join("")}</div>`
                  : this.renderLibraryEmpty("Noch keine Alben", "Lege hier zuerst einen Albumordner an.")
              }</div>
            </details>
            <details class="library-section" aria-labelledby="singles-library-title" open>
              <summary class="library-section-head"><span class="library-section-icon library-section-icon--single">${icon("tracks")}</span><span class="library-section-copy"><strong class="library-section-title" id="singles-library-title" role="heading" aria-level="3">Singles</strong><small>${library.singles.length} ${library.singles.length === 1 ? "Track" : "Tracks"}</small></span><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
              <div class="library-section-content">${
                library.singles.length
                  ? `${this.renderTrackTableHead()}<div class="track-list">${library.singles.map((track) => this.renderTrackRow(track, true)).join("")}</div>`
                  : this.renderLibraryEmpty(
                      "Keine passenden Singles",
                      "Lege eine Single an oder passe Suche und Statusfilter an."
                    )
              }</div>
            </details>
          </div>
        </section>
      </div>`;
  }

  protected renderAlbumGroup(album: AlbumTrackGroup<TrackSummary>): string {
    return `<details class="album-group" open>
        <summary class="album-group-head"><span class="album-cover" aria-hidden="true">${escapeHtml(titleInitials(album.title))}<i></i></span><span class="album-group-copy"><strong>${escapeHtml(album.title)}</strong><small>${album.tracks.length} ${album.tracks.length === 1 ? "Track" : "Tracks"}</small></span><button type="button" class="album-rename-button" data-rename-album="${escapeHtml(album.title)}" aria-label="Album ${escapeHtml(album.title)} umbenennen" title="Albumordner umbenennen">Umbenennen</button><span class="library-disclosure-icon">${icon("chevronDown")}</span></summary>
        <div class="album-group-content">${
          album.tracks.length
            ? `${this.renderTrackTableHead()}<div class="track-list">${album.tracks.map((track) => this.renderTrackRow(track, true)).join("")}</div>`
            : this.renderLibraryEmpty("Album ist noch leer", "Lege einen Track an und ordne ihn diesem Album zu.")
        }</div>
      </details>`;
  }

  protected renderTrackTableHead(): string {
    return `<div class="track-table-head"><span>Track</span><span>Status</span><span>Fortschritt</span><span>Aktualisiert</span><span></span></div>`;
  }

  protected renderLibraryEmpty(title: string, copy: string): string {
    return `<div class="library-empty">${icon("tracks")}<div><strong>${escapeHtml(title)}</strong><span>${escapeHtml(copy)}</span></div></div>`;
  }

  protected renderTrackRow(track: TrackSummary, detailed = false): string {
    return `<button class="track-row ${detailed ? "track-row--detailed" : ""}" data-track-open="${escapeHtml(track.id)}">
        ${this.renderTrackCover(track)}
        <span class="track-identity"><strong>${escapeHtml(track.title)}</strong><small>${escapeHtml(track.relativePath)}${track.legacy ? (this.language === "en" ? " · Legacy import" : " · Legacy-Import") : ""}</small></span>
        <span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span>
        <span class="row-progress"><progress max="100" value="${track.progress}" aria-label="${track.progress} Prozent"></progress><b>${track.progress}%</b></span>
        ${detailed ? `<time>${formatDate(track.updatedAt, false, this.language)}</time>` : `<span class="missing-hint">${track.missingCount ? `${track.missingCount} offen` : "Vollständig"}</span>`}
        <span class="row-arrow">${icon("arrow")}</span>
      </button>`;
  }

  protected renderTrack(): string {
    const track = this.state.track;
    if (!track)
      return this.emptyState(
        "current",
        "Kein Track ausgewählt",
        "Wähle einen Track aus deiner Bibliothek.",
        "go-tracks",
        "Tracks öffnen"
      );
    const tabs: Array<[TrackTab, string]> = [
      ["overview", "Übersicht"],
      ["suno", "Suno"],
      ["artwork", "Artwork"],
      ["release", "Release"],
      ["evidence", "Evidence"],
      ["certificate", "Zertifikat"]
    ];
    const workflowUpgrade = workflowUpgradePresentation(track, this.state.workflow);
    const libraryLabel =
      track.library.section === "album" && track.library.albumTitle ? `Album · ${track.library.albumTitle}` : "Single";
    return `<div class="track-page">
        <section class="track-hero">
          ${this.renderTrackCover(track, "track-cover--hero", "div")}
          <div class="track-hero-copy"><div><span class="status-chip status-chip--${track.status.toLowerCase()}">${statusLabel(track.status)}</span><span class="workflow-version">Workflow ${escapeHtml(track.workflowVersion)}</span><button class="library-chip" data-action="edit-track-library" title="${isTrackContentLocked(track.status) ? "Historischer Snapshot – Bibliothekszuordnung ist schreibgeschützt" : "Bibliothekszuordnung ändern"}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>${icon(track.library.section === "album" ? "workspace" : "tracks")} ${escapeHtml(libraryLabel)}</button></div><h2>${escapeHtml(track.title)}</h2><p>${escapeHtml(track.relativePath)}</p></div>
          <div class="hero-progress"><strong>${track.progress}%</strong><span>dokumentiert</span><progress class="progress-track" max="100" value="${track.progress}" aria-label="Dokumentationsfortschritt ${track.progress} Prozent"></progress></div>
        </section>
        <nav class="track-tabs" aria-label="Track-Ansichten">${tabs.map(([id, label]) => `<button class="${this.state.trackTab === id ? "is-active" : ""}" data-track-tab="${id}">${label}</button>`).join("")}</nav>
        <div class="track-content">${workflowUpgrade ? `<div class="policy-card">${icon("info")}<div><p class="overline">Workflow-Upgrade verfügbar</p><h4>${escapeHtml(this.systemText(workflowUpgrade.message))}</h4><p>Der bisherige Zertifikatssnapshot bleibt unverändert. Die Neubewertung verlangt aktuelle Dokumente, Prüfsummen und ein neues Zertifikat.</p></div>${workflowUpgrade.action ? `<button class="button button--secondary" data-action="${workflowUpgrade.action}">Mit aktuellem Workflow neu bewerten</button>` : ""}</div>` : ""}${this.renderTrackTab(track)}</div>
      </div>`;
  }

  protected renderTrackTab(track: TrackDetail): string {
    if (this.state.activeStep) return this.renderWorkflowEditor(track, this.state.activeStep);
    switch (this.state.trackTab) {
      case "overview":
        return this.renderTrackOverview(track);
      case "suno":
        return this.renderWorkflowEditor(track, "suno");
      case "artwork":
        return this.renderWorkflowEditor(track, "artwork");
      case "release":
        return this.renderWorkflowEditor(track, "release");
      case "evidence":
        return `${this.renderFinalizedSnapshotNotice(track)}${this.renderEvidence(track)}`;
      case "certificate":
        return this.renderCertificate(track);
    }
  }

  protected renderTrackOverview(track: TrackDetail): string {
    const missing = calculateMissingRequirements(track, track.profileSnapshot);
    const evaluatedStatuses = stepStatuses(track, track.profileSnapshot);
    const statuses = this.runtimeSteps().map(
      (step) => evaluatedStatuses.find((state) => state.id === step.id) ?? { id: step.id, status: "NOT_RUN" as const }
    );
    return `<div class="workflow-layout">
        <section class="workflow-main">
          ${this.renderFinalizedSnapshotNotice(track)}
          ${!isTrackContentLocked(track.status) && track.legacy && missingProfileFields(track.profileSnapshot).length ? `<div class="policy-card">${icon("info")}<div><p class="overline">Legacy-Track</p><h4>Historische Stammdaten ausdrücklich bestätigen</h4><p>Der Scan hat keine fehlenden Fakten erfunden. Übernimm die aktuellen Workspace-Stammdaten nur, wenn sie für diesen Track tatsächlich zutreffen; danach kannst du weitere Angaben prüfen und speichern.</p></div><button class="button button--secondary" data-action="adopt-legacy-profile">Stammdaten als Snapshot bestätigen</button></div>` : ""}
          ${this.renderTrackCheckSummary(track)}
          <div class="panel missing-panel ${missing.length === 0 ? "is-complete" : ""}">
            <div class="panel-heading"><div><p class="overline">Finalisierungs-Gate</p><h3>${missing.length ? "Was fehlt noch?" : "Bereit zur Finalisierung"}</h3></div><span class="missing-total">${missing.length}</span></div>
            ${
              missing.length
                ? `<ul class="missing-list">${missing
                    .slice(0, 8)
                    .map(
                      (item) =>
                        `<li><span>${icon("alert")}</span><div><strong>${escapeHtml(item.label)}</strong><small>${escapeHtml(this.runtimeSteps().find((step) => step.id === item.stepId)?.title ?? item.stepId)}</small></div><button data-step-open="${item.stepId}">${item.evidenceRole ? "Nachweis" : "Öffnen"} ${icon("arrow")}</button></li>`
                    )
                    .join(
                      ""
                    )}</ul>${missing.length > 8 ? `<button class="text-button" data-track-tab="evidence">${this.t(`Alle ${missing.length} offenen Punkte anzeigen`)}</button>` : ""}`
                : `<div class="success-message">${icon("check")}<div><strong>Alle lokalen Vorprüfungen sind erfüllt.</strong><span>Rust validiert den Track vor der Finalisierung nochmals vollständig.</span></div></div>`
            }
          </div>
          <div class="panel workflow-panel"><div class="panel-heading"><div><p class="overline">Geführter Ablauf</p><h3>10 Dokumentationsschritte</h3></div><span class="workflow-id">${escapeHtml(this.state.workflow?.id ?? track.workflowId)} · v${escapeHtml(this.state.workflow?.version ?? track.workflowVersion)}</span></div>
            <div class="step-list">${statuses.map((step, index) => this.renderStepRow(step.id, step.status, index)).join("")}</div>
            <p class="workflow-pass-definition"><strong>PASS / Erfüllt:</strong> Configured documentation requirements for this step were satisfied.</p>
          </div>
        </section>
        <aside class="workflow-side">
          <div class="panel quick-actions"><p class="overline">Schnellaktionen</p><h3>Dokumentsatz</h3>
            ${this.actionRow("file", "Dokumente", track.documents.current ? "Aktuell" : track.documents.generated ? "Veraltet" : "Nicht erzeugt", "generate-documents", isTrackContentLocked(track.status))}
            ${this.actionRow("hash", "SHA-256", track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Nicht erzeugt", "calculate-hashes", isTrackContentLocked(track.status))}
            ${this.actionRow("shield", "Verifikation", track.integrity.verified ? "Bestanden" : "Offen", "verify-hashes")}
          </div>
          <div class="panel snapshot-card"><p class="overline">Track-Snapshot</p><dl><div><dt>Evidence</dt><dd>${track.evidence.length}</dd></div><div><dt>Dokumente</dt><dd>${track.documents.files.length}</dd></div><div><dt>Verifiziert</dt><dd>${track.integrity.verifiedCount}/${track.integrity.fileCount}</dd></div><div><dt>Abweichungen</dt><dd>${track.blockingDeviations?.filter((item) => item.blocking && !item.resolved).length ?? track.integrity.mismatchFiles.length}</dd></div></dl></div>
        </aside>
      </div>`;
  }

  protected renderTrackCheckSummary(track: TrackDetail): string {
    const summary = trackCheckSummary(track);
    const consistency = automaticConsistencyPresentation(track);
    const findings = consistency.findings;
    const technicalFacts = [
      track.automation.sunoCreatedTimestamp
        ? `<li><strong>Suno-created</strong><span>${escapeHtml(track.automation.sunoCreatedTimestamp)}</span></li>`
        : "",
      track.automation.sunoId
        ? `<li><strong>Technische Suno-ID</strong><span>${escapeHtml(track.automation.sunoId)}</span></li>`
        : "",
      `<li><strong>Release identisch zum Suno-Export</strong><span>${track.automation.releaseIdenticalToSunoExport ? "PASS" : "Nicht nachgewiesen"}</span></li>`,
      `<li><strong>Byte-identische Paare</strong><span>${track.automation.byteIdenticalPairs.length}</span></li>`,
      `<li><strong>Menschlich bearbeitetes/finales Artwork</strong><span>${escapeHtml(humanEditedFinalArtworkStatus(track.evidence))}</span></li>`
    ]
      .filter(Boolean)
      .join("");
    return `<section class="panel check-summary">
        <div class="panel-heading"><div><p class="overline">Gesamtprüfung</p><h3>Dokumentationsstatus</h3></div><span class="check-warning-count ${consistency.outcome === "PASS" ? "is-clear" : ""}">${escapeHtml(this.t(`Automatische Konsistenz: ${this.t(consistency.outcome)} · ${consistency.warningCount} WARNING · ${consistency.infoCount} INFO`))}</span></div>
        <div class="check-summary-grid">
          ${this.checkSummaryItem("Dokumentation", summary.documentation, summary.documentation === "vollständig")}
          ${this.checkSummaryItem("Dateiintegrität", summary.fileIntegrity, summary.fileIntegrity === "geprüft")}
          ${this.checkSummaryItem("Suno-Metadaten", summary.sunoMetadata, summary.sunoMetadata === "erkannt", summary.sunoMetadata === "nicht erkannt")}
          ${this.checkSummaryItem("Subscription-Zeitraum", summary.subscriptionCoverage, summary.subscriptionCoverage === "passend" || summary.subscriptionCoverage === "nicht erforderlich", summary.subscriptionCoverage === "nicht geprüft")}
        </div>
        <details class="check-details"><summary>Technische Details anzeigen</summary><ul>${technicalFacts}</ul>${findings.length ? `<div class="check-issues"><strong>Automatische Konsistenz</strong><ul>${findings.map((finding) => `<li>${icon(finding.level === "INFO" ? "info" : "alert")}<span><strong>${escapeHtml(finding.level)}</strong> · ${escapeHtml(finding.userProvided ? finding.message : this.systemText(finding.message))}</span></li>`).join("")}</ul></div>` : `<p class="check-details-clear">Keine Konsistenzabweichungen erkannt.</p>`}</details>
      </section>`;
  }

  protected checkSummaryItem(label: string, value: string, passed: boolean, neutral = false): string {
    return `<div class="check-summary-item ${passed ? "is-pass" : neutral ? "is-neutral" : "is-open"}">${icon(passed ? "check" : neutral ? "info" : "alert")}<span><strong>${escapeHtml(label)}</strong><small>${escapeHtml(value)}</small></span></div>`;
  }

  protected renderStepRow(stepId: StepId, status: string, index: number): string {
    const nativeStep = this.state.workflow?.steps.find((item) => item.id === stepId);
    const fallback = WORKFLOW_STEPS.find((item) => item.id === stepId)!;
    const number = nativeStep?.number ?? fallback.number;
    const label = nativeStep?.title ?? nativeStep?.label ?? fallback.title;
    const description = nativeStep?.description ?? fallback.description;
    return `<button class="step-row" data-step-open="${stepId}"><span class="step-number">${escapeHtml(number)}</span><span class="step-state step-state--${status.toLowerCase().replace("_", "-")}">${status === "PASS" || status === "N_A" ? icon("check") : status === "FAIL" || status === "BLOCKED" ? icon("alert") : index + 1}</span><span class="step-copy"><strong>${escapeHtml(label)}</strong><small>${escapeHtml(description)}</small></span><span class="step-status">${statusLabel(status as Parameters<typeof statusLabel>[0])}</span>${icon("arrow")}</button>`;
  }

  protected runtimeSteps(): Array<{ id: StepId; number: string; title: string; description: string }> {
    if (this.state.workflow) {
      return this.state.workflow.steps.map((step) => ({
        id: step.id,
        number: step.number,
        title: step.title ?? step.label,
        description: step.description
      }));
    }
    // Explicit browser-demo/test fallback. Packaged Tauri builds use get_workflow().
    return WORKFLOW_STEPS.map((step) => ({
      id: step.id,
      number: step.number,
      title: step.title,
      description: step.description
    }));
  }

  protected actionRow(
    iconName: "file" | "hash" | "shield",
    label: string,
    state: string,
    action: string,
    disabled = false
  ): string {
    return `<button class="action-row" data-action="${action}" ${disabled ? "disabled" : ""}><span>${icon(iconName)}</span><div><strong>${label}</strong><small>${state}</small></div>${icon("arrow")}</button>`;
  }

  protected renderFinalizedSnapshotNotice(track: TrackDetail): string {
    const presentation = finalizedTrackPresentation(track);
    if (!presentation) return "";
    return `<div class="policy-card finalized-snapshot-notice ${presentation.invalid ? "is-invalid" : ""}">${icon(presentation.invalid ? "alert" : "lock")}<div><p class="overline">Revisionsschutz</p><h4>${escapeHtml(this.systemText(presentation.title))}</h4><p>${escapeHtml(this.systemText(presentation.message))}</p></div>${presentation.actionLabel ? `<button class="button ${presentation.invalid ? "button--danger" : "button--primary"}" data-action="create-revision">${icon("current")} ${escapeHtml(this.systemText(presentation.actionLabel))}</button>` : ""}</div>`;
  }

  protected renderWorkflowEditor(track: TrackDetail, stepId: StepId): string {
    const runtimeSteps = this.runtimeSteps();
    const index = runtimeSteps.findIndex((step) => step.id === stepId);
    const definition = this.state.workflow?.steps.find((step) => step.id === stepId);
    const fallback = runtimeSteps[index];
    const label = definition?.title ?? definition?.label ?? fallback.title;
    const description = definition?.description ?? fallback.description;
    const statuses = stepStatuses(track, track.profileSnapshot);
    const currentStatus = statuses.find((step) => step.id === stepId)?.status ?? "NOT_RUN";
    const naEligible = !evaluateRequirements(track, track.profileSnapshot).some(
      (requirement) => requirement.stepId === stepId
    );
    return `<div class="editor-layout">
        <aside class="workflow-rail">
          <button class="rail-back" data-action="back-overview">${icon("arrow")} Track-Übersicht</button>
          <div class="rail-progress"><span>${index + 1} / 10</span><progress max="10" value="${index + 1}" aria-label="${this.t(`Schritt ${index + 1} von 10`)}"></progress></div>
          <nav aria-label="Workflow-Schritte">${runtimeSteps
            .map((runtimeStep) => {
              const step = statuses.find((entry) => entry.id === runtimeStep.id) ?? {
                id: runtimeStep.id,
                status: "NOT_RUN" as const
              };
              const item = this.state.workflow?.steps.find((entry) => entry.id === step.id);
              const itemFallback = runtimeStep;
              return `<button class="rail-step ${step.id === stepId ? "is-active" : ""} ${step.status === "PASS" || step.status === "N_A" ? "is-complete" : ""}" data-step-open="${step.id}"><span>${step.status === "PASS" || step.status === "N_A" ? icon("check") : (item?.number ?? itemFallback.number)}</span><strong>${escapeHtml(item?.title ?? item?.label ?? itemFallback.title)}</strong></button>`;
            })
            .join("")}</nav>
        </aside>
        <section class="editor-main">
          <header class="editor-head"><div><p class="overline">${this.t(`Schritt ${definition?.number ?? fallback.number}`)}</p><h3>${escapeHtml(label)}</h3><p>${escapeHtml(description)}</p></div><span class="status-chip step-status-chip step-status-chip--${currentStatus.toLowerCase().replace("_", "-")}">${statusLabel(currentStatus)}</span></header>
          ${this.renderFinalizedSnapshotNotice(track)}
          ${naEligible ? `<div class="na-control">${icon("info")}<div><strong>Dieser Schritt hat für den aktuellen Track keine anwendbaren Pflichtpunkte.</strong><span>N/A wird nur mit einer gespeicherten sachlichen Begründung akzeptiert.</span></div>${currentStatus === "N_A" ? `<button class="button button--secondary" data-reset-na="${stepId}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>N/A zurücksetzen</button>` : `<button class="button button--secondary" data-mark-na="${stepId}" ${isTrackContentLocked(track.status) ? "disabled" : ""}>Als N/A dokumentieren</button>`}</div>` : ""}
          ${this.renderStepContent(track, stepId)}
          <footer class="editor-footer">
            <button class="button button--secondary" ${index === 0 ? "disabled" : ""} data-step-open="${runtimeSteps[Math.max(index - 1, 0)].id}">${icon("arrow")} Zurück</button>
            ${index < runtimeSteps.length - 1 ? `<button class="button button--dark" data-step-open="${runtimeSteps[index + 1].id}">${escapeHtml(this.t(`Weiter: ${this.t(runtimeSteps[index + 1].title)}`))} ${icon("arrow")}</button>` : ""}
          </footer>
        </section>
      </div>`;
  }
}
