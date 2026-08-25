import type {
  AudioScreeningProviderTestResult,
  AudioScreeningSecretInput,
  AudioScreeningSettings,
  TimestampAuthenticationMode,
  TimestampProviderKind,
  TimestampProviderTestResult,
  TimestampSettings,
  TrackDetail
} from "../domain/types";
import { automaticConsistencyPresentation, finalizationGate } from "../domain/workflow";
import { escapeHtml, formatDate } from "../ui/format";
import { translateRenderedUi } from "../ui/i18n";
import { icon } from "../ui/icons";
import {
  ACRCLOUD_MAX_REQUESTS,
  audioScreeningIntensityBand,
  audioScreeningIntensityEstimate,
  audioScreeningProviderStatusLabel,
  externalAudioScreeningIsCurrent,
  formatScreeningSeconds,
  localAudioScreeningIsCurrent
} from "./audio-screening";
import { aiSystemSuggestions, sunoPlanSuggestions } from "./choices";
import { AppWorkflowRender } from "./class-workflow-render";
import {
  SETTINGS_CATEGORY_DEFINITIONS,
  settingsCategoryNavigationMarkup,
  workflowUpgradeFinalizationBlocker
} from "./navigation";
import { canCreateTrackRevision, isTrackContentLocked } from "./state";
import { renderExternalTimestampSection } from "./timestamp-section";
import {
  termsMetadataComplete,
  timestampProviderProtocolPresentation,
  timestampProviderStatusLabel
} from "./timestamp-presentation";

export abstract class AppSettingsRender extends AppWorkflowRender {
  protected renderExternalTimestampSection(track: TrackDetail): string {
    if (!isTrackContentLocked(track.status)) return "";
    return renderExternalTimestampSection(track, {
      settings: this.state.timestampSettings,
      language: this.language,
      translate: (value) => this.t(value),
      systemText: (value) => this.systemText(value)
    });
  }

  protected finalizationHeading(track: TrackDetail, workflowBlocker: string | null, localGateValid: boolean): string {
    if (track.status === "SUPERSEDED") return "Ersetzter Snapshot – nur lesbar";
    if (track.status === "FINALIZED") {
      return track.certificate.valid ? "Dokumentation finalisiert" : "Finalisierter Snapshot nicht mehr gültig";
    }
    if (workflowBlocker) return "Workflow-Neubewertung erforderlich";
    return localGateValid ? "Bereit für den Abschluss" : "Finalisierung noch blockiert";
  }

  protected finalizationSummary(
    track: TrackDetail,
    workflowBlocker: string | null,
    localGateValid: boolean,
    blockerCount: number
  ): string {
    if (track.status === "SUPERSEDED") {
      return "Dieser historische Snapshot wurde durch eine neuere Revision ersetzt und bleibt unverändert. Navigation und Integritätsprüfungen sind weiterhin möglich.";
    }
    if (track.status === "FINALIZED") {
      return track.certificate.valid
        ? "Dieser Snapshot ist abgeschlossen und schreibgeschützt. Verwende die Revisionsaktion oben, um eine bearbeitbare Folgeversion anzulegen."
        : "Das bisherige Zertifikat ist ungültig. Verwende die Revisionsaktion oben, um die Abweichung in einer neuen Folgeversion zu bearbeiten.";
    }
    if (workflowBlocker) {
      return "Wähle zuerst die explizite Neubewertung mit dem aktuellen Workflow. Danach müssen Dokumente und Prüfsummen erneut erzeugt werden.";
    }
    return localGateValid
      ? "Die UI-Vorprüfung ist vollständig. Der native Dienst validiert vor dem Erzeugen des Zertifikats nochmals alle Pflichtschritte, Evidence und Hashes."
      : `${blockerCount} Punkte müssen vor dem Abschluss geklärt werden.`;
  }

  protected renderFinalizationGateList(blockers: string[]): string {
    if (blockers.length) {
      return `<ul class="gate-list">${blockers.map((item) => `<li>${icon("alert")}<span>${escapeHtml(item)}</span></li>`).join("")}</ul>`;
    }
    return `<ul class="gate-list gate-list--success"><li>${icon("check")}<span>Pflichtschritte erfüllt</span></li><li>${icon("check")}<span>Evidence vollständig</span></li><li>${icon("check")}<span>Dokumente aktuell</span></li><li>${icon("check")}<span>SHA-256 vollständig verifiziert</span></li></ul>`;
  }

  protected renderFinalizationLanguageNotice(track: TrackDetail, locked: boolean): string {
    if (!locked) {
      return `<div class="technical-note">${icon("info")}<p><strong>Standardausgabe:</strong> Bei der Finalisierung werden automatisch ein deutsches und ein englisches PDF erzeugt.</p></div>`;
    }
    if (!track.certificate.valid) return "";
    const language = track.certificate.certificateLanguage === "de" ? "Deutsch" : "Englisch";
    const languages = track.certificate.bilingual ? "Deutsch und Englisch" : language;
    const fileCount = track.certificate.bilingual ? " (zwei PDF-Dateien)" : "";
    return `<div class="technical-note">${icon("info")}<p><strong>Zertifikatsausgabe:</strong> ${languages}${fileCount}.</p></div>`;
  }

  protected renderFinalizationAction(track: TrackDetail, locked: boolean, ready: boolean): string {
    if (!locked) {
      return `<button class="button button--finalize" data-action="finalize-track" ${!ready ? "disabled" : ""}>${icon("certificate")} Dokumentation finalisieren</button>`;
    }
    if (!track.certificate.valid || !track.certificate.certificateId) return "";
    return `<button class="button button--finalize" data-action="show-certificate-popup">${icon("certificate")} Zertifikat anzeigen</button>`;
  }

  protected renderFinalization(track: TrackDetail): string {
    const localGate = finalizationGate(track, track.profileSnapshot);
    const workflowBlocker = workflowUpgradeFinalizationBlocker(track, this.state.workflow);
    const blockers = [
      ...localGate.missingItems,
      ...localGate.blockingItems,
      ...(workflowBlocker ? [workflowBlocker] : [])
    ];
    const locked = isTrackContentLocked(track.status);
    const ready = localGate.valid && !workflowBlocker;
    const heading = this.finalizationHeading(track, workflowBlocker, localGate.valid);
    const summary = this.finalizationSummary(track, workflowBlocker, localGate.valid, blockers.length);
    return `<div class="finalize-page">
        <div class="finalize-mark ${ready && !locked ? "is-ready" : ""}">${icon(ready && !locked ? "certificate" : "lock")}</div>
        <p class="overline">Track Documentation Completion Certificate</p><h3>${heading}</h3><p>${summary}</p>
        ${this.renderFinalizationGateList(blockers)}
        ${this.renderFinalizationLanguageNotice(track, locked)}
        ${this.renderFinalizationAction(track, locked, ready)}
        <div class="technical-note">${icon("info")}<p><strong>PASS:</strong> ${this.t("Configured documentation requirements for this step were satisfied.")}<br><strong>DOCUMENTATION COMPLETE:</strong> ${this.t("Meaning: configured documentation requirements completed.")}</p></div>
        <p class="certificate-disclaimer">Das Zertifikat bestätigt ausschließlich den Abschluss des konfigurierten Dokumentations- und Integritätsworkflows. Es ist keine behördliche Zertifizierung, Rechtsberatung oder unabhängige Feststellung von Urheberschaft oder Rechtskonformität.</p>
        ${this.renderExternalTimestampSection(track)}
      </div>`;
  }

  protected renderCertificate(track: TrackDetail): string {
    if (!track.certificate.certificateId)
      return `<div class="certificate-empty">${icon("certificate")}<p class="overline">Zertifikat</p><h3>Noch kein Completion Certificate</h3><p>Das Zertifikat wird erst nach erfolgreicher nativer Finalisierungsprüfung erzeugt.</p>${canCreateTrackRevision(track.status) ? `<button class="button button--primary" data-action="create-revision">${icon("current")} Neue Revision anlegen und bearbeiten</button>` : isTrackContentLocked(track.status) ? "" : `<button class="button button--dark" data-step-open="finalize">Finalisierungs-Gate öffnen ${icon("arrow")}</button>`}</div>`;
    const deviations = (track.blockingDeviations ?? []).filter((item) => item.blocking && !item.resolved);
    const consistency = automaticConsistencyPresentation(track);
    return `<div class="certificate-view ${track.certificate.valid ? "is-valid" : "is-invalid"}">
        <div class="certificate-paper"><header><div class="certificate-seal">${icon("certificate")}</div><div><p>Suno Documentation Manager</p><h3>Track Documentation<br>Completion Certificate</h3></div><span class="certificate-result">${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "CERTIFICATE INVALID"}</span></header>
        <div class="certificate-rule"></div><dl><div><dt>Certificate ID</dt><dd>${escapeHtml(track.certificate.certificateId)}</dd></div><div><dt>Track</dt><dd>${escapeHtml(track.title)}</dd></div><div><dt>Artist</dt><dd>${escapeHtml(track.profileSnapshot.artistName)}</dd></div><div><dt>Workflow</dt><dd>${escapeHtml(track.workflowId)} · ${escapeHtml(track.certificate.workflowVersion ?? track.workflowVersion)}</dd></div><div><dt>Finalisierung</dt><dd>${formatDate(track.certificate.finalizedAt, true, this.language)}</dd></div><div><dt>Evidence-Dateien</dt><dd>${track.evidence.length}</dd></div><div><dt>Blockierende Abweichungen</dt><dd>${deviations.length}</dd></div><div><dt>Automatische Konsistenz</dt><dd>${escapeHtml(consistency.outcome)}</dd></div><div><dt>Finales Ergebnis</dt><dd>${track.certificate.valid ? "DOCUMENTATION COMPLETE" : "INVALID"}</dd></div><div><dt>${this.t("Meaning")}</dt><dd>${this.t("configured documentation requirements completed")}</dd></div></dl>
        <footer>PASS means: Configured documentation requirements for this step were satisfied. This certificate confirms completion of the configured documentation workflow and integrity checks. It does not constitute governmental certification, legal advice, or an independent determination of copyright ownership or legal compliance.</footer></div>
        <div class="certificate-actions">${track.certificate.valid ? `<button class="button button--secondary" data-action="show-certificate-popup">${icon("certificate")} Zertifikatsübersicht öffnen</button>` : ""}${canCreateTrackRevision(track.status) ? `<button class="button button--primary" data-action="create-revision">${icon("current")} Neue Revision anlegen und bearbeiten</button>` : ""}${track.certificate.valid && canCreateTrackRevision(track.status) ? `<button class="button button--danger-soft" data-action="invalidate-certificate">Zertifikat invalidieren</button>` : ""}</div>
        ${this.renderExternalTimestampSection(track)}
      </div>`;
  }

  protected renderWorkspace(): string {
    const scan = this.state.scanResult;
    return `<div class="page-content workspace-page">
        <div class="page-lead"><div><p class="overline">Lokaler Projektordner</p><h2>${escapeHtml(this.state.workspace?.name)}</h2><p class="path-display">${icon("workspace")} ${escapeHtml(this.state.workspace?.path)}</p></div><button class="button button--primary" data-action="scan-workspace">${icon("scan")} Workspace scannen</button></div>
        <section class="workspace-stats"><article><span>${icon("tracks")}</span><div><strong>${this.state.tracks.length}</strong><small>indexierte Tracks</small></div></article><article><span>${icon("scan")}</span><div><strong>${formatDate(this.state.workspace?.lastScannedAt, false, this.language)}</strong><small>zuletzt gescannt</small></div></article><article><span>${icon("shield")}</span><div><strong>Lokal</strong><small>SQLite + Track-Ordner</small></div></article></section>
        <section class="panel workspace-scan"><div class="panel-heading"><div><p class="overline">Bestehende Projekte</p><h3>Legacy-Track-Import</h3><p>Der Scan erkennt bekannte Ordner, Evidence und Hashlisten. Bestehende Dateien werden dabei niemals verändert.</p></div></div>
          ${scan ? `<div class="scan-summary"><span><strong>${scan.discovered}</strong> erkannt</span><span><strong>${scan.indexed}</strong> indexiert</span><span><strong>${scan.unchanged}</strong> unverändert</span></div>${scan.warnings.length ? `<ul class="warning-list">${scan.warnings.map((warning) => `<li>${icon("alert")} ${escapeHtml(this.systemText(warning))}</li>`).join("")}</ul>` : ""}${scan.candidates?.length ? `<div class="candidate-list">${scan.candidates.map((candidate) => `<article><span class="file-icon">${icon("workspace")}</span><div><strong>${escapeHtml(candidate.name)}</strong><small>${escapeHtml(candidate.relativePath)}</small><p>${candidate.missingItems.length ? `${this.t("Fehlt:")} ${candidate.missingItems.map((item) => escapeHtml(this.systemText(item))).join(", ")}` : "Bekannte Struktur erkannt"}</p></div><span class="status-chip">${candidate.status === "NOT_VERIFIED" ? "Nicht verifiziert" : candidate.status === "INCOMPLETE" ? "Unvollständig" : "Indexiert"}</span>${candidate.hasManagedDocumentCollision ? `<span class="collision-note">${icon("alert")} Bestehendes Dokument – keine Übernahme ohne Bestätigung und Sicherung</span>` : ""}</article>`).join("")}</div>` : ""}` : `<div class="scan-placeholder">${icon("scan")}<div><strong>Noch kein Scan in dieser Sitzung</strong><span>Legacy-Projekte beginnen als „Nicht verifiziert“. Fehlende historische Angaben werden nie erfunden.</span></div></div>`}
        </section>
        <section class="panel safety-panel"><div>${icon("lock")}<div><h3>Dateisystem-Sicherheit</h3><p>Alle Dateizugriffe laufen über enge native Commands. Pfade werden kanonisiert, Traversal und Symlink-Escapes abgewiesen und Schreibvorgänge atomar ausgeführt.</p></div></div><ul><li>${icon("check")} Kein stilles Überschreiben</li><li>${icon("check")} Relative Track-Pfade</li><li>${icon("check")} Originale bleiben erhalten</li></ul></section>
        <div class="workspace-actions"><button class="button button--secondary" data-action="open-workspace">Anderen Workspace öffnen</button><button class="button button--secondary" data-action="create-workspace">Neuen Workspace anlegen</button></div>
      </div>`;
  }

  protected renderSettings(): string {
    const profile = this.state.profile;
    const timestampSettings = this.state.timestampSettings;
    const audioScreeningSettings = this.state.audioScreeningSettings;
    const subscriptions = this.state.globalEvidence.filter((item) => item.role === "subscription_payment");
    const terms = this.state.globalEvidence.filter((item) => item.role === "suno_terms_rights");
    return `<div class="page-content settings-page">
        <div class="page-lead"><div><p class="overline">Workspace-Konfiguration</p><h2>Einstellungen</h2><p>Dokumentationsdaten werden als Track-Snapshot übernommen. Die Zertifikatssprache gilt für künftige Finalisierungen und steuert zusätzlich die Sprache der App. Bestehende Snapshots werden nicht verändert.</p></div></div>
        ${settingsCategoryNavigationMarkup(this.state.settingsCategory, this.language)}
        <form id="profile-form" class="panel settings-form"${this.state.settingsCategory === "files" ? " hidden" : ""}>
          <section id="settings-global" data-settings-category-panel="global" class="settings-category settings-category--global"${this.state.settingsCategory !== "global" ? " hidden" : ""}>
            <header class="settings-category-header"><span class="settings-category-index">1</span><div><p class="overline">${SETTINGS_CATEGORY_DEFINITIONS[0].description}</p><h3>${SETTINGS_CATEGORY_DEFINITIONS[0].label}</h3><p>Allgemeine Workspace- und Produktionsvorgaben für neue oder bearbeitbare Tracks.</p></div></header>
            <div class="settings-category-body">
              <div class="settings-section"><div class="settings-section-copy"><span>01</span><div><h3>Artist & Suno</h3><p>Nur produktionsrelevante Profildaten – keine privaten Kontaktdaten.</p></div></div><div class="field-grid two-col">${this.textField("artistName", "Künstlername", "Künstlername", profile.artistName, true)}${this.textField("sunoProfileName", "Suno-Profilname", "Profilname", profile.sunoProfileName, true)}${this.textField("sunoHandle", "Suno-Benutzername", "@handle", profile.sunoHandle, true)}${this.suggestedTextField("sunoPlan", "Suno-Tarif", "z. B. Premier oder eigener Wert", profile.sunoPlan, sunoPlanSuggestions, true)}${this.dateField("subscriptionStartDate", "Abo-Startdatum", profile.subscriptionStartDate, true)}</div></div>
              <div class="settings-section"><div class="settings-section-copy"><span>02</span><div><h3>Standards</h3><p>Vorbelegte Werte können pro Track angepasst werden.</p></div></div><div class="field-grid two-col">${this.suggestedTextField("defaultAiImageService", "Standard-KI-Bilddienst", "z. B. ChatGPT / OpenAI oder eigener Wert", profile.defaultAiImageService, aiSystemSuggestions)}${this.boolQuestion("defaultCommercialUse", "Kommerzielle Nutzung standardmäßig vorgesehen?", "", profile.defaultCommercialUse)}</div></div>
              <div class="settings-section"><div class="settings-section-copy"><span>03</span><div><h3>Artwork-Transparenz</h3><p>Projektinterne Richtlinie; keine pauschale gesetzliche Kennzeichnungspflicht.</p></div></div><div>${this.radioCards(
                "artworkTransparencyPolicy",
                profile.artworkTransparencyPolicy,
                [
                  ["always", "Immer sichtbaren KI-Hinweis hinzufügen", "Empfohlener Projektstandard"],
                  ["per_artwork", "Pro Artwork entscheiden", "Entscheidung wird je Track dokumentiert"],
                  ["none", "Kein automatischer sichtbarer Hinweis", "Nur Prozessdokumentation"]
                ]
              )}${this.textField("disclosureText", "Standard-Hinweistext", "AI-assisted", profile.disclosureText, true)}</div></div>
              <div class="settings-section"><div class="settings-section-copy"><span>04</span><div><h3>Zertifikatssprache</h3><p>Gilt für die nächste Finalisierung und die App-Oberfläche. Nach dem Speichern wechseln Navigation, Workflow und Dialoge auf diese Sprache. Bei der Finalisierung werden automatisch beide PDF-Sprachen erzeugt.</p></div></div><div>${this.radioCards(
                "certificateLanguage",
                profile.certificateLanguage,
                [
                  ["de", "Deutsch", "Die deutsche PDF und die App werden primär auf Deutsch dargestellt"],
                  ["en", "Englisch", "The English PDF and the app are primarily displayed in English"]
                ]
              )}</div></div>
            </div>
          </section>
          <section id="settings-external" data-settings-category-panel="external" class="settings-category settings-category--external"${this.state.settingsCategory !== "external" ? " hidden" : ""}>
            <header class="settings-category-header"><span class="settings-category-index">2</span><div><p class="overline">${SETTINGS_CATEGORY_DEFINITIONS[1].description}</p><h3>${SETTINGS_CATEGORY_DEFINITIONS[1].label}</h3><p>Zeitstempel und optionale Audio-Katalogprüfung bleiben technisch und sicher getrennt von den Track-Snapshots.</p></div></header>
            <div class="settings-category-body">
              ${this.renderTimestampSettingsSection(timestampSettings)}
              ${this.renderAudioScreeningSettingsSection(audioScreeningSettings)}
            </div>
          </section>
          <div class="form-save settings-save"><span>${icon("shield")} Stammdaten verbleiben in der lokalen Workspace-Datenbank.</span><button class="button button--primary" type="submit">${icon("check")} Einstellungen speichern</button></div>
        </form>
        <section id="settings-files" data-settings-category-panel="files" class="settings-category settings-category--files"${this.state.settingsCategory !== "files" ? " hidden" : ""}>
          <header class="settings-category-header"><span class="settings-category-index">3</span><div><p class="overline">${SETTINGS_CATEGORY_DEFINITIONS[2].description}</p><h3>${SETTINGS_CATEGORY_DEFINITIONS[2].label}</h3><p>Workspaceweit verwaltete Evidence-Dateien mit eigener Import-, Bearbeitungs- und Entfernen-Logik.</p></div></header>
          <div class="settings-category-body settings-files-body">
            <section class="panel global-evidence-panel"><div class="panel-heading"><div><p class="overline">Wiederverwendbare Nachweise</p><h3>Suno-Abo-Evidence</h3><p>Registriere jeden Beleg einmal. Bezahlrhythmus und Startdatum bestimmen automatisch den abgedeckten Monat oder das abgedeckte Jahr.</p></div><button class="button button--secondary" data-action="import-global-evidence">${icon("upload")} Abo-Nachweis registrieren</button></div>
              ${subscriptions.length ? `<div class="global-evidence-list">${subscriptions.map((item) => `<article><span class="file-icon">${icon("file")}</span><div><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart, false, this.language)} – ${formatDate(item.coverageEnd, false, this.language)}</small></div><span class="verification is-valid">${icon("check")} ${this.t("Gehasht")}</span><button class="icon-button danger" data-remove-global-evidence="${item.id}" aria-label="${escapeHtml(this.t("Globalen Nachweis entfernen"))}">${icon("trash")}</button></article>`).join("")}</div>` : `<p class="empty-inline">Noch kein globaler Abo-Nachweis registriert.</p>`}
            </section>
            <section class="panel global-evidence-panel"><div class="panel-heading"><div><p class="overline">Globale Datei für alle Projekte</p><h3>Archivierte Suno-Nutzungsbedingungen</h3><p>Dokumenttitel, Provider und Abrufdatum sind Kernmetadaten. Die lokale PDF und ihre Metadaten werden in jedes neue sowie jedes noch bearbeitbare Projekt kopiert. Finalisierte Snapshots bleiben unverändert.</p><p>Source URL, Effective Date, anwendbarer Produktionszeitraum und sachliche Notiz sind optional. SunoDM trifft keine Rechte- oder Gültigkeitsaussage.</p></div><button class="button button--secondary" data-action="import-global-terms">${icon("upload")} Terms-Evidence registrieren</button></div>
              ${
                terms.length
                  ? `<div class="global-evidence-list terms-evidence-list">${terms
                      .map((item) => {
                        const provider = item.metadata?.provider?.trim() || this.t("Provider nicht dokumentiert");
                        const retrieval = formatDate(item.metadata?.retrievalDate, false, this.language);
                        const sourceUrl = item.metadata?.sourceUrl || this.t("Source URL: Not documented");
                        const metadataComplete = termsMetadataComplete(item.metadata);
                        const metadataStatus = this.t(metadataComplete ? "Vollständig" : "Metadaten fehlen");
                        const editLabel = this.t("Metadaten bearbeiten");
                        const removeLabel = this.t("Globale Nutzungsbedingungen entfernen");
                        return `<article><span class="file-icon">${icon("file")}</span><div><strong>${escapeHtml(item.metadata?.documentTitle || item.fileName)}</strong><small>${escapeHtml(this.t(`${provider} · Abruf ${retrieval}`))}</small><small>${escapeHtml(sourceUrl)} · ${escapeHtml(item.fileName)}</small></div><span class="verification ${metadataComplete ? "is-valid" : ""}">${metadataComplete ? `${icon("check")} ${escapeHtml(metadataStatus)}` : escapeHtml(metadataStatus)}</span><button class="button button--small button--secondary" data-edit-global-terms="${item.id}">${escapeHtml(editLabel)}</button><button class="icon-button danger" data-remove-global-evidence="${item.id}" aria-label="${escapeHtml(removeLabel)}">${icon("trash")}</button></article>`;
                      })
                      .join("")}</div>`
                  : `<p class="empty-inline">Noch keine globale PDF mit Suno-Nutzungsbedingungen registriert.</p>`
              }
            </section>
          </div>
        </section>
      </div>`;
  }

  protected renderTimestampSettingsSection(settings: TimestampSettings): string {
    const result = this.state.timestampProviderTest;
    const status = result?.status ?? settings.status;
    const message = result?.message ?? settings.statusMessage;
    const testedAt = result?.testedAt ?? settings.lastTestedAt;
    const protocol = timestampProviderProtocolPresentation(
      settings.provider,
      result?.provider === settings.provider ? result.capabilities : undefined
    );
    const isReady = status === "ready";
    const statusClass = isReady ? "is-valid" : status === "disabled" ? "" : "is-warning";
    return `<div class="settings-section timestamp-settings-section"><div class="settings-section-copy"><span>05</span><div><h3>Externer Zeitstempel</h3><p>Der Dienst wird einmal pro Workspace eingerichtet. Bei der Finalisierung wird der stabile Manifest-Anchor vor dem einmaligen PDF-Rendering automatisch verwendet.</p></div></div><div>
        <label class="toggle-row"><span><strong>Externer Zeitstempel aktiviert</strong><small>Ein deaktivierter Dienst beeinflusst die technische Finalisierung nicht.</small></span><input type="checkbox" name="timestampEnabled" aria-label="Externen Zeitstempel aktivieren" ${settings.enabled ? "checked" : ""}><i aria-hidden="true"></i></label>
        <div class="field-grid two-col timestamp-settings-grid">
          ${this.selectField("timestampProvider", "Timestamp Provider", settings.provider, [
            ["free_tsa", "FreeTSA"],
            ["open_timestamps", "OpenTimestamps"],
            ["sigstore_public_tsa", "Sigstore Public TSA"],
            ["custom_rfc3161", "Custom RFC 3161"],
            ["disabled", "Disabled"]
          ])}
          <label class="toggle-row timestamp-auto-toggle"><span><strong>Automatisch bei Finalisierung</strong><small>Provider- oder Qualification-Fehler blockieren die technische Finalisierung nicht.</small></span><input type="checkbox" name="timestampAutoAfterFinalization" aria-label="Zeitstempel automatisch bei Finalisierung anfordern" ${settings.autoAfterFinalization ? "checked" : ""}><i aria-hidden="true"></i></label>
        </div>
        <p class="timestamp-settings-note"><strong>${escapeHtml(this.t(`Protokoll: ${this.t(protocol.label)}`))}</strong><br>${escapeHtml(this.t(protocol.detail))}</p>
        <div class="timestamp-provider-status ${statusClass}"><div><strong>${escapeHtml(this.t(`Status: ${this.t(timestampProviderStatusLabel(status))}`))}</strong><span>${escapeHtml(this.systemText(message || "Noch nicht getestet."))}</span>${testedAt ? `<small>Zuletzt geprüft: ${formatDate(testedAt, true, this.language)}</small>` : ""}</div><button type="button" class="button button--secondary" data-action="test-timestamp-provider" ${settings.enabled && settings.provider !== "disabled" ? "" : "disabled"}>${icon("scan")} Verbindung testen</button></div>
        <div data-timestamp-provider-configuration>${this.renderTimestampProviderConfiguration(settings)}</div>
        <p class="timestamp-settings-note">${icon("shield")} Zugangsdaten werden ausschließlich über die getrennte lokale sichere Konfiguration gespeichert. Sie erscheinen nie in Tracks, Evidenz, PDFs, Manifesten oder Revisionen.</p>
      </div></div>`;
  }

  protected renderAudioScreeningSettingsSection(settings: AudioScreeningSettings): string {
    const result = this.state.audioScreeningProviderTest;
    const status = result?.status ?? settings.status;
    const message = result?.message ?? settings.statusMessage;
    const testedAt = result?.testedAt ?? settings.lastTestedAt;
    const statusClass =
      status === "ready" ? "is-valid" : status === "disabled" || status === "not_configured" ? "" : "is-warning";
    const engineStatus = this.t(settings.localEngineAvailable ? "YES" : "NO");
    const engineVersion = settings.localEngineVersion ? ` · ${settings.localEngineVersion}` : "";
    return `<div class="settings-section audio-screening-settings-section"><div class="settings-section-copy"><span>06</span><div><h3>Pre-Release Audio Screening</h3><p>Lokaler Chromaprint-Fingerprint plus eine bewusst gestartete, optionale ACRCloud-Katalogprüfung.</p><a class="text-button audio-screening-doc-link" href="https://github.com/kleiveist/Suno-Documentation-Manager/blob/main/docs/def/pre-release-audio-screening.md" target="_blank" rel="noopener noreferrer">${icon("arrow")} Dokumentation zum Audio-Screening öffnen</a></div></div><div>
        <div class="audio-screening-local-engine"><span>${icon("hash")}</span><div><strong>${this.t("Lokale Prüfung: Chromaprint verfügbar:")} <span>${engineStatus}${escapeHtml(engineVersion)}</span></strong><small>Die lokale Fingerprint-Erzeugung benötigt keine Netzwerkverbindung.</small></div></div>
        <label class="toggle-row"><span><strong>ACRCloud aktiviert</strong><small>Die externe Prüfung wird nie beim App-Start, Import, Hashing oder Finalisieren ausgelöst.</small></span><input type="checkbox" name="audioScreeningEnabled" aria-label="ACRCloud-Audio-Screening aktivieren" ${settings.enabled ? "checked" : ""}><i aria-hidden="true"></i></label>
        ${this.renderAudioScreeningIntensitySettings(settings)}
        <div class="field-grid two-col timestamp-settings-grid">
          ${this.textField("audioScreeningHost", "ACRCloud Host", "z. B. identify-eu-west-1.acrcloud.com", settings.host)}
          ${this.textField("audioScreeningTimeoutSeconds", "Timeout (Sekunden, max. 120)", "30", String(settings.timeoutSeconds), false, "number")}
          <label class="field"><span class="field-label">Access Key</span><input type="password" name="audioScreeningAccessKey" autocomplete="new-password" placeholder="Nur lokal sicher speichern"></label>
          <label class="field"><span class="field-label">Access Secret</span><input type="password" name="audioScreeningAccessSecret" autocomplete="new-password" placeholder="Nur lokal sicher speichern"></label>
        </div>
        <div class="timestamp-provider-status ${statusClass}"><div><strong>${escapeHtml(this.t(`Status: ${this.t(audioScreeningProviderStatusLabel(status))}`))}</strong><span>${escapeHtml(this.systemText(message || "Noch nicht getestet."))}</span><small>${settings.credentialsConfigured ? "Access Key und Access Secret sind lokal konfiguriert; Werte werden nie angezeigt." : "Noch keine vollständigen lokalen Zugangsdaten hinterlegt."}</small>${testedAt ? `<small>Zuletzt geprüft: ${formatDate(testedAt, true, this.language)}</small>` : ""}</div><button type="button" class="button button--secondary" data-action="test-audio-screening-provider" ${settings.enabled ? "" : "disabled"}>${icon("scan")} Verbindung testen</button></div>
        <p class="timestamp-settings-note">${icon("shield")} SunoDM zeigt kein ACRCloud-Loginfenster. Lege das Projekt bei ACRCloud an und hinterlege hier nur Host, Access Key und Access Secret. Die Zugangsdaten bleiben getrennt von Trackdaten, Evidenz, PDFs, Manifesten und Revisionen.</p>
      </div></div>`;
  }

  protected audioScreeningPreviewTrackDurationSeconds(): number | undefined {
    const track = this.state.track;
    if (!track) return undefined;
    const release = track.evidence.find((item) => item.role === "release_wav" && item.verified);
    const localIsCurrent = localAudioScreeningIsCurrent(track.audioScreening.local, track.evidence);
    const externalIsCurrent = externalAudioScreeningIsCurrent(track.audioScreening.external, track.evidence);
    const milliseconds =
      release?.metadata?.audioDurationMilliseconds ??
      (localIsCurrent ? track.audioScreening.local.durationMilliseconds : undefined) ??
      (externalIsCurrent ? track.audioScreening.external.sourceDurationMilliseconds : undefined);
    return Number.isFinite(milliseconds) && milliseconds! > 0 ? milliseconds! / 1_000 : undefined;
  }

  protected renderAudioScreeningIntensitySettings(settings: AudioScreeningSettings): string {
    const intensityPercent = Math.min(Math.max(Math.trunc(settings.intensityPercent || 5), 1), 100);
    const referenceDurationMinutes = Math.min(Math.max(Math.round(settings.referenceDurationSeconds / 60), 1), 60);
    return `<section class="audio-screening-intensity" aria-labelledby="audio-screening-intensity-title">
        <div class="audio-screening-intensity__heading"><h4 id="audio-screening-intensity-title">ACRCloud-Prüfintensität</h4><p>Lege fest, wie viel eines Tracks bei einer bewusst gestarteten externen Prüfung repräsentativ und ohne überlappende Samples geprüft wird.</p></div>
        <label class="field audio-screening-intensity__slider"><span class="field-label">Gewünschte Prüfintensität <strong data-audio-screening-intensity-value>${intensityPercent} %</strong></span><input type="range" name="audioScreeningIntensityPercent" min="1" max="100" step="1" value="${intensityPercent}" list="audio-screening-intensity-stops" aria-label="Gewünschte ACRCloud-Prüfintensität" aria-valuemin="1" aria-valuemax="100" aria-valuenow="${intensityPercent}"><datalist id="audio-screening-intensity-stops"><option value="5" label="5 %"></option><option value="10" label="10 %"></option><option value="25" label="25 %"></option><option value="50" label="50 %"></option><option value="75" label="75 %"></option><option value="100" label="100 %"></option></datalist><span class="audio-screening-intensity__stops" aria-hidden="true"><span>5 %</span><span>10 %</span><span>25 %</span><span>50 %</span><span>75 %</span><span>100 %</span></span></label>
        <div data-audio-screening-intensity-presentation aria-live="polite">${this.renderAudioScreeningIntensityPresentation(settings)}</div>
        <label class="toggle-row audio-screening-intensity__mode"><span><strong>Dynamische Berechnung nach tatsächlicher Tracklänge</strong><small>Ist der Schalter aktiv, wird die Zielprüfzeit beim Lauf aus der verifizierten Dauer der aktuellen Release-Datei bestimmt.</small></span><input type="checkbox" name="audioScreeningDynamicByTrackDuration" aria-label="Dynamische Berechnung nach tatsächlicher Tracklänge" ${settings.dynamicByTrackDuration ? "checked" : ""}><i aria-hidden="true"></i></label>
        <div class="audio-screening-intensity__reference" data-audio-screening-reference ${settings.dynamicByTrackDuration ? "hidden" : ""}><label class="field"><span class="field-label">Referenzlänge (Minuten)</span><input type="number" name="audioScreeningReferenceDurationMinutes" min="1" max="60" step="1" value="${escapeHtml(String(referenceDurationMinutes))}" inputmode="numeric" aria-label="Referenzlänge in Minuten"><small class="field-help">Die tatsächliche Trackdauer wird beim Lauf nie überschritten.</small></label></div>
      </section>`;
  }

  protected audioScreeningDurationBasis(
    settings: AudioScreeningSettings,
    trackDurationSeconds: number | undefined,
    calculationDurationSeconds: number,
    isGerman: boolean
  ): string {
    if (!settings.dynamicByTrackDuration) {
      return isGerman
        ? `Feste Referenzlänge: ${formatScreeningSeconds(calculationDurationSeconds, this.language)}`
        : `Fixed reference length: ${formatScreeningSeconds(calculationDurationSeconds, this.language)}`;
    }
    if (trackDurationSeconds) {
      return isGerman
        ? `Aktueller Track: ${formatScreeningSeconds(trackDurationSeconds, this.language)}`
        : `Current track: ${formatScreeningSeconds(trackDurationSeconds, this.language)}`;
    }
    return isGerman
      ? `Dynamisch · Vorschau mit Referenzlänge ${formatScreeningSeconds(calculationDurationSeconds, this.language)}`
      : `Dynamic · preview with ${formatScreeningSeconds(calculationDurationSeconds, this.language)} reference length`;
  }

  protected audioScreeningRequestBand(band: ReturnType<typeof audioScreeningIntensityBand>, isGerman: boolean): string {
    const german = {
      low: "niedrig",
      normal: "normal",
      elevated: "erhöht",
      high: "hoch",
      very_high: "sehr hoch"
    } as const;
    const english = {
      low: "low",
      normal: "normal",
      elevated: "elevated",
      high: "high",
      very_high: "very high"
    } as const;
    return isGerman ? german[band] : english[band];
  }

  protected renderAudioScreeningIntensityWarning(
    actualRequestCount: number,
    band: ReturnType<typeof audioScreeningIntensityBand>,
    isGerman: boolean
  ): string {
    if (band !== "high" && band !== "very_high") return "";
    const maximumClass = band === "very_high" ? "is-maximum" : "";
    const heading = isGerman ? "Hohe Prüfintensität" : "High screening intensity";
    const message = isGerman
      ? "Diese Einstellung erzeugt viele externe ACRCloud-Anfragen und kann Prüfzeit sowie API-Verbrauch deutlich erhöhen."
      : "This setting creates many external ACRCloud requests and can significantly increase screening time and API usage.";
    const maximum =
      actualRequestCount === ACRCLOUD_MAX_REQUESTS
        ? `<small>${isGerman ? "Maximale Prüfintensität: Bis zu 25 eindeutige ACRCloud-Samples bzw. maximal 300 Sekunden Audio pro Track." : "Maximum screening intensity: up to 25 unique ACRCloud samples and at most 300 seconds of audio per track."}</small>`
        : "";
    return `<div class="audio-screening-intensity__warning ${maximumClass}">${icon("alert")}<div><strong>${heading}</strong><span>${message}</span>${maximum}</div></div>`;
  }

  protected renderAudioScreeningIntensityPresentation(settings: AudioScreeningSettings): string {
    const trackDurationSeconds = this.audioScreeningPreviewTrackDurationSeconds();
    const estimate = audioScreeningIntensityEstimate(settings, trackDurationSeconds);
    const intensityPercent = Math.min(Math.max(Math.trunc(settings.intensityPercent || 5), 1), 100);
    const band = audioScreeningIntensityBand(estimate.actualRequestCount);
    const isGerman = this.language === "de";
    const summary = isGerman
      ? `${intensityPercent} % · Ziel: ca. ${formatScreeningSeconds(estimate.targetDurationSeconds, this.language)} · ${estimate.actualRequestCount} ACRCloud-Requests`
      : `${intensityPercent}% · target: approx. ${formatScreeningSeconds(estimate.targetDurationSeconds, this.language)} · ${estimate.actualRequestCount} ACRCloud requests`;
    const durationBasis = this.audioScreeningDurationBasis(
      settings,
      trackDurationSeconds,
      estimate.calculationDurationSeconds,
      isGerman
    );
    const requestBand = this.audioScreeningRequestBand(band, isGerman);
    const requestCaption = isGerman
      ? `${estimate.actualRequestCount} erwartete Requests · ${requestBand}`
      : `${estimate.actualRequestCount} expected requests · ${requestBand}`;
    const capNotice = estimate.capped
      ? `<small class="audio-screening-intensity__cap">${isGerman ? "Die verfügbare Trackdauer oder die feste Höchstgrenze reduziert die Request-Anzahl automatisch." : "The available track duration or fixed cap automatically reduces the request count."}</small>`
      : "";
    const hardLimit = isGerman
      ? "Feste Obergrenze: maximal 25 ACRCloud-Requests bzw. 300 Sekunden eindeutiges Audio pro Track; jeder Request enthält höchstens 12 Sekunden."
      : "Hard limit: at most 25 ACRCloud requests and 300 seconds of unique audio per track; each request contains at most 12 seconds.";
    const warning = this.renderAudioScreeningIntensityWarning(estimate.actualRequestCount, band, isGerman);
    return `<div class="audio-screening-intensity__preview"><strong>${escapeHtml(summary)}</strong><span>${escapeHtml(durationBasis)}</span><dl><div><dt>${isGerman ? "Berechnete Zielprüfzeit" : "Calculated target screening time"}</dt><dd>${escapeHtml(formatScreeningSeconds(estimate.targetDurationSeconds, this.language))}</dd></div><div><dt>${isGerman ? "Erwartete Request-Anzahl" : "Expected request count"}</dt><dd>${escapeHtml(requestCaption)}</dd></div><div><dt>${isGerman ? "Max. eindeutige Audiodauer" : "Max. unique audio duration"}</dt><dd>${escapeHtml(formatScreeningSeconds(estimate.maxUniqueDurationSeconds, this.language))}</dd></div></dl><small class="audio-screening-intensity__cap">${escapeHtml(hardLimit)}</small>${capNotice}${warning}</div>`;
  }

  protected renderTimestampProviderConfiguration(settings: TimestampSettings): string {
    const isRfc3161 =
      settings.provider === "free_tsa" ||
      settings.provider === "sigstore_public_tsa" ||
      settings.provider === "custom_rfc3161";
    if (!isRfc3161) return "";
    const custom = settings.custom;
    const trustAnchorField = this.textField(
      "timestampCaCertificatePath",
      "TSA CA Trust Anchor",
      "Lokaler PEM- oder DER-Pfad (erforderlich für VERIFIED)",
      custom.caCertificatePath,
      true
    );
    if (settings.provider !== "custom_rfc3161") {
      return `<details class="timestamp-provider-advanced" open><summary>RFC-3161-Verifikation</summary>
          <p>VERIFIED wird nur nach CMS-Signatur-, Nonce-, Policy-, EKU-, Gültigkeits- und Vertrauensketteprüfung gegen diesen ausdrücklich gewählten Trust Anchor vergeben.</p>
          <div class="field-grid two-col">${trustAnchorField}</div>
        </details>`;
    }
    const auth = custom.authenticationMode;
    const needsSecret = auth === "basic" || auth === "bearer_token" || auth === "api_key";
    const secretLabel = auth === "basic" ? "Passwort" : auth === "api_key" ? "API-Key" : "Token";
    return `<details class="timestamp-provider-advanced" open><summary>Erweiterte Einstellungen für Custom RFC 3161</summary>
        <p>Provider-, Authentifizierungs- und Policy-Angaben für den eigenen RFC-3161-Dienst. Der Trust Anchor ist für den Status VERIFIED erforderlich.</p>
        <div class="field-grid two-col">
          ${this.textField("timestampCustomProviderName", "Provider Name", "z. B. Unternehmens-TSA", custom.providerName)}
          ${this.textField("timestampCustomEndpoint", "TSA Endpoint", "https://…", custom.endpoint, false, "url")}
          ${this.selectField("timestampAuthenticationMode", "Authentication Mode", auth, [
            ["none", "Keine Authentifizierung"],
            ["basic", "Basic Authentication"],
            ["bearer_token", "Bearer Token"],
            ["api_key", "API-Key"],
            ["client_certificate", "Client Certificate"]
          ])}
          ${this.textField("timestampTimeoutSeconds", "Timeout (Sekunden, max. 120)", "15", String(custom.timeoutSeconds), false, "number")}
          ${this.textField("timestampPolicyOid", "Policy OID", "Optional", custom.policyOid)}
          ${trustAnchorField}
          ${auth === "basic" ? this.textField("timestampUsername", "Username", "Optionaler Accountname", custom.username) : ""}
          ${auth === "client_certificate" ? this.textField("timestampClientCertificatePath", "Client Certificate", "Lokaler Zertifikatspfad", custom.clientCertificatePath) : ""}
          ${needsSecret ? `<label class="field"><span class="field-label">${secretLabel}</span><input type="password" name="timestampSecret" autocomplete="new-password" placeholder="Nur lokal sicher speichern"></label><label class="timestamp-clear-secret"><input type="checkbox" name="timestampClearSecret"><span>Gespeicherte Zugangsdaten entfernen</span></label>` : ""}
        </div>
      </details>`;
  }

  protected readTimestampSettings(form: HTMLFormElement): TimestampSettings {
    const data = new FormData(form);
    const previous = this.state.timestampSettings;
    const read = (name: string, fallback: string): string => {
      const value = data.get(name);
      return typeof value === "string" ? value : fallback;
    };
    const providerCandidate = read("timestampProvider", previous.provider);
    const provider: TimestampProviderKind = [
      "disabled",
      "free_tsa",
      "open_timestamps",
      "sigstore_public_tsa",
      "custom_rfc3161"
    ].includes(providerCandidate)
      ? (providerCandidate as TimestampProviderKind)
      : previous.provider;
    const authenticationCandidate = read("timestampAuthenticationMode", previous.custom.authenticationMode);
    const authenticationMode: TimestampAuthenticationMode = [
      "none",
      "basic",
      "bearer_token",
      "api_key",
      "client_certificate"
    ].includes(authenticationCandidate)
      ? (authenticationCandidate as TimestampAuthenticationMode)
      : previous.custom.authenticationMode;
    const timeoutCandidate = Number(read("timestampTimeoutSeconds", String(previous.custom.timeoutSeconds)));
    const timeoutSeconds =
      Number.isFinite(timeoutCandidate) && timeoutCandidate > 0
        ? Math.min(Math.trunc(timeoutCandidate), 120)
        : previous.custom.timeoutSeconds;
    return {
      ...previous,
      enabled: data.get("timestampEnabled") === "on",
      provider,
      autoAfterFinalization: data.get("timestampAutoAfterFinalization") === "on",
      custom: {
        ...previous.custom,
        providerName: read("timestampCustomProviderName", previous.custom.providerName).trim(),
        endpoint: read("timestampCustomEndpoint", previous.custom.endpoint).trim(),
        authenticationMode,
        username: read("timestampUsername", previous.custom.username).trim(),
        clientCertificatePath: read("timestampClientCertificatePath", previous.custom.clientCertificatePath).trim(),
        caCertificatePath: read("timestampCaCertificatePath", previous.custom.caCertificatePath).trim(),
        policyOid: read("timestampPolicyOid", previous.custom.policyOid).trim(),
        timeoutSeconds
      }
    };
  }

  protected readAudioScreeningSettings(form: HTMLFormElement): AudioScreeningSettings {
    const data = new FormData(form);
    const previous = this.state.audioScreeningSettings;
    const read = (name: string, fallback: string): string => {
      const value = data.get(name);
      return typeof value === "string" ? value : fallback;
    };
    const timeoutCandidate = Number(read("audioScreeningTimeoutSeconds", String(previous.timeoutSeconds)));
    const timeoutSeconds =
      Number.isFinite(timeoutCandidate) && timeoutCandidate > 0
        ? Math.min(Math.trunc(timeoutCandidate), 120)
        : previous.timeoutSeconds;
    const intensityCandidate = Number(read("audioScreeningIntensityPercent", String(previous.intensityPercent)));
    const intensityPercent = Number.isFinite(intensityCandidate)
      ? Math.min(Math.max(Math.trunc(intensityCandidate), 1), 100)
      : previous.intensityPercent;
    const referenceMinutesCandidate = Number(
      read("audioScreeningReferenceDurationMinutes", String(previous.referenceDurationSeconds / 60))
    );
    const referenceDurationSeconds =
      Number.isFinite(referenceMinutesCandidate) && referenceMinutesCandidate > 0
        ? Math.min(Math.max(Math.round(referenceMinutesCandidate * 60), 1), 3_600)
        : previous.referenceDurationSeconds;
    return {
      ...previous,
      enabled: data.get("audioScreeningEnabled") === "on",
      host: read("audioScreeningHost", previous.host).trim(),
      timeoutSeconds,
      intensityPercent,
      dynamicByTrackDuration: data.get("audioScreeningDynamicByTrackDuration") === "on",
      referenceDurationSeconds
    };
  }

  protected readTimestampSecret(form: HTMLFormElement): string | null | undefined {
    const data = new FormData(form);
    if (data.get("timestampClearSecret") === "on") return null;
    const value = data.get("timestampSecret");
    return typeof value === "string" && value.length > 0 ? value : undefined;
  }

  protected readAudioScreeningSecret(form: HTMLFormElement): AudioScreeningSecretInput | undefined {
    const data = new FormData(form);
    const accessKey = data.get("audioScreeningAccessKey");
    const accessSecret = data.get("audioScreeningAccessSecret");
    const input: AudioScreeningSecretInput = {};
    if (typeof accessKey === "string" && accessKey.trim()) input.accessKey = accessKey;
    if (typeof accessSecret === "string" && accessSecret.trim()) input.accessSecret = accessSecret;
    return Object.keys(input).length ? input : undefined;
  }

  protected async updateTimestampSettingsFromForm(
    form: HTMLFormElement,
    testProvider = false
  ): Promise<{ settings: TimestampSettings; test: TimestampProviderTestResult | null }> {
    const settings = this.readTimestampSettings(form);
    const secret = this.readTimestampSecret(form);
    await this.api.updateTimestampSettings(settings);
    if (secret !== undefined) await this.api.updateTimestampSecret(secret);
    const test = testProvider ? await this.api.testTimestampProvider() : null;
    const refreshed = await this.api.getTimestampSettings();
    return { settings: refreshed, test };
  }

  protected async updateAudioScreeningSettingsFromForm(
    form: HTMLFormElement,
    testProvider = false
  ): Promise<{ settings: AudioScreeningSettings; test: AudioScreeningProviderTestResult | null }> {
    const settings = this.readAudioScreeningSettings(form);
    const secret = this.readAudioScreeningSecret(form);
    await this.api.updateAudioScreeningSettings(settings);
    if (secret) await this.api.updateAudioScreeningSecret(secret);
    const test = testProvider ? await this.api.testAudioScreeningProvider() : null;
    const refreshed = await this.api.getAudioScreeningSettings();
    return { settings: refreshed, test };
  }

  protected syncTimestampProviderConfiguration(form: HTMLFormElement): void {
    const target = form.querySelector<HTMLElement>("[data-timestamp-provider-configuration]");
    if (!target) return;
    target.innerHTML = this.renderTimestampProviderConfiguration(this.readTimestampSettings(form));
    translateRenderedUi(target, this.language, this.protectedRenderedValues());
  }

  protected syncAudioScreeningIntensityPresentation(form: HTMLFormElement): void {
    const target = form.querySelector<HTMLElement>("[data-audio-screening-intensity-presentation]");
    if (!target) return;
    const settings = this.readAudioScreeningSettings(form);
    const range = form.elements.namedItem("audioScreeningIntensityPercent") as HTMLInputElement | null;
    const value = form.querySelector<HTMLElement>("[data-audio-screening-intensity-value]");
    const reference = form.querySelector<HTMLElement>("[data-audio-screening-reference]");
    if (range) range.setAttribute("aria-valuenow", String(settings.intensityPercent));
    if (value) value.textContent = `${settings.intensityPercent} %`;
    if (reference) reference.hidden = settings.dynamicByTrackDuration;
    target.innerHTML = this.renderAudioScreeningIntensityPresentation(settings);
    translateRenderedUi(target, this.language, this.protectedRenderedValues());
  }
}
