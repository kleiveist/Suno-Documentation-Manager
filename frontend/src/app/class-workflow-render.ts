import type {
  AudioScreeningExternalSummary,
  EvidenceItem,
  FactOrigin,
  StepId,
  TrackDetail,
  TrackFields
} from "../domain/types";
import {
  calculateMissingRequirements,
  evidenceRoleFileTypes,
  evidenceRoleLabel,
  subscriptionEvidenceRelevance,
  subscriptionGenerationCoverageStatus,
  subscriptionProductionCoverageStatus,
  visibleConditionalFields
} from "../domain/workflow";
import { escapeHtml, formatBytes, formatDate } from "../ui/format";
import { icon } from "../ui/icons";
import {
  audioScreeningProviderIsReady,
  audioScreeningProviderStatusLabel,
  audioScreeningStatusClass,
  audioScreeningStatusLabel,
  externalAudioScreeningIsCurrent,
  formatAudioDuration,
  formatAudioTimestamp,
  formatScreeningSeconds,
  localAudioScreeningIsCurrent,
  visibleExternalAudioScreening,
  visibleLocalAudioScreening
} from "./audio-screening";
import {
  aiArtworkHumanChangeChoices,
  aiSystemSuggestions,
  audioDisclosureLocationChoices,
  codeAudioPostProcessingChoices,
  evidenceProvenanceLabel,
  externalAudioRightsChoices,
  externalAudioSourceChoices,
  humanArtworkProcessChoices,
  humanWorkChoices,
  ownAudioRightsChoices,
  ownAudioSourceChoices,
  postExportWorkChoices,
  releaseNoteChoices,
  sampleRightsChoices,
  sampleSourceChoices,
  singleChoiceFieldMarkup,
  SUNO_CONTENT_CLASSIFICATION_CHOICES,
  sunoModelSuggestions,
  sunoPlanSuggestions,
  VOCAL_INTENT_CHOICES
} from "./choices";
import { AppLibraryRender } from "./class-library-render";
import { isTrackContentLocked } from "./state";
import { legacySunoPlanNoticeMarkup, termsMetadataComplete } from "./timestamp-presentation";
import { generativeAiAudioDetailState } from "./track-presentation";

interface StepContentSections {
  body: string;
  bottomSection: string;
}

export abstract class AppWorkflowRender extends AppLibraryRender {
  protected renderTrackStepSections(track: TrackDetail, draft: TrackFields): StepContentSections {
    let body = "";
    const bottomSection = "";
    body = `<div class="field-grid two-col">${this.textField("title", "Track-Titel", "Name des Tracks", draft.title, true)}${this.dateField("productionStartDate", "Produktionsstart", draft.productionStartDate, true)}${this.automatedDateField("productionEndDate", "Produktionsende", draft.productionEndDate, track.automation.productionEndOrigin)}</div>
                <div class="form-section">${this.boolQuestion("commercialUseIntended", "Kommerzielle Nutzung vorgesehen?", "Der tatsächlich für diesen Track verwendete Wert wird im Dokument-Snapshot gespeichert.", draft.commercialUseIntended)}</div>`;
    return { body, bottomSection };
  }

  protected renderSourceStepSections(
    track: TrackDetail,
    draft: TrackFields,
    conditional: ReadonlySet<string>
  ): StepContentSections {
    let body = "";
    const bottomSection = "";
    body = `${this.boolQuestion("externalAudioUploaded", "Externes Audio hochgeladen?", "Audio außerhalb der eigenen Produktion, das Suno als Quelle erhalten hat.", draft.externalAudioUploaded)}
                ${
                  conditional.has("externalAudioSource")
                    ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("externalAudioSource", "Quelle", draft.externalAudioSource, externalAudioSourceChoices, true)}${this.guidedSingleChoiceField("externalAudioOwnership", "Rechtezuordnung", draft.externalAudioOwnership, externalAudioRightsChoices, true)}</div>${this.inlineEvidenceActions(
                        track,
                        [
                          ["external_audio_file", "Audiodatei importieren"],
                          ["external_audio_license", "Lizenznachweis importieren"]
                        ]
                      )}</div>`
                    : ""
                }
                ${this.boolQuestion("ownAudioUploaded", "Eigene Audiodatei hochgeladen?", "Eine von dir erstellte Aufnahme oder Instrumentalspur.", draft.ownAudioUploaded)}
                ${conditional.has("ownAudioSource") ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("ownAudioSource", "Quelle", draft.ownAudioSource, ownAudioSourceChoices, true)}${this.guidedSingleChoiceField("ownAudioOwnership", "Rechtezuordnung", draft.ownAudioOwnership, ownAudioRightsChoices, true)}</div>${this.inlineEvidenceActions(track, [["own_audio_file", "Eigene Audiodatei importieren"]])}</div>` : ""}
                ${this.boolQuestion("codeBasedGeneration", "Codebasierte Erzeugung?", "Wurde eine Audiodatei oder ein Ausgangsmaterial mithilfe von Quellcode erzeugt?", draft.codeBasedGeneration)}
                ${conditional.has("sourceCodeFile") ? `<div class="conditional-panel"><div class="conditional-line"></div><p class="field-help">Importiere zuerst den tatsächlich verwendeten Quellcode oder die Quelldatei.</p>${this.inlineEvidenceActions(track, [["source_code_file", "Quellcode oder Quelldatei importieren"]])}${this.boolQuestion("codeAudioPostProcessed", "Wurde das aus dem Quellcode erzeugte Audio nachbearbeitet?", "Es werden nur ausdrücklich bestätigte Bearbeitungen dokumentiert.", draft.codeAudioPostProcessed)}${conditional.has("codeAudioPostProcessingOperations") ? this.multiChoiceArrayField("codeAudioPostProcessingOperations", "Welche Nachbearbeitungen wurden durchgeführt?", draft.codeAudioPostProcessingOperations, codeAudioPostProcessingChoices, true) : ""}${conditional.has("codeAudioPostProcessingNote") ? this.textArea("codeAudioPostProcessingNote", "Sonstige Nachbearbeitung – Details", "Frei beschreibbare zusätzliche Nachbearbeitung", draft.codeAudioPostProcessingNote) : ""}<div class="form-section"><p class="field-label">Erzeugte Audio-Datei</p><p class="field-help">Importiere abschließend die tatsächlich aus dem Quellcode erzeugte WAV- oder MP3-Datei.</p>${this.inlineEvidenceActions(track, [["code_generated_audio_file", "Erzeugte WAV- oder MP3-Datei importieren"]])}</div></div>` : ""}
                ${this.boolQuestion("thirdPartySamplesUploaded", "Fremde Samples hochgeladen?", "Samples oder Loops, die von Dritten stammen.", draft.thirdPartySamplesUploaded)}
                ${
                  conditional.has("thirdPartySampleSource")
                    ? `<div class="conditional-panel"><div class="conditional-line"></div><div class="field-grid two-col">${this.guidedSingleChoiceField("thirdPartySampleSource", "Sample-Quelle", draft.thirdPartySampleSource, sampleSourceChoices, true)}${this.guidedSingleChoiceField("thirdPartySampleOwnership", "Lizenz / Rechte", draft.thirdPartySampleOwnership, sampleRightsChoices, true)}</div>${this.inlineEvidenceActions(
                        track,
                        [
                          ["third_party_sample_file", "Sample-Datei importieren"],
                          ["third_party_sample_license", "Sample-Lizenz importieren"]
                        ]
                      )}</div>`
                    : ""
                }`;
    return { body, bottomSection };
  }

  protected renderSunoStepSections(track: TrackDetail, draft: TrackFields): StepContentSections {
    let body = "";
    const bottomSection = "";
    {
      const projectHint = track.evidence.find((item) => item.role === "suno_screenshot");
      body = `<div class="field-grid two-col">${this.suggestedTextField("sunoModel", "Suno-Modell", "z. B. v5.5 oder eigener Wert", draft.sunoModel, sunoModelSuggestions, true)}${this.suggestedTextField("sunoPlanAtGeneration", "Suno-Tarif bei der finalen Generation", "z. B. Premier oder historischer Tarif", draft.sunoPlanAtGeneration, sunoPlanSuggestions, true)}${this.textField("sunoProjectUrl", "Suno-Projekt-URL", "https://suno.com/song/…", draft.sunoProjectUrl, true, "url")}${this.automatedTextField("sunoFinalGenerationId", "Final generation ID (optional)", "Wird aus gültigen Suno-WAV-Metadaten übernommen", draft.sunoFinalGenerationId, track.automation.finalGenerationIdOrigin)}${this.automatedDateField("sunoFinalGenerationDate", "Datum der finalen Generation", draft.sunoFinalGenerationDate, track.automation.finalGenerationOrigin)}${this.automatedDateField("sunoDownloadExportDate", "Download-/Exportdatum (optional)", draft.sunoDownloadExportDate, track.automation.downloadExportOrigin, false, "Kein gültiges Datum in den WAV-Metadaten erkannt – die manuelle Angabe bleibt optional.")}</div>
                ${legacySunoPlanNoticeMarkup(draft.legacySunoPlanAtCreation)}
                ${this.renderAutomaticSunoMetadata(track)}
                <div class="form-section"><p class="field-label">Suno-Projektnachweis</p>${projectHint ? `<p class="field-help">Sunoprojekthinweis hinterlegt: <strong>${escapeHtml(projectHint.fileName)}</strong></p>` : ""}${this.inlineEvidenceActions(
                  track,
                  [
                    ["suno_screenshot", "Screenshot importieren"],
                    ["suno_project_zip", "Projekt-ZIP importieren"],
                    ["suno_final_export", "Suno-Export importieren"]
                  ]
                )}</div>`;
      body += this.filenameConfirmation(
        track,
        "suno_final_export",
        "sunoExportFilenameDifferenceConfirmed",
        draft.sunoExportFilenameDifferenceConfirmed,
        "Suno-Export"
      );
    }
    return { body, bottomSection };
  }

  protected renderHumanWorkStepSections(
    track: TrackDetail,
    draft: TrackFields,
    conditional: ReadonlySet<string>
  ): StepContentSections {
    let body = "";
    const bottomSection = "";
    {
      const hasLegacyLyrics = Boolean(draft.legacyLyricsSource || draft.legacyLyricsText.trim());
      body = `<section class="question-group lyrics-structure-section"><div><p class="overline">Suno Generation Text Field</p><h4>Suno-Textfeld, Vocal Intent und finales Audio getrennt dokumentieren</h4><p>Strukturanweisungen wie [Intro], [Drop] oder [Outro] werden nicht als Vocal Lyrics oder Gesang gewertet. Die Klassifikation des Textfeldes und das tatsächliche Audioergebnis bleiben getrennte Angaben.</p></div>
                ${this.boolQuestion("instrumentalTrack", "Suno-Instrumentalmodus ausgewählt?", "UI-Einstellung in Suno; daraus wird das Audioergebnis nicht automatisch abgeleitet.", draft.instrumentalTrack)}
                <div class="field-grid two-col">${this.selectField("sunoContentClassification", "Inhaltsklassifizierung", draft.sunoContentClassification ?? "", SUNO_CONTENT_CLASSIFICATION_CHOICES, true)}${this.selectField("vocalIntent", "Vocal Intent", draft.vocalIntent ?? "", VOCAL_INTENT_CHOICES, true)}</div>
                ${this.renderGenerationTextFieldFacts(draft)}
                ${
                  conditional.has("sunoLyricsFieldText")
                    ? `<div class="conditional-panel"><div class="conditional-line"></div>${conditional.has("sunoLyricsOtherContentType") ? this.textField("sunoLyricsOtherContentType", "Sonstiger Content-Typ", "Faktisch beschreiben", draft.sunoLyricsOtherContentType, true) : ""}${singleChoiceFieldMarkup(
                        "sunoLyricsContentSource",
                        "Content source",
                        draft.sunoLyricsContentSource ?? "",
                        [
                          ["human", "Human"],
                          ["ai", "AI"],
                          ["mixed", "Mixed"]
                        ],
                        true
                      )}${this.textArea("sunoLyricsFieldText", "Exakter Inhalt des Suno-Textfelds", "Exakte verwendete Fassung einschließlich eckiger Klammern dokumentieren.", draft.sunoLyricsFieldText, true)}</div>`
                    : ""
                }
                ${this.boolQuestion("vocalLyricsPresent", "Finaler Audioinhalt enthält Gesang?", "Tatsächliches Ergebnis des finalen Audios; nicht aus dem Textfeld automatisch ableiten.", draft.vocalLyricsPresent)}
                ${hasLegacyLyrics ? `<div class="legacy-data-notice">${icon("info")}<div class="legacy-data-notice-copy"><strong>Historische Lyrics-Angaben – nicht automatisch klassifiziert</strong><p>${escapeHtml(this.t(`Quelle: ${draft.legacyLyricsSource || this.t("Not documented")}`))}</p>${draft.legacyLyricsText ? `<pre>${escapeHtml(draft.legacyLyricsText)}</pre>` : ""}<small>Diese unverändert erhaltenen Altwerte bestimmen weder Vocal Lyrics noch Content-Typen. Prüfe die neuen Felder ausdrücklich.</small></div><button type="button" class="icon-button danger legacy-data-remove" data-action="remove-legacy-lyrics" aria-label="Historische Lyrics-Angaben entfernen" title="Historische Lyrics-Angaben entfernen" ${isTrackContentLocked(track.status) ? "disabled" : ""}>${icon("trash")}</button></div>` : ""}
              </section>
                ${this.textArea("sunoStylePrompt", "Suno-Style-Prompt", "Den in Suno verwendeten Style-Prompt vollständig dokumentieren.", draft.sunoStylePrompt, true)}
                ${this.boolQuestion("humanEditingPerformed", "Menschliche Bearbeitung durchgeführt?", "Nur bestätigen, wenn sie tatsächlich stattgefunden hat. Freitext wird nicht zur Vocal-Lyrics-Klassifikation verwendet.", draft.humanEditingPerformed)}
                ${conditional.has("humanEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceField("humanEditingDetails", "Bestätigte Schritte", draft.humanEditingDetails, humanWorkChoices, true)}</div>` : ""}`;
    }
    return { body, bottomSection };
  }

  protected renderArtworkStepSections(
    track: TrackDetail,
    draft: TrackFields,
    conditional: ReadonlySet<string>
  ): StepContentSections {
    let body = "";
    const bottomSection = "";
    body = `<div class="policy-card artwork-factual-notice">${icon("info")}<div><p class="overline">Nur relevante Angaben</p><h4>Faktische Dokumentation</h4><p>Die App dokumentiert deine Bestätigung und trifft keine rechtliche Entscheidung.</p></div></div><div class="field-grid two-col">${this.selectField(
      "artworkOrigin",
      "Entstehung des Artworks",
      draft.artworkOrigin,
      [
        ["", "Bitte auswählen"],
        ["none", "Kein Artwork"],
        ["human", "Menschlich erstellt"],
        ["ai_generated", "KI-generiert"],
        ["ai_assisted", "KI-assistiert"]
      ],
      true
    )}${conditional.has("aiImageService") ? this.suggestedTextField("aiImageService", "KI-Bilddienst", "Verwendeter Dienst", draft.aiImageService, aiSystemSuggestions, true) : ""}</div>
                <div class="technical-note">${icon("info")}<p>Import-Zeitstempel dokumentieren nur den Import in SunoDM und nicht die tatsächliche Erstellungs- oder Bearbeitungsreihenfolge der Artwork-Dateien.</p></div>
                <div class="form-section"><p class="field-label">Suno-Original-Artwork</p>${this.inlineEvidenceActions(track, [["artwork_suno_original", "Suno-Original importieren"]])}</div>
                ${conditional.has("humanArtworkProcessOperations") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceArrayField("humanArtworkProcessOperations", "Menschlicher Arbeitsprozess", draft.humanArtworkProcessOperations, humanArtworkProcessChoices)}${this.textArea("humanArtworkProcessNotes", "Beschreibung / Ergänzungen", "Arbeitsprozess frei beschreiben oder die Auswahl ergänzen", draft.humanArtworkProcessNotes)}</div>` : ""}
                ${conditional.has("humanArtworkModifications") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceArrayField("humanArtworkModifications", "Menschliche Änderungen", draft.humanArtworkModifications, aiArtworkHumanChangeChoices, true)}${conditional.has("customArtworkChange") ? this.textArea("customArtworkChange", "Sonstige menschliche Bearbeitung – Details", "Frei beschreibbare zusätzliche Änderung", draft.customArtworkChange) : ""}</div>` : ""}
                ${
                  conditional.has("aiArtworkOriginal")
                    ? `<div class="form-section"><p class="field-label">Originale KI-Ausgabe</p>${this.inlineEvidenceActions(
                        track,
                        [
                          ["ai_artwork_original", "KI-Original importieren"],
                          ["ai_artwork_edited", "KI-bearbeitete Version importieren"],
                          ["human_edited_artwork", "Menschlich bearbeitete Version importieren"]
                        ]
                      )}</div>`
                    : ""
                }
                ${draft.artworkOrigin && draft.artworkOrigin !== "none" ? `<div class="question-group"><div><p class="overline">Artwork Content Check</p><h4>Ja oder Nein auswählen</h4><p>Folgeangaben erscheinen nur bei Ja und bleiben frei beschreibbar.</p></div>${this.boolQuestion("depictsRealPerson", "Zeigt das Artwork absichtlich eine reale Person?", "", draft.depictsRealPerson)}${conditional.has("realPersonNotes") ? this.textArea("realPersonNotes", "Welche reale Person wird dargestellt bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.realPersonNotes, true) : ""}${this.boolQuestion("depictsRealEvent", "Stellt es ein reales Ereignis als authentisch dar?", "", draft.depictsRealEvent)}${conditional.has("realEventNotes") ? this.textArea("realEventNotes", "Welches reale Ereignis wird dargestellt bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.realEventNotes, true) : ""}${this.boolQuestion("containsTrademark", "Reproduziert es eine Marke oder ein Firmenlogo?", "", draft.containsTrademark)}${conditional.has("trademarkNotes") ? this.textArea("trademarkNotes", "Welche Marke oder welches Firmenlogo wird reproduziert bzw. in welchem Zusammenhang?", "Darstellung und Kontext faktisch beschreiben", draft.trademarkNotes, true) : ""}</div><div class="form-section"><p class="field-label">Finales Artwork</p><p class="field-help">Lade hier die endgültige JPG- oder PNG-Datei hoch, die du aus Suno heruntergeladen hast. Falls ein sichtbarer KI-Hinweis erforderlich ist, ersetzt du sie anschließend durch die lokal gekennzeichnete Fassung.</p>${this.inlineEvidenceActions(track, [["final_artwork", "Finales Suno-Artwork importieren"]])}</div>` : ""}`;
    return { body, bottomSection };
  }

  protected renderAiTransparencyStepSections(
    track: TrackDetail,
    draft: TrackFields,
    conditional: ReadonlySet<string>
  ): StepContentSections {
    let body = "";
    const bottomSection = "";
    body = `<div class="policy-card">${icon("info")}<div><p class="overline">Faktische Dokumentation</p><h4>Audio-Assessment und Artwork-Disclosure sind getrennt</h4><p>SunoDM erfasst Benutzerangaben und technische Workflow-Prüfungen. Daraus wird keine AI-Act-, Rechte- oder sonstige Rechtsbewertung abgeleitet.</p></div></div>
                <section class="question-group ai-assessment-section"><div><p class="overline">AI Transparency Assessment – Audio</p><h4>Audio-bezogene Tatsachen</h4><p>YES, NO und NOT DOCUMENTED bleiben ausdrücklich unterscheidbar.</p></div>
                  ${this.documentationBooleanQuestion("generativeAiUsed", "Generative AI used", "NOT DOCUMENTED bleibt offen und ist nicht dasselbe wie NO.", draft.generativeAiUsed)}
                  ${
                    generativeAiAudioDetailState(draft.generativeAiUsed) === "details_required"
                      ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.suggestedTextField("audioAiSystem", "AI system", "z. B. Suno, ChatGPT / OpenAI oder eigener Wert", draft.audioAiSystem, aiSystemSuggestions, true)}
                    ${this.documentationAnswerQuestion("aiAssistedAudioElements", "AI-assisted audio elements", draft.aiAssistedAudioElements)}
                    ${this.documentationAnswerQuestion("aiGeneratedAudioElements", "AI-generated audio elements", draft.aiGeneratedAudioElements)}
                    ${this.documentationAnswerQuestion("realPersonVoiceIntentionallyImitated", "Real person voice intentionally imitated", draft.realPersonVoiceIntentionallyImitated)}
                    ${this.documentationAnswerQuestion("realPersonIdentityIntentionallyRepresented", "Real person's identity intentionally represented", draft.realPersonIdentityIntentionallyRepresented)}
                    ${this.documentationAnswerQuestion("realEventRepresentedAsAuthenticRecording", "Real event represented as authentic recording", draft.realEventRepresentedAsAuthenticRecording)}
                    ${this.documentationAnswerQuestion("realLocationInstitutionEventPresentedAsAuthenticAiRecording", "Real location / institution / event presented as authentic AI recording", draft.realLocationInstitutionEventPresentedAsAuthenticAiRecording)}
                    ${this.documentationAnswerQuestion("audioDisclosureApplied", "Disclosure applied", draft.audioDisclosureApplied)}
                    ${conditional.has("audioDisclosureLocations") ? this.multiChoiceArrayField("audioDisclosureLocations", "Disclosure locations", draft.audioDisclosureLocations, audioDisclosureLocationChoices, true) : ""}
                    ${conditional.has("audioDisclosureText") ? this.textArea("audioDisclosureText", "Disclosure text", "Tatsächlich verwendeten Hinweis dokumentieren", draft.audioDisclosureText, true) : ""}
                    ${conditional.has("audioDisclosureReason") ? this.textArea("audioDisclosureReason", "Reason / note for NO", "Optionaler sachlicher Grund; die App bewertet ihn nicht rechtlich.", draft.audioDisclosureReason) : ""}
                    ${draft.commercialUseIntended && draft.audioDisclosureApplied === "not_documented" ? `<div class="danger-banner is-warning">${icon("alert")}<div><strong>Audio-Disclosure: NOT DOCUMENTED</strong><span>Bei kommerziell vorgesehener Nutzung mit generativer KI bleibt KI-Transparenz offen.</span></div></div>` : ""}
                  </div>`
                      : generativeAiAudioDetailState(draft.generativeAiUsed) === "not_applicable"
                        ? `<div class="neutral-message">${icon("info")}<div><strong>Audio-Detailfragen: N/A</strong><span>„Generative AI used“ wurde ausdrücklich mit NO dokumentiert.</span></div></div>`
                        : `<div class="danger-banner is-warning">${icon("alert")}<div><strong>Generative AI used: NOT DOCUMENTED</strong><span>Die Angabe ist offen. Erst YES blendet die Audio-Detailfragen ein; NO markiert sie als N/A.</span></div></div>`
                  }
                </section>
                <section class="question-group ai-assessment-section"><div><p class="overline">AI Transparency Assessment – Artwork</p><h4>${this.policyLabel(track.profileSnapshot.artworkTransparencyPolicy)}</h4><p>Projektinterne Artwork-Regel; keine pauschale gesetzliche Aussage.</p></div>
                  ${conditional.has("disclosure") ? `${this.boolQuestion("disclosureApplied", "Sichtbaren Artwork-Hinweis anwenden?", "Explizite YES-/NO-Entscheidung. Bei YES muss die gekennzeichnete Fassung lokal erzeugt werden.", draft.disclosureApplied)}${draft.disclosureApplied === true ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.textField("disclosureText", "Sichtbarer Artwork-Hinweis", "AI-assisted", draft.disclosureText, true)}<button type="button" class="button button--accent" data-action="generate-disclosure">${icon("certificate")} Sichtbaren Artwork-Hinweis lokal erzeugen</button></div>` : draft.disclosureApplied === false ? `<div class="neutral-message">${icon("info")}<div><strong>Artwork-Hinweis: NO</strong><span>Die Nichtanwendung wurde bewusst dokumentiert.</span></div></div>` : `<div class="danger-banner is-warning">${icon("alert")}<div><strong>Artwork-Hinweis: Entscheidung offen</strong><span>Für jedes KI-Artwork muss YES oder NO ausdrücklich dokumentiert werden.</span></div></div>`}` : `<div class="neutral-message">${icon("info")}<div><strong>Artwork-Disclosure: N/A</strong><span>Für dieses Artwork wurde keine KI-Entstehung dokumentiert.</span></div></div>`}
                </section>`;
    return { body, bottomSection };
  }

  protected renderReleaseStepSections(
    track: TrackDetail,
    draft: TrackFields,
    conditional: ReadonlySet<string>
  ): StepContentSections {
    let body = "";
    let bottomSection = "";
    {
      const metadataDate =
        track.automation.sunoMetadataDetected && track.automation.sunoCreatedTimestamp
          ? track.automation.sunoCreatedTimestamp.slice(0, 10)
          : "";
      const noDesktopEditing = draft.postExportEditingPerformed === false;
      const metadataControlsLastEditing = noDesktopEditing && Boolean(metadataDate);
      const finalDateOrigin: FactOrigin = metadataControlsLastEditing
        ? "evidence_derived_metadata"
        : track.automation.finalExportOrigin === "evidence_derived_metadata"
          ? "not_documented"
          : track.automation.finalExportOrigin;
      const finalDateValue = metadataControlsLastEditing
        ? metadataDate
        : track.automation.finalExportOrigin === "evidence_derived_metadata"
          ? ""
          : draft.finalExportDate;
      body = `${this.boolQuestion("postExportEditingPerformed", "Wurde die Datei noch einmal auf dem Desktop-PC bearbeitet?", "Bei Ja dokumentierst du das Datum der letzten Bearbeitung selbst. Bei Nein übernimmt die App das erkannte Datum aus der Suno-WAV.", draft.postExportEditingPerformed)}
                ${conditional.has("postExportEditingDetails") ? `<div class="conditional-panel"><div class="conditional-line"></div>${this.multiChoiceField("postExportEditingDetails", "Bestätigte Bearbeitungsschritte auf dem Desktop-PC", draft.postExportEditingDetails, postExportWorkChoices, true)}</div>` : ""}
                ${
                  draft.postExportEditingPerformed === null
                    ? `<div class="neutral-message">${icon("info")}<div><strong>Datum der letzten Bearbeitung</strong><span>Beantworte zuerst die Ja-/Nein-Frage.</span></div></div>`
                    : `<div class="field-grid two-col">${this.automatedDateField("finalExportDate", "Datum der letzten Bearbeitung", finalDateValue, finalDateOrigin, true, "Kein gültiges WAV-Metadatum erkannt – bitte Datum manuell dokumentieren.")}${this.multiChoiceField("releaseNotes", "Release-Notizen", draft.releaseNotes, releaseNoteChoices)}</div>`
                }
                <div class="form-section"><p class="field-label">Finale Release-Dateien</p><p class="field-help">Der ursprüngliche Quelldateiname wird getrennt vom verwalteten Pfad dokumentiert. Das finale Artwork wird einmalig in Schritt 05 verwaltet.</p>${this.inlineEvidenceActions(
                  track,
                  [
                    ["release_wav", "Finale Release-Audiodatei importieren"],
                    ["release_mp3", "Zusätzliche MP3 importieren"],
                    ["release_mp4", "MP4 importieren"]
                  ]
                )}</div>`;
      body += this.filenameConfirmation(
        track,
        "release_wav",
        "releaseFilenameDifferenceConfirmed",
        draft.releaseFilenameDifferenceConfirmed,
        "Release-Datei"
      );
      bottomSection = this.renderPreReleaseAudioScreening(track);
    }
    return { body, bottomSection };
  }

  protected renderStepContent(track: TrackDetail, stepId: StepId): string {
    const draft = this.state.trackDraft ?? track.fields;
    const conditional = visibleConditionalFields(draft, track.profileSnapshot);
    if (stepId === "evidence_licenses") return this.renderEvidence(track, true);
    if (stepId === "integrity") return this.renderIntegrity(track);
    if (stepId === "finalize") return this.renderFinalization(track);

    const sections = (() => {
      switch (stepId) {
        case "track":
          return this.renderTrackStepSections(track, draft);
        case "source":
          return this.renderSourceStepSections(track, draft, conditional);
        case "suno":
          return this.renderSunoStepSections(track, draft);
        case "human_work":
          return this.renderHumanWorkStepSections(track, draft, conditional);
        case "artwork":
          return this.renderArtworkStepSections(track, draft, conditional);
        case "ai_transparency":
          return this.renderAiTransparencyStepSections(track, draft, conditional);
        case "release":
          return this.renderReleaseStepSections(track, draft, conditional);
      }
    })();
    const { body, bottomSection } = sections;
    const locked = isTrackContentLocked(track.status);
    return `<form id="track-step-form" class="workflow-form ${locked ? "is-read-only" : ""}" data-step="${stepId}" ${locked ? `aria-label="Historischer Snapshot – schreibgeschützt"` : ""}><fieldset class="workflow-form-fields" ${locked ? "disabled" : ""}>${this.renderStepConsistencyIssues(track, stepId)}${body}</fieldset>${locked ? "" : `<div class="form-save"><span>${icon("shield")} Änderungen bleiben lokal im Workspace.</span><button class="button button--primary" type="submit">${icon("check")} Schritt speichern</button></div>`}</form>${bottomSection}`;
  }

  protected renderGenerationTextFieldFacts(fields: TrackFields): string {
    const classification = fields.sunoContentClassification;
    const fieldValue = classification === null ? "NOT DOCUMENTED" : classification === "EMPTY" ? "NO" : "YES";
    const vocalLyricsValue =
      classification === null
        ? "NOT DOCUMENTED"
        : classification === "EMPTY"
          ? "N/A"
          : classification === "OTHER"
            ? "NOT DOCUMENTED"
            : classification === "VOCAL_LYRICS_ONLY" || classification === "MIXED"
              ? "YES"
              : "NO";
    const structureValue =
      classification === null
        ? "NOT DOCUMENTED"
        : classification === "EMPTY"
          ? "N/A"
          : classification === "OTHER"
            ? "NOT DOCUMENTED"
            : classification === "STRUCTURE_ONLY" || classification === "MIXED"
              ? "YES"
              : "NO";
    return `<div class="field-grid two-col generation-text-field-facts"><div class="read-only-field"><span>Generierungstextfeld verwendet</span><strong>${fieldValue}</strong></div><div class="read-only-field"><span>Vocal Lyrics vorhanden</span><strong>${vocalLyricsValue}</strong></div><div class="read-only-field"><span>Strukturanweisungen vorhanden</span><strong>${structureValue}</strong></div><div class="read-only-field"><span>Vokale Intention</span><strong>${escapeHtml(fields.vocalIntent ?? "NOT DOCUMENTED")}</strong></div></div>`;
  }

  protected hasRecordedAudioScreeningPlan(external: AudioScreeningExternalSummary): boolean {
    return (
      external.plannedRequestCount > 0 ||
      external.executedRequestCount > 0 ||
      external.samples.length > 0 ||
      Boolean(external.checkedAt) ||
      (external.status !== "not_run" && external.status !== "skipped_not_configured")
    );
  }

  protected screeningRunCalculationMode(external: AudioScreeningExternalSummary, isGerman: boolean): string {
    if (external.dynamicByTrackDuration) return isGerman ? "Dynamisch nach Tracklänge" : "Dynamic by track duration";
    return isGerman ? "Feste Referenzlänge" : "Fixed reference length";
  }

  protected screeningRunValue(number: number | undefined, unit = "ms"): string {
    return Number.isFinite(number) ? `${Math.round(number!)} ${unit}` : this.t("Nicht dokumentiert");
  }

  protected renderAudioScreeningSamples(external: AudioScreeningExternalSummary, isGerman: boolean): string {
    if (!external.samples.length) {
      return `<p class="screening-run-summary__empty">${isGerman ? "Für diesen Lauf wurden keine eindeutigen Samples ausgeführt." : "No unique samples were executed for this run."}</p>`;
    }
    return `<ol class="screening-sample-list">${external.samples
      .map((sample) => {
        const result = this.t(audioScreeningStatusLabel(sample.status));
        const sampleMatch = sample.status === "match_detected" || sample.matches.length > 0;
        const archive =
          sample.responseRelativePath || sample.responseSha256
            ? `<small>${escapeHtml(sample.responseRelativePath || this.t("Antwortarchiv nicht dokumentiert"))}${sample.responseSha256 ? ` · <code>${escapeHtml(sample.responseSha256)}</code>` : ""}</small>`
            : "";
        const matchTitles = sample.matches.length
          ? `<small>${isGerman ? "Treffer" : "Matches"}: ${escapeHtml(sample.matches.map((match) => match.title || (isGerman ? "Titel nicht dokumentiert" : "Title not documented")).join(", "))}</small>`
          : "";
        return `<li class="${sampleMatch ? "is-match" : ""}"><strong>${isGerman ? "Sample" : "Sample"} ${String(sample.sequence).padStart(2, "0")}</strong><span>${isGerman ? "Offset" : "Offset"} ${escapeHtml(this.screeningRunValue(sample.offsetMilliseconds))} · ${isGerman ? "Ende" : "End"} ${escapeHtml(this.screeningRunValue(sample.endOffsetMilliseconds))} · ${isGerman ? "Dauer" : "Duration"} ${escapeHtml(this.screeningRunValue(sample.durationMilliseconds))} · ${escapeHtml(result)}</span>${sample.message ? `<small>${escapeHtml(this.systemText(sample.message))}</small>` : ""}${matchTitles}${archive}</li>`;
      })
      .join("")}</ol>`;
  }

  protected renderAudioScreeningMetrics(
    external: AudioScreeningExternalSummary,
    isGerman: boolean,
    calculationMode: string,
    coverage: string
  ): string {
    return `<dl>
          <div><dt>${isGerman ? "Gewünschte Intensität" : "Requested intensity"}</dt><dd>${external.requestedIntensityPercent} %</dd></div>
          <div><dt>${isGerman ? "Berechnungsmodus" : "Calculation mode"}</dt><dd>${escapeHtml(calculationMode)}${!external.dynamicByTrackDuration ? ` · ${external.referenceDurationSeconds === null ? "N/A" : escapeHtml(formatScreeningSeconds(external.referenceDurationSeconds, this.language))}` : ""}</dd></div>
          <div><dt>${isGerman ? "Tatsächliche Trackdauer" : "Actual track duration"}</dt><dd>${escapeHtml(this.screeningRunValue(external.sourceDurationMilliseconds))}</dd></div>
          <div><dt>${isGerman ? "Zielprüfzeit" : "Target screening time"}</dt><dd>${escapeHtml(this.screeningRunValue(external.targetDurationMilliseconds))}</dd></div>
          <div><dt>${isGerman ? "Geplante Requests" : "Planned requests"}</dt><dd>${external.plannedRequestCount}</dd></div>
          <div><dt>${isGerman ? "Ausgeführte Requests" : "Executed requests"}</dt><dd>${external.executedRequestCount}</dd></div>
          <div><dt>${isGerman ? "Eindeutige Samples" : "Unique samples"}</dt><dd>${external.uniqueSampleCount}</dd></div>
          <div><dt>${isGerman ? "Doppelte Samples" : "Duplicate samples"}</dt><dd>${external.duplicateSampleCount}</dd></div>
          <div><dt>${isGerman ? "Überlappende Samples" : "Overlapping samples"}</dt><dd>${external.overlappingSampleCount}</dd></div>
          <div><dt>${isGerman ? "Eindeutig geprüfte Audiodauer" : "Unique sampled duration"}</dt><dd>${escapeHtml(this.screeningRunValue(external.uniqueSampleDurationMilliseconds))}</dd></div>
          <div><dt>${isGerman ? "Track-Abdeckung" : "Track coverage"}</dt><dd>${escapeHtml(coverage)}</dd></div>
          <div><dt>${isGerman ? "Providerstatus" : "Provider status"}</dt><dd>${escapeHtml(this.t(audioScreeningProviderStatusLabel(external.providerStatus)))}</dd></div>
        </dl>`;
  }

  protected renderAudioScreeningRunSummary(external: AudioScreeningExternalSummary, current: boolean): string {
    if (!current || !this.hasRecordedAudioScreeningPlan(external)) return "";
    const isGerman = this.language === "de";
    const mode = external.screeningMode === "multi_sample" ? "MULTI-SAMPLE" : "SINGLE-SAMPLE";
    const calculationMode = this.screeningRunCalculationMode(external, isGerman);
    const coverage = Number.isFinite(external.trackCoveragePercent)
      ? `${external.trackCoveragePercent.toLocaleString(this.language === "de" ? "de-DE" : "en-US", { maximumFractionDigits: 2 })} %`
      : this.t("Nicht dokumentiert");
    return `<section class="screening-run-summary">
        <div class="screening-run-summary__heading"><div><small>${isGerman ? "Dokumentierter Screening-Lauf" : "Recorded screening run"}</small><h5>${mode}</h5></div><strong>${external.executedRequestCount} ${isGerman ? "ausgeführt" : "executed"}</strong></div>
        ${this.renderAudioScreeningMetrics(external, isGerman, calculationMode, coverage)}
        ${external.sourceSha256 ? `<p class="screening-run-summary__source">${isGerman ? "Release SHA-256" : "Release SHA-256"}: <code>${escapeHtml(external.sourceSha256)}</code></p>` : ""}
        ${this.renderAudioScreeningSamples(external, isGerman)}
      </section>`;
  }

  protected preReleaseExternalAction(track: TrackDetail, locked: boolean, canRunLocal: boolean): string {
    const external = track.audioScreening.external;
    const settings = this.state.audioScreeningSettings;
    const localCurrent = localAudioScreeningIsCurrent(track.audioScreening.local, track.evidence);
    const providerReady = audioScreeningProviderIsReady(settings);
    const canRetryProvider =
      settings.enabled && settings.credentialsConfigured && settings.status !== "configuration_invalid";
    if (locked) return `<button class="button button--secondary" disabled>${icon("lock")} Snapshot geschützt</button>`;
    if (!localCurrent) {
      return `<button type="button" class="button button--secondary" data-action="run-local-audio-screening" ${!canRunLocal ? "disabled" : ""}>${icon("scan")} Zuerst lokale Prüfung</button>`;
    }
    if (providerReady || canRetryProvider) {
      return `<button type="button" class="button button--primary" data-action="run-external-audio-screening">${icon("upload")} ${external.status === "provider_unavailable" || external.status === "authentication_failed" ? "Erneut versuchen" : "ACRCloud-Prüfung starten"}</button>`;
    }
    return `<button type="button" class="button button--secondary" data-action="open-audio-screening-settings">${icon("settings")} Zu Einstellungen</button>`;
  }

  protected renderLocalAudioScreeningCard(
    track: TrackDetail,
    sourceLabel: string,
    locked: boolean,
    canRunLocal: boolean
  ): string {
    const local = visibleLocalAudioScreening(track.audioScreening.local, track.evidence);
    const localCurrent = localAudioScreeningIsCurrent(track.audioScreening.local, track.evidence);
    return `<article class="screening-summary ${audioScreeningStatusClass(local.status)}">
            <div class="screening-summary-head"><span>${icon("hash")}</span><div><small>Lokale Audio-Fingerprint-Prüfung</small><strong>Chromaprint</strong></div><b>${escapeHtml(audioScreeningStatusLabel(local.status))}</b></div>
            <p>${escapeHtml(this.systemText(local.message))}</p>
            <dl>
              <div><dt>Quelle</dt><dd>${escapeHtml(sourceLabel)}</dd></div>
              <div><dt>Source SHA-256</dt><dd>${local.sourceSha256 ? `<code>${escapeHtml(local.sourceSha256)}</code>` : "Nicht dokumentiert"}</dd></div>
              <div><dt>Audio-Dauer</dt><dd>${formatAudioDuration(local.durationMilliseconds, this.language)}</dd></div>
              <div><dt>Chromaprint</dt><dd>${escapeHtml([local.engine, local.engineVersion].filter(Boolean).join(" ") || "Nicht dokumentiert")}</dd></div>
            </dl>
            ${local.generatedAt ? `<small class="screening-checked-at">Erzeugt: ${formatDate(local.generatedAt, true, this.language)}</small>` : ""}
            <button type="button" class="button button--secondary" data-action="run-local-audio-screening" ${locked || !canRunLocal ? "disabled" : ""}>${icon("scan")} Lokale Prüfung ${localCurrent ? "erneut" : "starten"}</button>
          </article>`;
  }

  protected renderExternalAudioScreeningCard(track: TrackDetail, locked: boolean, canRunLocal: boolean): string {
    const external = track.audioScreening.external;
    const externalCurrent = externalAudioScreeningIsCurrent(external, track.evidence);
    const visibleExternal = visibleExternalAudioScreening(external, this.state.audioScreeningSettings, track.evidence);
    return `<article class="screening-summary ${audioScreeningStatusClass(visibleExternal.status)}">
            <div class="screening-summary-head"><span>${icon("tracks")}</span><div><small>Externe Katalogprüfung</small><strong>ACRCloud</strong></div><b>${escapeHtml(audioScreeningStatusLabel(visibleExternal.status))}</b></div>
            <p>${escapeHtml(this.systemText(visibleExternal.message))}</p>
            ${externalCurrent && external.checkedAt ? `<small class="screening-checked-at">Zuletzt geprüft: ${formatDate(external.checkedAt, true, this.language)}</small>` : ""}
            ${externalCurrent && visibleExternal.status === "match_detected" ? `<div class="screening-match-warning">${icon("alert")} Ein externer Anbieter hat eine Audio-Übereinstimmung gemeldet. Prüfe den Treffer vor Veröffentlichung.</div>` : ""}
            ${this.renderAudioScreeningRunSummary(external, externalCurrent)}
            <div class="screening-summary-actions">${this.preReleaseExternalAction(track, locked, canRunLocal)}</div>
          </article>`;
  }

  protected renderPreReleaseAudioScreening(track: TrackDetail): string {
    const local = visibleLocalAudioScreening(track.audioScreening.local, track.evidence);
    const release =
      track.evidence.find((item) => item.id === local.sourceEvidenceId) ??
      track.evidence.find((item) => item.role === "release_wav" && item.verified && Boolean(item.sha256));
    const sourceLabel = release?.fileName ?? local.sourceRelativePath ?? "Keine aktuelle Release-Datei";
    const canRunLocal = Boolean(
      track.evidence.some((item) => item.role === "release_wav" && item.verified && item.sha256)
    );
    const locked = isTrackContentLocked(track.status);
    return `<section class="pre-release-screening">
        <header><div><p class="overline">Pre-Release Audio Screening</p><h4>Technische Audio-Erkennung</h4><p>Die lokale Prüfung gehört zur aktuellen finalen Release-Datei. Die optionale externe Katalogprüfung ist kein Finalisierungsblocker.</p></div></header>
        <div class="screening-summary-grid">
          ${this.renderLocalAudioScreeningCard(track, sourceLabel, locked, canRunLocal)}
          ${this.renderExternalAudioScreeningCard(track, locked, canRunLocal)}
        </div>
        <p class="screening-disclaimer">${icon("info")} Das Audio-Screening ist ausschließlich ein technischer Vergleichsdatensatz und begründet keine Aussage zu Urheberschaft, Rechteinhaberschaft, Erlaubnis, Nichtverletzung, Rechtmäßigkeit oder Release-Freigabe.</p>
      </section>`;
  }

  protected renderEvidence(track: TrackDetail, embedded = false): string {
    const locked = isTrackContentLocked(track.status);
    const hasTermsEvidence = track.evidence.some((item) => item.role === "suno_terms_rights" && item.verified);
    const missing = calculateMissingRequirements(track, track.profileSnapshot).filter((item) => item.evidenceRole);
    return `<div class="${embedded ? "embedded-content" : "evidence-page"}">
        <div class="section-intro"><div><p class="overline">Lokale Nachweise</p><h3>Evidence & Lizenzen</h3><p>Originale bleiben am Quellort. Importierte Kopien werden gehasht und niemals still überschrieben.</p></div><button class="button button--primary" data-action="import-evidence" ${locked ? "disabled" : ""}>${icon("upload")} Evidence importieren</button></div>
        ${this.renderStepConsistencyIssues(track, "evidence_licenses")}
        ${
          missing.length
            ? `<div class="evidence-needed"><strong>${this.t(`${missing.length} erforderliche Nachweise fehlen`)}</strong><div>${missing
                .map((item) => {
                  if (item.evidenceRole === "subscription_payment")
                    return `<span class="evidence-reminder">${icon("info")} ${escapeHtml(this.t(`${item.label} – unten aus globaler Evidence zuordnen`))}</span>`;
                  if (item.evidenceRole === "suno_terms_rights")
                    return `<span class="evidence-reminder">${icon("info")} ${escapeHtml(this.t(`${item.label} – globale Datei unter Einstellungen registrieren`))}</span>`;
                  const current = [...track.evidence].reverse().find((evidence) => evidence.role === item.evidenceRole);
                  return `<button data-import-role="${item.evidenceRole}" ${current ? `data-replace-evidence="${escapeHtml(current.id)}"` : ""} ${locked ? "disabled" : ""}>${icon(current ? "upload" : "plus")}<span><strong>${escapeHtml(item.label)}</strong><small>${current ? "Vorhandene Datei sicher ersetzen" : escapeHtml(evidenceRoleLabel(item.evidenceRole!))}</small><small>${escapeHtml(this.t(`Gefordert: ${evidenceRoleFileTypes(item.evidenceRole!)}`))}</small></span></button>`;
                })
                .join("")}</div></div>`
            : ""
        }
        ${track.fields.commercialUseIntended ? this.renderGlobalEvidencePicker(track, locked) : ""}
        ${track.fields.commercialUseIntended ? this.renderGlobalTermsEvidencePicker(track, locked, hasTermsEvidence) : ""}
        <div class="panel evidence-table-panel"><div class="evidence-table-head"><span>Datei</span><span>Rolle</span><span>Integrität</span><span>Größe</span><span></span></div>
          ${track.evidence.length ? `<div class="evidence-list">${track.evidence.map((item) => this.renderEvidenceRow(item, locked)).join("")}</div>` : this.emptyState("file", "Noch keine Evidence", "Importiere echte lokale Dateien über den nativen Dateidialog.")}
        </div>
        <div class="deviation-section"><div class="section-intro compact"><div><p class="overline">Abweichungen</p><h3>Offene Hinweise & Blocker</h3></div><button class="button button--secondary" data-action="add-deviation" ${locked ? "disabled" : ""}>${icon("plus")} Abweichung erfassen</button></div>${this.renderDeviations(track, locked)}</div>
      </div>`;
  }

  protected renderEvidenceRow(item: EvidenceItem, locked: boolean): string {
    const provenance = this.t(evidenceProvenanceLabel(item.provenance));
    const role = this.t(evidenceRoleLabel(item.role));
    const verification = this.t(item.verified ? "Verifiziert" : "Nicht verifiziert");
    const verificationMarkup = item.verified
      ? `${icon("check")} ${escapeHtml(verification)}`
      : escapeHtml(verification);
    const previewLabel = this.t("Evidence-Vorschau öffnen");
    const verifyLabel = this.t("Evidence prüfen");
    const removeLabel = this.t("Evidence entfernen");
    const sunoTechnical =
      item.role === "suno_final_export" && item.metadata?.sunoStudioDetected
        ? `<small class="evidence-suno-meta">Suno Studio · ${escapeHtml(item.metadata.sunoCreatedTimestamp || this.t("Zeitstempel nicht dokumentiert"))}${item.metadata.sunoId ? ` · ID ${escapeHtml(item.metadata.sunoId)}` : ""}</small>`
        : "";
    return `<div class="evidence-row"><span class="file-icon">${icon("file")}</span><button class="evidence-name" data-preview-evidence="${escapeHtml(item.id)}" title="${escapeHtml(previewLabel)}"><strong>${escapeHtml(item.fileName)}</strong><small>${escapeHtml(item.relativePath)} · ${escapeHtml(provenance)}</small>${item.metadata?.documentTitle ? `<small>${escapeHtml(item.metadata.documentTitle)} · ${escapeHtml(item.metadata.provider)}</small>` : ""}${sunoTechnical}</button><span>${escapeHtml(role)}</span><span class="verification ${item.verified ? "is-valid" : ""}">${verificationMarkup}</span><span>${formatBytes(item.sizeBytes, this.language)}</span><span class="row-actions"><button class="icon-button" data-verify-evidence="${escapeHtml(item.id)}" aria-label="${escapeHtml(verifyLabel)}">${icon("shield")}</button><button class="icon-button danger" data-remove-evidence="${escapeHtml(item.id)}" aria-label="${escapeHtml(removeLabel)}" ${locked ? "disabled" : ""}>${icon("trash")}</button></span></div>`;
  }

  protected renderDeviations(track: TrackDetail, locked = false): string {
    const deviations = track.blockingDeviations ?? [];
    if (!deviations.length)
      return `<div class="neutral-message">${icon("check")}<div><strong>Keine Abweichungen erfasst.</strong><span>Ungeklärte blockierende Abweichungen verhindern die Finalisierung.</span></div></div>`;
    return `<div class="deviation-list">${deviations.map((item) => `<article class="deviation ${item.resolved ? "is-resolved" : ""}">${icon(item.resolved ? "check" : "alert")}<div><strong>${escapeHtml(item.title)}</strong><p>${escapeHtml(item.description)}</p><small>${item.resolved ? `Gelöst ${formatDate(item.resolvedAt, true, this.language)}` : `Erfasst ${formatDate(item.createdAt, true, this.language)}`}</small></div><div>${!item.resolved ? `<button class="button button--small button--secondary" data-resolve-deviation="${item.id}" ${locked ? "disabled" : ""}>Als gelöst markieren</button>` : ""}<button class="icon-button danger" data-remove-deviation="${item.id}" aria-label="Abweichung entfernen" ${locked ? "disabled" : ""}>${icon("trash")}</button></div></article>`).join("")}</div>`;
  }

  protected renderStepConsistencyIssues(track: TrackDetail, stepId: StepId): string {
    const issues = track.automation.consistencyIssues.filter((item) => item.stepId === stepId);
    if (!issues.length) return "";
    return `<div class="consistency-notices">${issues.map((item) => `<div class="danger-banner ${item.blocking ? "" : "is-warning"}">${icon(item.blocking ? "alert" : "info")}<div><strong>${item.blocking ? "Abweichung erkannt" : "Hinweis"}</strong><span>${escapeHtml(this.systemText(item.message))}</span></div></div>`).join("")}</div>`;
  }

  protected renderGlobalEvidencePicker(track: TrackDetail, locked = false): string {
    const subscriptions = this.state.globalEvidence.filter((item) => item.role === "subscription_payment");
    const attachedIds = new Set(
      track.evidence
        .filter((item) => item.role === "subscription_payment")
        .map((item) => item.sourceGlobalEvidenceId)
        .filter((value): value is string => Boolean(value))
    );
    const productionCoverage = subscriptionProductionCoverageStatus(track.evidence, track.fields);
    const generationCoverage = subscriptionGenerationCoverageStatus(track.evidence, track.fields);
    const productionCoverageLabel = this.t(productionCoverage.replace("_", " "));
    const generationCoverageLabel = this.t(generationCoverage.replace("_", " "));
    const productionCaption = this.t("Produktion:");
    const finalGenerationCaption = this.t("Finalgeneration:");
    const coverageIntroduction = this.t(
      "Beim Zuordnen kopiert der native Dienst den Nachweis in den Track-Ordner. Produktionszeitraum:"
    );
    const coverageNote = this.t(
      "Mehrere lückenlos anschließende Abrechnungszeiträume werden gemeinsam gewertet. Dies ist ausschließlich ein Datumsabgleich, keine Rechteaussage."
    );
    return `<section class="global-picker"><div><p class="overline">Global registriert</p><h4>Abo-Nachweis für Produktionszeitraum und Finalgeneration</h4><p>${coverageIntroduction} <strong>${productionCoverageLabel}</strong> · ${finalGenerationCaption} <strong>${generationCoverageLabel}</strong>. ${coverageNote}</p></div>${
      subscriptions.length
        ? `<div>${subscriptions
            .map((item) => {
              const coverage = subscriptionEvidenceRelevance(item, track.fields);
              const attached = attachedIds.has(item.id);
              const productionLabel = coverage.coversProduction
                ? "YES"
                : coverage.overlapsProduction
                  ? "TEILWEISE"
                  : "NO";
              const generationLabel = coverage.coversGeneration
                ? "YES"
                : track.fields.sunoFinalGenerationDate
                  ? "NO"
                  : "NOT VERIFIED";
              return `<article class="${coverage.relevant || attached ? "is-covering" : ""}">${icon("file")}<span><strong>${escapeHtml(item.fileName)}</strong><small>${formatDate(item.coverageStart, false, this.language)} – ${formatDate(item.coverageEnd, false, this.language)} · ${productionCaption} ${this.t(productionLabel)} · ${finalGenerationCaption} ${this.t(generationLabel)}</small></span><button class="button button--small button--secondary" data-attach-global="${item.id}" ${locked || attached || !coverage.relevant ? "disabled" : ""}>${attached ? "Zugeordnet" : coverage.relevant ? "Diesem Track zuordnen" : "Nicht passend"}</button></article>`;
            })
            .join("")}</div>`
        : `<p class="empty-inline">Noch keine globale Abo-Evidence. Registriere sie unter Einstellungen.</p>`
    }</section>`;
  }

  protected renderGlobalTermsEvidencePicker(track: TrackDetail, locked: boolean, hasTermsEvidence: boolean): string {
    const terms = this.state.globalEvidence.filter((item) => item.role === "suno_terms_rights");
    const attachedIds = new Set(
      track.evidence
        .filter((item) => item.role === "suno_terms_rights")
        .map((item) => item.sourceGlobalEvidenceId)
        .filter((value): value is string => Boolean(value))
    );
    return `<section class="global-picker"><div><p class="overline">Globale Service-Terms-Evidence</p><h4>Archivierte Suno-Nutzungsbedingungen</h4><p>Die Datei wird einmal unter Einstellungen lokal registriert und als gehashte portable Kopie in nicht finalisierte Projekte übernommen. SunoDM trifft keine Rechte- oder Gültigkeitsaussage.</p></div>
        ${
          terms.length
            ? `<div>${terms
                .map((item) => {
                  const attached = attachedIds.has(item.id);
                  const metadataComplete = termsMetadataComplete(item.metadata);
                  const provider = item.metadata?.provider?.trim() || this.t("Provider nicht dokumentiert");
                  const retrieval = formatDate(item.metadata?.retrievalDate, false, this.language);
                  return `<article class="${attached && metadataComplete ? "is-covering" : ""}">${icon("file")}<span><strong>${escapeHtml(item.metadata?.documentTitle || item.fileName)}</strong><small>${escapeHtml(this.t(`${provider} · Abruf ${retrieval}`))}</small><small>${metadataComplete ? "Kernmetadaten vollständig" : "Terms evidence exists, but descriptive metadata is incomplete."}</small></span><button class="button button--small button--secondary" data-attach-global="${item.id}" ${locked || attached ? "disabled" : ""}>${attached ? "Im Projekt hinterlegt" : "Diesem Projekt zuordnen"}</button></article>`;
                })
                .join("")}</div>`
            : `<p class="empty-inline">Noch keine globalen Suno-Nutzungsbedingungen. Registriere die Datei unter Einstellungen.</p>`
        }
        ${track.fields.sunoTermsEvidenceNotAvailable === true ? `<div class="danger-banner is-warning">${icon("info")}<div><strong>Historischer Status: Terms evidence not available</strong><span>Dieser Legacy-Wert bleibt erhalten, erfüllt Workflow 1.7 bei kommerzieller Nutzung aber nicht. Erforderlich sind lokale Evidence sowie Titel, Provider und Abrufdatum.${hasTermsEvidence ? " Eine Datei ist bereits vorhanden; prüfe ihre Metadaten." : ""}</span></div></div>` : ""}
      </section>`;
  }

  protected renderIntegrity(track: TrackDetail): string {
    const mismatches = track.integrity.mismatchFiles;
    const locked = isTrackContentLocked(track.status);
    return `<div class="integrity-page">
        <div class="integrity-hero ${track.integrity.verified && !mismatches.length ? "is-valid" : ""}"><span>${icon("shield")}</span><div><p class="overline">SHA-256 Integrität</p><h3>${track.integrity.verified ? "Dateien erfolgreich verifiziert" : "Integritätsprüfung ausstehend"}</h3><p>${this.t(`${track.integrity.fileCount} Dateien gehasht · ${track.integrity.verifiedCount} Dateien verifiziert`)}</p></div><strong>${track.integrity.verified && !mismatches.length ? "PASS" : track.integrity.generated ? "NICHT VERIFIZIERT" : "NICHT ERZEUGT"}</strong></div>
        ${mismatches.length ? `<div class="danger-banner">${icon("alert")}<div><strong>${this.t(`${mismatches.length} Integritätsabweichungen`)}</strong><span>${mismatches.map(escapeHtml).join(", ")}</span></div></div>` : ""}
        <div class="integrity-actions"><article class="action-card">${icon("file")}<div><h4>1. Dokumente erzeugen</h4><p>Versionierte Markdown- und Textdokumente aus den aktuellen Angaben erstellen.</p><span>${track.documents.current ? this.t(`Aktuell · ${formatDate(track.documents.generatedAt, true, this.language)}`) : "Ausstehend oder veraltet"}</span></div><button class="button button--secondary" data-action="generate-documents" ${locked ? "disabled" : ""}>Erzeugen</button></article>
        <article class="action-card">${icon("hash")}<div><h4>2. SHA-256 berechnen</h4><p>Alle relevanten Dateien in einer extern prüfbaren Hashliste erfassen.</p><span>${track.integrity.generated ? `${track.integrity.fileCount} Dateien` : "Ausstehend"}</span></div><button class="button button--secondary" data-action="calculate-hashes" ${locked || !track.documents.current ? "disabled" : ""}>Berechnen</button></article>
        <article class="action-card">${icon("shield")}<div><h4>3. Prüfsummen verifizieren</h4><p>Hashliste erneut lesen und jede erfasste Datei nativ überprüfen.</p><span>${track.integrity.verified ? formatDate(track.integrity.verifiedAt, true, this.language) : "Ausstehend"}</span></div><button class="button button--primary" data-action="verify-hashes" ${!track.integrity.generated ? "disabled" : ""}>Verifizieren</button></article></div>
        ${this.renderAudioScreeningIntegritySection(track)}
        <div class="technical-note">${icon("info")}<p><strong>Unabhängig prüfbar.</strong> SHA256SUMS.txt bleibt möglichst mit <code>sha256sum -c</code> kompatibel. Zertifikat, Archiv und interne Verwaltungsdaten werden nicht in dieselbe Hashliste aufgenommen.</p></div>
      </div>`;
  }

  protected renderAudioScreeningIntegritySection(track: TrackDetail): string {
    const local = visibleLocalAudioScreening(track.audioScreening.local, track.evidence);
    const external = track.audioScreening.external;
    const settings = this.state.audioScreeningSettings;
    const externalCurrent = externalAudioScreeningIsCurrent(external, track.evidence);
    const visibleExternal = visibleExternalAudioScreening(external, settings, track.evidence);
    const sample =
      externalCurrent &&
      external.sampleOffsetMilliseconds !== undefined &&
      external.sampleDurationMilliseconds !== undefined
        ? `${formatAudioTimestamp(external.sampleOffsetMilliseconds, this.language)}–${formatAudioTimestamp(external.sampleOffsetMilliseconds + external.sampleDurationMilliseconds, this.language)}`
        : this.t("Nicht dokumentiert");
    const matches = external.matches.length
      ? `<div class="screening-match-list">${external.matches.map((match) => `<article><strong>${escapeHtml(match.title || "Titel nicht dokumentiert")}</strong><span>${escapeHtml((match.artists ?? []).join(", ") || "Artist nicht dokumentiert")}${match.album ? ` · ${escapeHtml(match.album)}` : ""}${match.score !== undefined ? ` · Score ${escapeHtml(String(match.score))}` : ""}</span>${match.isrc || match.acrid ? `<small>${match.isrc ? `ISRC ${escapeHtml(match.isrc)}` : ""}${match.isrc && match.acrid ? " · " : ""}${match.acrid ? `ACRID ${escapeHtml(match.acrid)}` : ""}</small>` : ""}</article>`).join("")}</div>`
      : "";
    return `<section class="panel audio-screening-integrity-section">
        <div class="panel-heading"><div><p class="overline">Pre-Release Audio Screening</p><h3>Audio-Fingerprint</h3><p>Chromaprint ist ein akustischer Fingerprint und wird getrennt von der SHA-256-Dateiintegrität dargestellt. ACRCloud bleibt eine bewusste, optionale externe Prüfung.</p></div></div>
        <div class="audio-screening-integrity-grid">
          <article class="audio-screening-card ${audioScreeningStatusClass(local.status)}"><span>${icon("hash")}</span><div><small>Chromaprint · lokal</small><h4>${escapeHtml(this.t(audioScreeningStatusLabel(local.status)))}</h4><p>${escapeHtml(this.systemText(local.message))}</p>${local.generatedAt ? `<small>Erzeugt: ${formatDate(local.generatedAt, true, this.language)}</small>` : ""}</div></article>
          <article class="audio-screening-card ${audioScreeningStatusClass(visibleExternal.status)}"><span>${icon("tracks")}</span><div><small>ACRCloud · extern</small><h4>${escapeHtml(this.t(audioScreeningStatusLabel(visibleExternal.status)))}</h4><p>${escapeHtml(this.systemText(visibleExternal.message))}</p>${externalCurrent && external.checkedAt ? `<small>Geprüft: ${formatDate(external.checkedAt, true, this.language)} · Sample: ${sample}</small>` : ""}</div></article>
        </div>
        ${externalCurrent && visibleExternal.status === "match_detected" ? `<div class="audio-screening-alert">${icon("alert")}<div><strong>ACRCloud meldet eine Audio-Übereinstimmung</strong><span>Der externe Anbieter hat eine Audio-Übereinstimmung gemeldet. Prüfe den Treffer vor Veröffentlichung.</span></div></div>${matches}` : ""}
        ${this.renderAudioScreeningRunSummary(external, externalCurrent)}
        ${!settings.enabled || !settings.credentialsConfigured ? `<p class="audio-screening-configuration">Extern: ÜBERSPRUNGEN – kein ACRCloud-Zugang eingerichtet. Die lokale Chromaprint-Prüfung bleibt davon unabhängig.</p>` : ""}
        <p class="screening-disclaimer">${icon("info")} Das Audio-Screening ist ausschließlich ein technischer Vergleichsdatensatz und begründet keine Aussage zu Urheberschaft, Rechteinhaberschaft, Erlaubnis, Nichtverletzung, Rechtmäßigkeit oder Release-Freigabe.</p>
      </section>`;
  }
}
