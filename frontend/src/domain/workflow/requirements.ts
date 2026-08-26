import type { EvidenceItem, EvidenceRole, GlobalProfile, StepId, TrackDetail, TrackFields } from "../types";
import { contentCheckAllNegative, isAiArtwork } from "./conditional-fields";
import {
  documentationAnswerProvided,
  filenameRequirementMet,
  hasCoveringSubscriptionEvidence,
  hasCurrentLocalAudioScreening,
  hasDisclosedFinalArtwork,
  hasEvidence,
  hasSelections,
  hasText,
  isoDay,
  localDisclosureArtifacts,
  subscriptionGenerationCoverageStatus
} from "./evidence";

export interface MissingRequirement {
  id: string;
  stepId: StepId;
  label: string;
  evidenceRole?: EvidenceRole;
}

export interface RequirementEvaluation extends MissingRequirement {
  completed: boolean;
}

export type RequirementTrack = Pick<
  TrackDetail,
  "fields" | "evidence" | "documents" | "integrity" | "blockingDeviations" | "automation" | "audioScreening"
>;

type AddRequirement = (
  id: string,
  stepId: StepId,
  label: string,
  completed: boolean,
  evidenceRole?: EvidenceRole
) => void;

interface RequirementContext {
  track: RequirementTrack;
  profile: GlobalProfile;
  fields: TrackFields;
  evidence: EvidenceItem[];
  documents: RequirementTrack["documents"];
  integrity: RequirementTrack["integrity"];
  add: AddRequirement;
}

function addTrackRequirements({ fields, profile, add }: RequirementContext): void {
  add("title", "track", "Track-Titel", hasText(fields.title));
  add("production-start", "track", "Produktionsstart", hasText(fields.productionStartDate));
  add("production-end", "track", "Produktionsende", hasText(fields.productionEndDate));
  add("commercial-intent", "track", "Angabe zur kommerziellen Nutzung", true);
  add("profile-artist", "track", "Globaler Künstlername", hasText(profile.artistName));
  add(
    "profile-policy",
    "track",
    "Globale KI-Artwork-Transparenzrichtlinie",
    hasText(profile.artworkTransparencyPolicy)
  );
}

function addSourceRequirements({ fields, evidence, add }: RequirementContext): void {
  add("external-audio-answer", "source", "Angabe zu externem Audio", fields.externalAudioUploaded !== null);
  add("own-audio-answer", "source", "Angabe zu eigenem Audio", fields.ownAudioUploaded !== null);
  add("code-generation-answer", "source", "Angabe zur codebasierten Erzeugung", fields.codeBasedGeneration !== null);
  add("samples-answer", "source", "Angabe zu fremden Samples", fields.thirdPartySamplesUploaded !== null);
  if (fields.externalAudioUploaded === true) {
    add("external-source", "source", "Quelle des externen Audios", hasText(fields.externalAudioSource));
    add("external-ownership", "source", "Rechtezuordnung des externen Audios", hasText(fields.externalAudioOwnership));
    add(
      "external-license",
      "evidence_licenses",
      "Lizenznachweis für externes Audio",
      hasEvidence(evidence, "external_audio_license"),
      "external_audio_license"
    );
    add(
      "external-audio-file",
      "evidence_licenses",
      "Importierte externe Audiodatei",
      hasEvidence(evidence, "external_audio_file"),
      "external_audio_file"
    );
  }
  if (fields.ownAudioUploaded === true) {
    add("own-source", "source", "Quelle des eigenen Audios", hasText(fields.ownAudioSource));
    add("own-ownership", "source", "Rechtezuordnung des eigenen Audios", hasText(fields.ownAudioOwnership));
    add(
      "own-audio-file",
      "evidence_licenses",
      "Importierte eigene Audiodatei",
      hasEvidence(evidence, "own_audio_file"),
      "own_audio_file"
    );
  }
  if (fields.codeBasedGeneration === true) {
    add(
      "source-code-file",
      "evidence_licenses",
      "Quellcode oder Quelldatei der codebasierten Erzeugung",
      hasEvidence(evidence, "source_code_file"),
      "source_code_file"
    );
    add(
      "code-audio-post-processed-answer",
      "source",
      "Angabe zur Nachbearbeitung des codebasiert erzeugten Audios",
      fields.codeAudioPostProcessed !== null
    );
    if (fields.codeAudioPostProcessed === true) {
      add(
        "code-audio-post-processing-operations",
        "source",
        "Bestätigte Nachbearbeitungsschritte des codebasiert erzeugten Audios",
        hasSelections(fields.codeAudioPostProcessingOperations)
      );
    }
    add(
      "code-generated-audio-file",
      "evidence_licenses",
      "Mit dem Quellcode erzeugte WAV- oder MP3-Datei",
      hasEvidence(evidence, "code_generated_audio_file"),
      "code_generated_audio_file"
    );
  }
  if (fields.thirdPartySamplesUploaded === true) {
    add("sample-source", "source", "Quelle der fremden Samples", hasText(fields.thirdPartySampleSource));
    add("sample-ownership", "source", "Rechtezuordnung der fremden Samples", hasText(fields.thirdPartySampleOwnership));
    add(
      "sample-file",
      "evidence_licenses",
      "Importierte Sample-Datei",
      hasEvidence(evidence, "third_party_sample_file"),
      "third_party_sample_file"
    );
    add(
      "sample-license",
      "evidence_licenses",
      "Lizenznachweis der fremden Samples",
      hasEvidence(evidence, "third_party_sample_license"),
      "third_party_sample_license"
    );
  }
}

function addSunoRequirements({ fields, evidence, profile, add }: RequirementContext): void {
  add("suno-model", "suno", "Suno-Modell", hasText(fields.sunoModel));
  add("profile-suno-name", "suno", "Globaler Suno-Profilname", hasText(profile.sunoProfileName));
  add("profile-suno-handle", "suno", "Globaler Suno-Benutzername", hasText(profile.sunoHandle));
  add("profile-suno-plan", "suno", "Globaler Suno-Tarif", hasText(profile.sunoPlan));
  add(
    "profile-subscription-start",
    "suno",
    "Startdatum des globalen Suno-Abonnements",
    hasText(profile.subscriptionStartDate)
  );
  add("suno-url", "suno", "Suno-Projekt-URL", hasText(fields.sunoProjectUrl));
  add("suno-generation-date", "suno", "Datum der finalen Suno-Generation", hasText(fields.sunoFinalGenerationDate));
  add("suno-plan", "suno", "Suno-Tarif bei der finalen Generation", hasText(fields.sunoPlanAtGeneration));
  add(
    "suno-final-export",
    "suno",
    "Finaler Suno-Export",
    hasEvidence(evidence, "suno_final_export"),
    "suno_final_export"
  );
  add(
    "suno-filename",
    "suno",
    "Suno-Exportdateiname stimmt mit Titel überein oder Abweichung ist bestätigt",
    filenameRequirementMet(evidence, "suno_final_export", fields.title, fields.sunoExportFilenameDifferenceConfirmed),
    "suno_final_export"
  );
}

function addHumanWorkRequirements({ fields, add }: RequirementContext): void {
  add("instrumental-answer", "human_work", "Angabe: Instrumentaltrack", fields.instrumentalTrack !== null);
  add("final-audio-vocals", "human_work", "Angabe: Finales Audio enthält Gesang", fields.vocalLyricsPresent !== null);
  add("vocal-intent", "human_work", "Beabsichtigte Gesangsnutzung", fields.vocalIntent !== null);
  add(
    "suno-content-classification",
    "human_work",
    "Eindeutige Inhaltsklassifizierung des Suno Generation Text Field",
    fields.sunoContentClassification !== null
  );
  if (fields.sunoContentClassification !== null && fields.sunoContentClassification !== "EMPTY") {
    add(
      "suno-lyrics-content-source",
      "human_work",
      "Quelle des Inhalts im Suno-Lyrics-/Structure-Feld",
      fields.sunoLyricsContentSource !== null
    );
    add(
      "suno-lyrics-field-text",
      "human_work",
      "Exakter Inhalt des Suno-Lyrics-/Structure-Felds",
      hasText(fields.sunoLyricsFieldText)
    );
    if (fields.sunoContentClassification === "OTHER") {
      add(
        "suno-lyrics-other-content-type",
        "human_work",
        "Beschreibung des sonstigen Suno-Feldinhalts",
        hasText(fields.sunoLyricsOtherContentType)
      );
    }
  }
  add("suno-style-prompt", "human_work", "In Suno verwendeter Style-Prompt", hasText(fields.sunoStylePrompt));
  add(
    "human-editing-answer",
    "human_work",
    "Angabe zu menschlicher Bearbeitung",
    fields.humanEditingPerformed !== null
  );
  if (fields.humanEditingPerformed === true)
    add(
      "human-editing-details",
      "human_work",
      "Bestätigte menschliche Bearbeitungsschritte",
      hasText(fields.humanEditingDetails)
    );
  add(
    "post-editing-answer",
    "release",
    "Angabe zur Bearbeitung auf dem Desktop-PC",
    fields.postExportEditingPerformed !== null
  );
  if (fields.postExportEditingPerformed === true)
    add(
      "post-editing-details",
      "release",
      "Bearbeitungsschritte auf dem Desktop-PC",
      hasText(fields.postExportEditingDetails)
    );
}

function addArtworkRequirements({ fields, evidence, profile, add }: RequirementContext): void {
  add("artwork-origin", "artwork", "Entstehungsart des Artworks", Boolean(fields.artworkOrigin));
  if (fields.artworkOrigin === "ai_assisted") {
    add(
      "artwork-human-changes",
      "artwork",
      "Mindestens eine menschliche Änderung am KI-assistierten Artwork",
      hasSelections(fields.humanArtworkModifications)
    );
  }
  if (fields.artworkOrigin && fields.artworkOrigin !== "none") {
    add("real-person-answer", "artwork", "Content-Check: reale Person", fields.depictsRealPerson !== null);
    add("real-event-answer", "artwork", "Content-Check: reales Ereignis", fields.depictsRealEvent !== null);
    add("trademark-answer", "artwork", "Content-Check: Marke oder Logo", fields.containsTrademark !== null);
    if (fields.depictsRealPerson === true)
      add("real-person-notes", "artwork", "Notiz zur dargestellten realen Person", hasText(fields.realPersonNotes));
    if (fields.depictsRealEvent === true)
      add("real-event-notes", "artwork", "Notiz zum dargestellten realen Ereignis", hasText(fields.realEventNotes));
    if (fields.containsTrademark === true)
      add("trademark-notes", "artwork", "Notiz zur dargestellten Marke oder zum Logo", hasText(fields.trademarkNotes));
    const disclosureRequired = isAiArtwork(fields) && fields.disclosureApplied === true;
    const finalArtworkComplete = disclosureRequired
      ? hasDisclosedFinalArtwork(evidence, fields)
      : hasEvidence(evidence, "final_artwork");
    const finalArtworkLabel = disclosureRequired
      ? "Finales Artwork muss exakt die lokal gekennzeichnete Fassung sein"
      : "Finales, aus Suno heruntergeladenes Artwork";
    add("artwork-final", "artwork", finalArtworkLabel, finalArtworkComplete, "final_artwork");
  }

  if (isAiArtwork(fields)) {
    add(
      "ai-original",
      "artwork",
      "Unverändertes KI-Artwork",
      hasEvidence(evidence, "ai_artwork_original"),
      "ai_artwork_original"
    );
    if (!contentCheckAllNegative(fields)) {
      add("ai-service", "ai_transparency", "Verwendeter KI-Bilddienst", hasText(fields.aiImageService));
      add(
        "profile-ai-service",
        "ai_transparency",
        "Globaler Standarddienst für KI-Bilder",
        hasText(profile.defaultAiImageService)
      );
      add("ai-policy", "ai_transparency", "KI-Transparenzrichtlinie", hasText(profile.artworkTransparencyPolicy));
    }
    const hasDisclosureArtifact = localDisclosureArtifacts(evidence, fields).length > 0;
    const disclosureDecisionComplete =
      fields.disclosureApplied === false ||
      (fields.disclosureApplied === true && hasText(fields.disclosureText) && hasDisclosureArtifact);
    add(
      "ai-disclosure-decision",
      "ai_transparency",
      fields.disclosureApplied === true
        ? "Artwork-Hinweistext und lokal erzeugte Fassung"
        : "Explizite YES-/NO-Entscheidung zum Artwork-Hinweis",
      disclosureDecisionComplete
    );
  }
}

function addAiTransparencyRequirements({ fields, add }: RequirementContext): void {
  add(
    "generative-ai-answer",
    "ai_transparency",
    "Angabe zur Verwendung generativer KI im Audio",
    fields.generativeAiUsed !== null
  );
  if (fields.generativeAiUsed === true) {
    add("audio-ai-system", "ai_transparency", "Für das Audio verwendetes KI-System", hasText(fields.audioAiSystem));
    add(
      "ai-assisted-audio-elements",
      "ai_transparency",
      "Angabe zu KI-assistierten Audioelementen",
      documentationAnswerProvided(fields.aiAssistedAudioElements)
    );
    add(
      "ai-generated-audio-elements",
      "ai_transparency",
      "Angabe zu KI-generierten Audioelementen",
      documentationAnswerProvided(fields.aiGeneratedAudioElements)
    );
    add(
      "voice-imitation",
      "ai_transparency",
      "Angabe zur absichtlichen Imitation einer realen Stimme",
      documentationAnswerProvided(fields.realPersonVoiceIntentionallyImitated)
    );
    add(
      "identity-representation",
      "ai_transparency",
      "Angabe zur absichtlichen Darstellung der Identität einer realen Person",
      documentationAnswerProvided(fields.realPersonIdentityIntentionallyRepresented)
    );
    add(
      "authentic-event-recording",
      "ai_transparency",
      "Angabe zu einem realen Ereignis als authentische Aufnahme",
      documentationAnswerProvided(fields.realEventRepresentedAsAuthenticRecording)
    );
    add(
      "authentic-location-recording",
      "ai_transparency",
      "Angabe zu realem Ort, Institution oder Ereignis als authentische KI-Aufnahme",
      documentationAnswerProvided(fields.realLocationInstitutionEventPresentedAsAuthenticAiRecording)
    );
    const disclosureDecisionComplete =
      fields.audioDisclosureApplied !== null &&
      !(fields.commercialUseIntended && fields.audioDisclosureApplied === "not_documented");
    add(
      "audio-disclosure-status",
      "ai_transparency",
      "Audio-Disclosure-Entscheidung dokumentiert; bei kommerzieller Nutzung nicht 'Not documented'",
      disclosureDecisionComplete
    );
    if (fields.audioDisclosureApplied === "yes") {
      add(
        "audio-disclosure-locations",
        "ai_transparency",
        "Ort des Audio-Disclosures",
        hasSelections(fields.audioDisclosureLocations)
      );
      add(
        "audio-disclosure-text",
        "ai_transparency",
        "Text des Audio-Disclosures",
        hasText(fields.audioDisclosureText)
      );
    }
  }
}

function addReleaseRequirements({ track, fields, evidence, add }: RequirementContext): void {
  add("export-date", "release", "Datum der letzten Bearbeitung", hasText(fields.finalExportDate));
  add("release-wav", "release", "Finale Release-Audiodatei", hasEvidence(evidence, "release_wav"), "release_wav");
  add(
    "release-filename",
    "release",
    "Release-Dateiname stimmt mit Titel überein oder Abweichung ist bestätigt",
    filenameRequirementMet(evidence, "release_wav", fields.title, fields.releaseFilenameDifferenceConfirmed),
    "release_wav"
  );
  add(
    "local-audio-screening",
    "release",
    "Lokaler Chromaprint-Fingerprint für die aktuelle finale Release-Audiodatei",
    hasCurrentLocalAudioScreening(track)
  );
}

function addCommercialEvidenceRequirements({ fields, evidence, add }: RequirementContext): void {
  if (fields.commercialUseIntended) {
    add(
      "subscription-evidence",
      "evidence_licenses",
      "Abo-/Zahlungsnachweis für den Produktionszeitraum",
      hasCoveringSubscriptionEvidence(evidence, fields),
      "subscription_payment"
    );
    add(
      "subscription-generation-coverage",
      "evidence_licenses",
      "Abo-Nachweis deckt das Datum der finalen Generation ab",
      subscriptionGenerationCoverageStatus(evidence, fields) === "YES",
      "subscription_payment"
    );
    const verifiedTerms = evidence.filter((item) => hasEvidence([item], "suno_terms_rights"));
    const completeTerms = verifiedTerms.some(
      (item) =>
        hasText(item.metadata?.documentTitle ?? "") &&
        hasText(item.metadata?.provider ?? "") &&
        isoDay(item.metadata?.retrievalDate) !== null
    );
    add(
      "terms-evidence",
      "evidence_licenses",
      verifiedTerms.length
        ? "Terms evidence exists, but descriptive metadata is incomplete: document title, provider/source, and retrieval date are required."
        : "Verifizierter lokaler Suno-Terms-/Rights-Nachweis mit Titel, Provider und Abrufdatum",
      completeTerms && fields.sunoTermsEvidenceNotAvailable !== true,
      "suno_terms_rights"
    );
  }
}

function addIntegrityAndBlockingRequirements({ track, evidence, documents, integrity, add }: RequirementContext): void {
  add("documents-current", "integrity", "Aktuelle generierte Dokumente", documents.generated && documents.current);
  add(
    "hashes-verified",
    "integrity",
    "Vollständige SHA-256-Verifikation",
    integrity.verified && integrity.mismatchFiles.length === 0
  );
  for (const issue of track.automation.consistencyIssues.filter((item) => item.blocking)) {
    add(`consistency-${issue.code}`, issue.stepId, issue.message, false);
  }
  add(
    "blocking-deviations",
    "finalize",
    "Alle blockierenden Abweichungen gelöst",
    (track.blockingDeviations ?? []).every((item) => !item.blocking || item.resolved)
  );
  for (const item of evidence.filter((entry) => !entry.verified || !entry.sha256 || Boolean(entry.verificationError))) {
    add(
      `unverified-evidence-${item.id}`,
      "evidence_licenses",
      `Evidence fehlt oder ist nicht verifiziert: ${item.relativePath}`,
      false,
      item.role
    );
  }
}

export function evaluateRequirements(track: RequirementTrack, profile: GlobalProfile): RequirementEvaluation[] {
  const { fields, evidence, documents, integrity } = track;
  const requirements: RequirementEvaluation[] = [];
  const add: AddRequirement = (id, stepId, label, completed, evidenceRole) => {
    requirements.push({ id, stepId, label, evidenceRole, completed });
  };
  const context: RequirementContext = { track, profile, fields, evidence, documents, integrity, add };

  addTrackRequirements(context);
  addSourceRequirements(context);
  addSunoRequirements(context);
  addHumanWorkRequirements(context);
  addArtworkRequirements(context);
  addAiTransparencyRequirements(context);
  addReleaseRequirements(context);
  addCommercialEvidenceRequirements(context);
  addIntegrityAndBlockingRequirements(context);
  return requirements;
}
