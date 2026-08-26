import type {
  DocumentationAnswer,
  EvidenceMetadata,
  EvidenceRole,
  FactOrigin,
  GlobalProfile,
  OperationProgress,
  TrackDetail
} from "../domain/types";
import { evidenceRoleFileTypes, evidenceRoleLabel, filenameMatchesDocumentedTitle } from "../domain/workflow";
import { escapeHtml, formatDate } from "../ui/format";
import { icon } from "../ui/icons";
import {
  canonicalGuidedChoiceArray,
  canonicalGuidedChoiceList,
  canonicalGuidedChoiceValue,
  canonicalTrackFieldPatch,
  evidenceRoles,
  parseMultiChoiceValue,
  singleChoiceFieldMarkup
} from "./choices";
import type { GuidedChoice, SingleChoiceOption } from "./choices";
import { AppSettingsRender } from "./class-settings-render";
import { workflowUpgradeFinalizationBlocker } from "./navigation";
import type { LongOperationKind } from "./progress";
import { isTrackContentLocked, shouldDiscardLockedDraft } from "./state";
import { isAutomaticDateReadonly } from "./timestamp-presentation";
import { factOriginLabel } from "./track-presentation";

export abstract class AppActions extends AppSettingsRender {
  protected inlineEvidenceActions(track: TrackDetail, actions: Array<[EvidenceRole, string]>): string {
    const locked = isTrackContentLocked(track.status);
    return `<div class="inline-evidence">${actions
      .map(([role, label]) => {
        const present = [...track.evidence].reverse().find((item) => item.role === role);
        const types = evidenceRoleFileTypes(role);
        return `<div class="evidence-control ${present ? "is-present" : ""}">
          <button type="button" class="evidence-button ${present?.verified ? "is-present" : ""}" ${present ? `data-preview-evidence="${escapeHtml(present.id)}"` : `data-import-role="${role}" ${locked ? "disabled" : ""}`}>${present?.verified ? icon("check") : icon("upload")}<span><strong>${escapeHtml(label)}</strong><small>${present ? (present.verified ? "Vorhanden – klicken für Vorschau" : "Vorhanden, aber nicht verifiziert – klicken für Vorschau") : evidenceRoleLabel(role)}</small><small class="evidence-types">Gefordert: ${escapeHtml(types)}</small></span></button>
          ${present ? `<button type="button" class="evidence-reupload" data-import-role="${role}" data-replace-evidence="${escapeHtml(present.id)}" aria-label="${escapeHtml(label)} ersetzen" title="Datei ersetzen" ${locked ? "disabled" : ""}>${icon("upload")}</button>` : ""}
        </div>`;
      })
      .join("")}</div>`;
  }

  protected filenameConfirmation(
    track: TrackDetail,
    role: EvidenceRole,
    field: "releaseFilenameDifferenceConfirmed" | "sunoExportFilenameDifferenceConfirmed",
    confirmed: boolean | null,
    label: string
  ): string {
    const item = [...track.evidence].reverse().find((entry) => entry.role === role && entry.verified);
    const actual = item?.metadata?.originalFileName?.trim() ?? "";
    if (!actual)
      return `<div class="neutral-message">${icon("info")}<div><strong>${escapeHtml(label)}: tatsächlicher Quelldateiname nicht erfasst</strong><span>Importiere oder ersetze die Datei, damit der Name als Evidence-derived metadata gespeichert wird.</span></div></div>`;
    if (filenameMatchesDocumentedTitle(this.state.trackDraft?.title ?? track.fields.title, actual)) {
      return `<div class="neutral-message">${icon("check")}<div><strong>${escapeHtml(label)} passt zum dokumentierten Titel</strong><span>${escapeHtml(actual)}</span></div></div>`;
    }
    return `<div class="conditional-panel"><div class="conditional-line"></div><p><strong>Dateinamenabweichung erkannt</strong><br>Dokumentierter Titel: ${escapeHtml(this.state.trackDraft?.title ?? track.fields.title)}<br>Tatsächlicher Dateiname: ${escapeHtml(actual)}</p>${this.boolQuestion(field, "Ist diese Abweichung beabsichtigt?", "Bestätige ausdrücklich oder korrigiere den dokumentierten Titel. Der Titel wird niemals aus dem Dateinamen abgeleitet.", confirmed)}</div>`;
  }

  protected textField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    required = false,
    type = "text"
  ): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><input type="${type}" name="${name}" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${required ? "required" : ""}></label>`;
  }

  protected suggestedTextField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    suggestions: readonly string[],
    required = false
  ): string {
    const listId = `${name}-suggestions`;
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><input type="text" name="${escapeHtml(name)}" list="${escapeHtml(listId)}" autocomplete="off" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${required ? "required" : ""}><datalist id="${escapeHtml(listId)}">${suggestions.map((suggestion) => `<option value="${escapeHtml(suggestion)}"></option>`).join("")}</datalist><small class="field-help">Vorschlag wählen oder einen beliebigen eigenen Wert eingeben.</small></label>`;
  }

  protected dateField(name: string, label: string, value: string, required = false): string {
    return this.textField(name, label, "", value, required, "date");
  }

  protected automatedDateField(
    name: string,
    label: string,
    value: string,
    origin: FactOrigin,
    required = true,
    fallbackCaption?: string
  ): string {
    const automated = isAutomaticDateReadonly(origin);
    const requirement = !automated && required ? "required" : "";
    const caption = automated ? factOriginLabel(origin) : (fallbackCaption ?? factOriginLabel(origin));
    return `<label class="field ${automated ? "field--automated" : ""}"><span class="field-label">${escapeHtml(label)}${!automated && required ? " *" : ""}</span><span class="field-input-wrap"><input type="date" name="${escapeHtml(name)}" value="${escapeHtml(value)}" ${automated ? 'readonly aria-readonly="true"' : requirement}>${automated ? `<i title="Evidence-derived metadata">${icon("check")}</i>` : ""}</span><small class="field-caption">${escapeHtml(caption)}</small></label>`;
  }

  protected automatedTextField(
    name: string,
    label: string,
    placeholder: string,
    value: string,
    origin: FactOrigin
  ): string {
    const automated = isAutomaticDateReadonly(origin);
    const caption = automated
      ? factOriginLabel(origin)
      : "Manuelle Angabe; wird nur bei leerem Feld aus gültigen Suno-WAV-Metadaten ergänzt.";
    return `<label class="field ${automated ? "field--automated" : ""}"><span class="field-label">${escapeHtml(label)}</span><span class="field-input-wrap"><input type="text" name="${escapeHtml(name)}" placeholder="${escapeHtml(placeholder)}" value="${escapeHtml(value)}" ${automated ? 'readonly aria-readonly="true"' : ""}>${automated ? `<i title="Evidence-derived metadata">${icon("check")}</i>` : ""}</span><small class="field-caption">${escapeHtml(caption)}</small></label>`;
  }

  protected hasManualSunoDateOverriddenByMetadata(previous: TrackDetail, imported: TrackDetail): boolean {
    const dates: Array<[string, string, FactOrigin, FactOrigin]> = [
      [
        previous.fields.sunoFinalGenerationDate,
        imported.fields.sunoFinalGenerationDate,
        previous.automation.finalGenerationOrigin,
        imported.automation.finalGenerationOrigin
      ],
      [
        previous.fields.productionEndDate,
        imported.fields.productionEndDate,
        previous.automation.productionEndOrigin,
        imported.automation.productionEndOrigin
      ],
      [
        previous.fields.sunoDownloadExportDate,
        imported.fields.sunoDownloadExportDate,
        previous.automation.downloadExportOrigin,
        imported.automation.downloadExportOrigin
      ],
      [
        previous.fields.finalExportDate,
        imported.fields.finalExportDate,
        previous.automation.finalExportOrigin,
        imported.automation.finalExportOrigin
      ]
    ];
    return dates.some(
      ([before, after, previousOrigin, importedOrigin]) =>
        Boolean(before) &&
        before !== after &&
        previousOrigin !== "evidence_derived_metadata" &&
        importedOrigin === "evidence_derived_metadata"
    );
  }

  protected renderAutomaticSunoMetadata(track: TrackDetail): string {
    const timestamp = track.automation.sunoCreatedTimestamp;
    const sunoId = track.automation.sunoId;
    if (!track.automation.sunoMetadataDetected || !timestamp || !sunoId) return "";

    return `<section class="policy-card"><div>${icon("check")}<p class="overline">Automatisch aus Suno-WAV erkannt</p><h4>Aus Dateimetadaten</h4><dl><div><dt>Suno Studio</dt><dd>Ja</dd></div><div><dt>Download/Export</dt><dd>${escapeHtml(formatDate(track.fields.sunoDownloadExportDate, false, this.language))}</dd></div><div><dt>Suno ID</dt><dd><code>${escapeHtml(sunoId)}</code></dd></div></dl><details><summary>Technische Details</summary><p>Embedded Suno export timestamp: <code>${escapeHtml(timestamp)}</code></p></details></div></section>`;
  }

  protected textArea(name: string, label: string, placeholder: string, value: string, required = false): string {
    return `<label class="field field--wide"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><textarea name="${name}" placeholder="${escapeHtml(placeholder)}" ${required ? "required" : ""}>${escapeHtml(value)}</textarea></label>`;
  }

  protected multiChoiceField(
    name: string,
    label: string,
    value: string,
    options: readonly GuidedChoice[],
    required = false
  ): string {
    const selected = parseMultiChoiceValue(canonicalGuidedChoiceList(value, options));
    const known = new Set(options.map(([option]) => option));
    const choices: GuidedChoice[] = [
      ...options,
      ...selected
        .filter((item) => !known.has(item))
        .map(
          (item) =>
            [
              item,
              this.language === "en"
                ? `Previous selection: ${item} (please review)`
                : `Bisherige Auswahl: ${item} (bitte prüfen)`
            ] as const
        )
    ];
    return `<fieldset class="multi-choice-field field--wide" data-multi-choice-group ${required ? `data-multi-choice-required aria-required="true"` : ""}><legend>${escapeHtml(label)}${required ? " *" : ""}</legend><div>${choices.map(([option, optionLabel]) => `<label><input type="checkbox" name="${name}" value="${escapeHtml(option)}" data-multi-choice ${selected.includes(option) ? "checked" : ""}><span>${escapeHtml(known.has(option) ? this.guidedChoiceLabel(option, optionLabel) : optionLabel)}</span></label>`).join("")}</div>${required ? `<p class="field-help">Wähle mindestens einen tatsächlich ausgeführten Schritt aus.</p>` : ""}</fieldset>`;
  }

  protected multiChoiceArrayField(
    name: string,
    label: string,
    value: readonly string[],
    options: readonly GuidedChoice[],
    required = false
  ): string {
    const selected = canonicalGuidedChoiceArray(value, options);
    const known = new Set(options.map(([option]) => option));
    const choices: GuidedChoice[] = [
      ...options,
      ...selected
        .filter((item) => !known.has(item))
        .map(
          (item) =>
            [item, this.language === "en" ? `Previous free text: ${item}` : `Bisheriger Freitext: ${item}`] as const
        )
    ];
    return `<fieldset class="multi-choice-field field--wide" data-multi-choice-group data-choice-array ${required ? `data-multi-choice-required aria-required="true"` : ""}><legend>${escapeHtml(label)}${required ? " *" : ""}</legend><div>${choices.map(([option, optionLabel]) => `<label><input type="checkbox" name="${escapeHtml(name)}" value="${escapeHtml(option)}" data-multi-choice ${selected.includes(option) ? "checked" : ""}><span>${escapeHtml(known.has(option) ? this.guidedChoiceLabel(option, optionLabel) : optionLabel)}</span></label>`).join("")}</div>${required ? `<p class="field-help">Wähle mindestens einen tatsächlich ausgeführten Schritt aus.</p>` : `<p class="field-help">Mehrere Angaben können gleichzeitig ausgewählt und durch Freitext ergänzt werden.</p>`}</fieldset>`;
  }

  protected selectField(
    name: string,
    label: string,
    value: string,
    options: readonly SingleChoiceOption[],
    required = false
  ): string {
    return `<label class="field"><span class="field-label">${escapeHtml(label)}${required ? " *" : ""}</span><select name="${name}" ${required ? "required" : ""}>${options.map(([id, text]) => `<option value="${escapeHtml(id)}" ${value === id ? "selected" : ""}>${escapeHtml(text)}</option>`).join("")}</select></label>`;
  }

  protected guidedSingleChoiceField(
    name: string,
    label: string,
    value: string,
    choices: readonly GuidedChoice[],
    required = false
  ): string {
    const selected = canonicalGuidedChoiceValue(value, choices);
    const known = choices.some(([option]) => option === selected);
    const options: SingleChoiceOption[] = choices.map(([option, optionLabel]) => [
      option,
      this.guidedChoiceLabel(option, optionLabel)
    ]);
    if (selected && !known)
      options.unshift([
        selected,
        this.language === "en"
          ? `Previous value: ${selected} (please review)`
          : `Bisheriger Wert: ${selected} (bitte prüfen)`
      ]);
    return singleChoiceFieldMarkup(name, label, selected, options, required);
  }

  protected guidedChoiceLabel(option: string, germanLabel: string): string {
    return this.language === "en" ? option : germanLabel;
  }

  protected boolQuestion(name: string, label: string, help: string, value: boolean | null): string {
    return `<fieldset class="boolean-field"><legend>${escapeHtml(label)}</legend>${help ? `<p>${escapeHtml(help)}</p>` : ""}<div><label><input type="radio" name="${name}" value="true" ${value === true ? "checked" : ""}><span>${icon("check")} Ja</span></label><label><input type="radio" name="${name}" value="false" ${value === false ? "checked" : ""}><span>${icon("close")} Nein</span></label></div></fieldset>`;
  }

  protected documentationBooleanQuestion(name: string, label: string, help: string, value: boolean | null): string {
    return `<fieldset class="boolean-field documentation-answer-field"><legend>${escapeHtml(label)}</legend>${help ? `<p>${escapeHtml(help)}</p>` : ""}<div><label><input type="radio" name="${escapeHtml(name)}" value="yes" data-documentation-boolean ${value === true ? "checked" : ""}><span>${icon("check")} YES</span></label><label><input type="radio" name="${escapeHtml(name)}" value="no" data-documentation-boolean ${value === false ? "checked" : ""}><span>${icon("close")} NO</span></label><label><input type="radio" name="${escapeHtml(name)}" value="not_documented" data-documentation-boolean ${value === null ? "checked" : ""}><span>${icon("info")} NOT DOCUMENTED</span></label></div></fieldset>`;
  }

  protected documentationAnswerQuestion(name: string, label: string, value: DocumentationAnswer | null): string {
    return `<fieldset class="boolean-field documentation-answer-field"><legend>${escapeHtml(label)}</legend><div><label><input type="radio" name="${escapeHtml(name)}" value="yes" ${value === "yes" ? "checked" : ""} required><span>${icon("check")} YES</span></label><label><input type="radio" name="${escapeHtml(name)}" value="no" ${value === "no" ? "checked" : ""} required><span>${icon("close")} NO</span></label><label><input type="radio" name="${escapeHtml(name)}" value="not_documented" ${value === "not_documented" ? "checked" : ""} required><span>${icon("info")} NOT DOCUMENTED</span></label></div></fieldset>`;
  }

  protected radioCards(name: string, value: string, options: Array<[string, string, string]>): string {
    return `<div class="radio-cards">${options.map(([id, label, help]) => `<label><input type="radio" name="${name}" value="${id}" ${id === value ? "checked" : ""}><span><i></i><strong>${escapeHtml(label)}</strong><small>${escapeHtml(help)}</small></span></label>`).join("")}</div>`;
  }

  protected policyLabel(policy: GlobalProfile["artworkTransparencyPolicy"]): string {
    return policy === "always"
      ? "Immer sichtbaren KI-Hinweis hinzufügen"
      : policy === "per_artwork"
        ? "Pro Artwork entscheiden"
        : "Kein automatischer sichtbarer Hinweis";
  }

  protected emptyState(
    iconName: "tracks" | "current" | "file",
    title: string,
    copy: string,
    action?: string,
    actionLabel?: string
  ): string {
    return `<div class="empty-state">${icon(iconName)}<h3>${escapeHtml(title)}</h3><p>${escapeHtml(copy)}</p>${action ? `<button class="button button--secondary" data-action="${action}">${escapeHtml(actionLabel)}</button>` : ""}</div>`;
  }

  protected requireTrack(): TrackDetail {
    if (!this.state.track) throw new Error("Wähle zuerst einen Track aus.");
    return this.state.track;
  }

  protected rejectLockedContentMutation(): boolean {
    const track = this.state.track;
    if (!track || !isTrackContentLocked(track.status)) return false;
    this.state.trackDraft = structuredClone(track.fields);
    this.draftDirty = false;
    if (track.status === "FINALIZED") {
      this.showToast(
        "info",
        "Neue Revision erforderlich",
        "Der finalisierte Snapshot wurde nicht verändert. Lege zuerst eine neue Revision an."
      );
    } else {
      this.showToast(
        "info",
        "Ersetzter Snapshot – nur lesbar",
        "Dieser historische Snapshot bleibt unverändert. Öffne die aktuelle Revision, um Inhalte zu bearbeiten."
      );
    }
    return true;
  }

  protected async chooseWorkspace(kind: "open" | "create"): Promise<void> {
    if (!(await this.flushDraft())) return;
    const workspace = await this.withBusy(
      kind === "open" ? "Ordnerdialog wird geöffnet …" : "Workspace wird angelegt …",
      () => (kind === "open" ? this.api.openWorkspace(this.language) : this.api.createWorkspace(this.language))
    );
    if (workspace) await this.enterWorkspace(workspace);
  }

  protected async scanWorkspace(): Promise<void> {
    if (!(await this.flushDraft())) return;
    const scan = await this.withBusy("Workspace wird sicher gescannt …", () => this.api.scanWorkspace());
    if (!scan) return;
    this.state.scanResult = scan;
    await this.refreshTracks();
    this.state.view = "workspace";
    this.showToast(
      "success",
      "Scan abgeschlossen",
      `${scan.discovered} Track-Ordner erkannt. Es wurden keine bestehenden Dateien überschrieben.`
    );
  }

  protected async importEvidence(role: EvidenceRole, replaceEvidenceId?: string): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    if (
      replaceEvidenceId &&
      !window.confirm(
        this.t(
          "Vorhandene Evidence durch die neu ausgewählte Datei ersetzen? Die bisherige verwaltete Kopie wird lokal archiviert."
        )
      )
    )
      return;
    const track = this.requireTrack();
    const metadata = this.collectEvidenceMetadata(role);
    if (metadata === null) return;
    const imported = await this.withBusy(
      "Datei auswählen; große Dateien werden im Hintergrund kopiert und gehasht …",
      () => this.api.importEvidence(track.id, role, replaceEvidenceId, metadata, this.language)
    );
    if (imported) {
      if (role === "final_artwork") this.trackCoverCache.delete(track.id);
      this.applyTrack(imported);
      const overriddenBySunoMetadata =
        role === "suno_final_export" &&
        imported.automation.sunoMetadataDetected &&
        this.hasManualSunoDateOverriddenByMetadata(track, imported);
      const importMessage = this.t(`${evidenceRoleLabel(role)} wurde kopiert, gehasht und dem Track zugeordnet.`);
      const archivedMessage = replaceEvidenceId ? ` ${this.t("Die vorherige Kopie wurde archiviert.")}` : "";
      const sunoSummary =
        role === "suno_final_export" && imported.automation.sunoMetadataDetected
          ? ` ${this.t("Suno Studio und der eingebettete Erzeugungszeitpunkt wurden automatisch erkannt.")}${imported.automation.sunoId ? ` ${this.t("Die technische Suno-ID wurde als Evidence erhalten.")}` : ""}`
          : "";
      this.showToast(
        "success",
        replaceEvidenceId ? "Evidence ersetzt" : "Evidence importiert",
        `${importMessage}${archivedMessage}${sunoSummary}`
      );
      if (overriddenBySunoMetadata) {
        this.showToast(
          "info",
          "Suno-Metadaten überschreiben Nutzerangabe",
          "Abweichende Benutzerangabe durch Suno-WAV-Metadaten erkannt. Die technisch aus dem WAV gewonnene Information wird als Evidence-derived metadata verwendet."
        );
      }
    }
  }

  protected collectEvidenceMetadata(_role: EvidenceRole): Partial<EvidenceMetadata> | null | undefined {
    void _role;
    return undefined;
  }

  protected async previewEvidence(evidenceId: string): Promise<void> {
    const track = this.requireTrack();
    const preview = await this.withBusy("Evidence-Vorschau wird vorbereitet …", () =>
      this.api.previewEvidence(track.id, evidenceId)
    );
    if (!preview) return;
    this.state.showNewTrack = false;
    this.state.showTrackLibrary = false;
    this.state.showSubscriptionEvidence = false;
    this.state.evidencePreview = preview;
    this.render();
  }

  protected async chooseEvidenceRole(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    const labels = evidenceRoles.map((role, index) => `${index + 1}: ${this.t(evidenceRoleLabel(role))}`).join("\n");
    const choice = window.prompt(
      `${this.t("Rolle der Evidence wählen:")}\n\n${labels}\n\n${this.t("Nummer eingeben:")}`
    );
    if (!choice) return;
    const role = evidenceRoles[Number(choice) - 1];
    if (!role) {
      this.showToast("error", "Ungültige Rolle", "Wähle eine Nummer aus der angezeigten Liste.");
      return;
    }
    const existing = ["release_wav", "suno_final_export", "final_artwork"].includes(role)
      ? [...this.requireTrack().evidence].reverse().find((item) => item.role === role)
      : undefined;
    await this.importEvidence(role, existing?.id);
  }

  protected async addDeviation(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const description = window.prompt(this.t("Abweichung sachlich beschreiben:"));
    if (!description?.trim()) return;
    const blocking = window.confirm(this.t("Soll diese Abweichung die Finalisierung blockieren?"));
    await this.trackMutation(
      "Abweichung wird gespeichert …",
      () => this.api.addDeviation(this.requireTrack().id, description, blocking),
      "Abweichung gespeichert"
    );
  }

  protected async saveTrackDraft(): Promise<void> {
    if (!this.state.trackDraft) return;
    if (this.rejectLockedContentMutation()) return;
    const patch = canonicalTrackFieldPatch(this.state.trackDraft);
    const updated = await this.withBusy("Track-Angaben werden gespeichert …", () =>
      this.api.updateTrack(this.requireTrack().id, patch)
    );
    if (updated) {
      this.applyTrack(updated);
      this.showToast("success", "Schritt gespeichert", "Der Dokumentationsstatus wurde neu bewertet.");
    }
  }

  protected async flushDraft(): Promise<boolean> {
    if (!this.draftDirty || !this.state.trackDraft || !this.state.track) return true;
    if (shouldDiscardLockedDraft(this.state.track.status, this.draftDirty)) {
      this.state.trackDraft = structuredClone(this.state.track.fields);
      this.draftDirty = false;
      return true;
    }
    const patch = canonicalTrackFieldPatch(this.state.trackDraft);
    const updated = await this.withBusy("Ungespeicherte Angaben werden zuerst gesichert …", () =>
      this.api.updateTrack(this.state.track!.id, patch)
    );
    if (!updated) return false;
    this.applyTrack(updated);
    return true;
  }

  protected async trackMutation(
    label: string,
    action: () => Promise<TrackDetail>,
    success: string,
    allowLocked = false
  ): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = await this.withBusy(label, action);
    if (track) {
      this.applyTrack(track);
      this.showToast("success", success, "Der Track-Status wurde neu bewertet.");
    }
  }

  protected async runAction(
    label: string,
    action: () => Promise<{ message: string; track?: TrackDetail }>,
    allowLocked = false
  ): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const result = await this.withBusy(label, action);
    if (!result) return;
    if (result.track) this.applyTrack(result.track);
    else await this.refreshTracks();
    this.showToast(
      "success",
      "Aktion abgeschlossen",
      this.systemText(result.message, this.language === "en" ? "Action completed." : "Aktion abgeschlossen.")
    );
  }

  protected async runProgressAction(
    kind: LongOperationKind,
    label: string,
    action: (onProgress: (progress: OperationProgress) => void) => Promise<{ message: string; track?: TrackDetail }>,
    allowLocked = false
  ): Promise<void> {
    if (!allowLocked && this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const result = await this.withOperationProgress(kind, label, action);
    if (!result) return;
    if (result.track) this.applyTrack(result.track);
    else await this.refreshTracks();
    this.showToast(
      "success",
      "Aktion abgeschlossen",
      this.systemText(result.message, this.language === "en" ? "Action completed." : "Aktion abgeschlossen.")
    );
  }

  protected async finalizeTrack(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const workflowBlocker = workflowUpgradeFinalizationBlocker(track, this.state.workflow);
    if (workflowBlocker) {
      this.showToast("error", "Workflow-Neubewertung erforderlich", this.systemText(workflowBlocker));
      return;
    }
    const validation = await this.withBusy("Finalisierungs-Gate wird nativ geprüft …", () =>
      this.api.validateTrack(track.id)
    );
    if (!validation) return;
    if (!validation.valid) {
      this.showToast(
        "error",
        "Finalisierung blockiert",
        [...validation.missingItems, ...validation.blockingItems].map((item) => this.systemText(item)).join(" · ")
      );
      return;
    }
    const result = await this.withOperationProgress(
      "finalization",
      "Unveränderlicher Snapshot und Zertifikat werden erzeugt …",
      (onProgress) => this.api.finalizeTrack(track.id, undefined, onProgress)
    );
    if (result) {
      if (result.track) this.applyTrack(result.track);
      else await this.refreshTracks();
      this.state.trackTab = "certificate";
      this.state.activeStep = null;
      this.state.showCertificatePopup = Boolean(
        this.state.track?.certificate.valid && this.state.track.certificate.certificateId
      );
      this.showToast(
        "success",
        "Dokumentation finalisiert",
        this.systemText(
          result.message,
          this.language === "en" ? "Documentation finalized." : "Dokumentation finalisiert."
        )
      );
    }
  }

  protected async generateDocumentsSafely(): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    if (!(await this.flushDraft())) return;
    const track = this.requireTrack();
    const preview = await this.withBusy("Dokumentgenerierung wird sicher vorbereitet …", () =>
      this.api.previewDocumentGeneration(track.id)
    );
    if (!preview) return;
    let adoptExisting = false;
    if (preview.adoptionRequired || preview.collisions.length > 0) {
      const collisionList = preview.collisions.join("\n");
      const confirmation =
        this.language === "en"
          ? `Existing managed documents detected:\n\n${collisionList}\n\nThe native application preserves the existing state under .archive before writing new managed documents. Continue?`
          : `Bestehende verwaltete Dokumente erkannt:\n\n${collisionList}\n\nDie native Anwendung sichert den vorhandenen Zustand unter .archive, bevor neue verwaltete Dokumente geschrieben werden. Fortfahren?`;
      adoptExisting = window.confirm(confirmation);
      if (!adoptExisting) {
        this.showToast("info", "Dokumentgenerierung abgebrochen", "Bestehende Dateien wurden nicht verändert.");
        return;
      }
    }
    await this.runProgressAction("documents", "Dokumente werden atomar erzeugt …", (onProgress) =>
      this.api.generateDocuments(track.id, adoptExisting, onProgress)
    );
  }
}
