import { subscriptionCoverageEnd } from "../domain/subscription";
import type {
  EvidenceMetadata,
  FolderImportProposal,
  GlobalProfile,
  SubscriptionBillingCycle,
  TrackLibraryAssignment
} from "../domain/types";
import { formatDate } from "../ui/format";
import { AppClickEvents } from "./class-click-events";
import { missingProfileFields } from "./navigation";
import { termsMetadataComplete } from "./timestamp-presentation";

export abstract class AppSubmitEvents extends AppClickEvents {
  protected abstract readTrackLibraryAssignment(form: HTMLFormElement): TrackLibraryAssignment | null;

  protected rejectNewTrackForMissingProfile(): boolean {
    const profileMissing = missingProfileFields(this.state.profile);
    if (!profileMissing.length) return false;
    this.state.showNewTrack = false;
    this.state.view = "settings";
    this.state.settingsCategory = "global";
    this.showToast(
      "error",
      "Track nicht angelegt",
      this.t(`Vervollständige zuerst: ${this.localizedLabels(profileMissing)}.`)
    );
    this.render();
    return true;
  }

  protected async submitFolderImportForm(form: HTMLFormElement, proposal: FolderImportProposal): Promise<void> {
    const data = new FormData(form);
    const singleTrackLibrary = proposal.kind === "single" ? this.readTrackLibraryAssignment(form) : null;
    if (proposal.kind === "single" && !singleTrackLibrary) return;
    const tracks = await this.withBusy("Ordner wird in normale Track-Strukturen übernommen …", () =>
      this.api.executeFolderImport({
        sourcePath: proposal.sourcePath,
        expectedKind: proposal.kind,
        singleTrackTitle: proposal.kind === "single" ? String(data.get("title") ?? "") : undefined,
        singleTrackLibrary: singleTrackLibrary ?? undefined,
        productionStartDate: proposal.kind === "single" ? String(data.get("productionStartDate") ?? "") : "",
        commercialUseIntended: data.get("commercialUseIntended") === "on"
      })
    );
    if (!tracks?.length) return;
    this.applyTrack(tracks[0]);
    this.state.tracks = await this.api.listTracks();
    this.state.albums = await this.api.listAlbums();
    this.state.showNewTrack = false;
    this.state.folderImport = null;
    this.state.view = "current";
    this.state.activeStep = "track";
    this.showToast(
      "success",
      "Ordner importiert",
      `${tracks.length} ${tracks.length === 1 ? "Track wurde" : "Tracks wurden"} als unvollständige normale SunoDM-Struktur angelegt.`
    );
    this.render();
  }

  protected async submitNewTrackForm(form: HTMLFormElement): Promise<void> {
    if (this.rejectNewTrackForMissingProfile()) return;
    const proposal = this.state.folderImport;
    if (proposal) {
      await this.submitFolderImportForm(form, proposal);
      return;
    }
    const data = new FormData(form);
    const library = this.readTrackLibraryAssignment(form);
    if (!library) return;
    const track = await this.withBusy("Track-Struktur wird angelegt …", () =>
      this.api.createTrack({
        title: String(data.get("title") ?? ""),
        productionStartDate: String(data.get("productionStartDate") ?? ""),
        commercialUseIntended: data.get("commercialUseIntended") === "on",
        library
      })
    );
    if (!track) return;
    this.applyTrack(track);
    this.state.showNewTrack = false;
    this.state.view = "current";
    this.state.activeStep = "track";
    this.showToast("success", "Track angelegt", "Die portable Track-Struktur wurde lokal erstellt.");
    this.render();
  }

  protected async submitTrackLibraryForm(form: HTMLFormElement): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    const library = this.readTrackLibraryAssignment(form);
    if (!library) return;
    const updated = await this.withBusy("Track-Ordner wird verschoben …", () =>
      this.api.updateTrackLibrary(this.requireTrack().id, library)
    );
    if (!updated) return;
    this.applyTrack(updated);
    this.state.showTrackLibrary = false;
    this.showToast("success", "Ordnerstruktur aktualisiert", `Der Track liegt jetzt unter ${updated.relativePath}.`);
    this.render();
  }

  protected async submitSubscriptionEvidenceForm(form: HTMLFormElement): Promise<void> {
    const data = new FormData(form);
    const coverageStart = String(data.get("coverageStart") ?? "");
    const rawBillingCycle = String(data.get("billingCycle") ?? "");
    if (rawBillingCycle !== "monthly" && rawBillingCycle !== "annual") {
      this.showToast("error", "Bezahlrhythmus fehlt", "Wähle monatliche oder jährliche Zahlung aus.");
      this.render();
      return;
    }
    const billingCycle: SubscriptionBillingCycle = rawBillingCycle;
    const coverageEnd = subscriptionCoverageEnd(coverageStart, billingCycle);
    if (!coverageEnd) {
      this.showToast("error", "Startdatum ungültig", "Gib den Beginn des auf der Rechnung abgedeckten Zeitraums an.");
      this.render();
      return;
    }
    const imported = await this.withBusy("Abo-Nachweis wird registriert …", async () => {
      const item = await this.api.importGlobalEvidence(
        "subscription_payment",
        coverageStart,
        billingCycle,
        this.language
      );
      if (!item) return null;
      return { item, globalEvidence: await this.api.listGlobalEvidence() };
    });
    if (!imported) return;
    this.state.globalEvidence = imported.globalEvidence;
    this.state.showSubscriptionEvidence = false;
    this.showToast(
      "success",
      "Abo-Nachweis registriert",
      `Abgedeckter Zeitraum: ${formatDate(coverageStart, false, this.language)} – ${formatDate(imported.item.coverageEnd ?? coverageEnd, false, this.language)}.`
    );
    this.render();
  }

  protected termsEvidenceMetadata(form: HTMLFormElement): Partial<EvidenceMetadata> {
    const data = new FormData(form);
    return {
      documentTitle: String(data.get("documentTitle") ?? "").trim(),
      provider: String(data.get("provider") ?? "").trim(),
      retrievalDate: String(data.get("retrievalDate") ?? "").trim(),
      sourceUrl: String(data.get("sourceUrl") ?? "").trim(),
      effectiveDate: String(data.get("effectiveDate") ?? "").trim(),
      applicableProductionPeriod: String(data.get("applicableProductionPeriod") ?? "").trim(),
      factualNote: String(data.get("factualNote") ?? "").trim()
    };
  }

  protected async submitTermsMetadataForm(form: HTMLFormElement): Promise<void> {
    const dialog = this.state.termsMetadataDialog;
    if (!dialog) return;
    const metadata = this.termsEvidenceMetadata(form);
    if (!termsMetadataComplete(metadata)) {
      this.showToast(
        "error",
        "Terms-Metadaten unvollständig",
        "Dokumenttitel, Provider/Source und Abrufdatum sind erforderlich."
      );
      this.render();
      return;
    }
    const currentTrackId = this.state.track?.id;
    const outcome = await this.withBusy(
      dialog.evidenceId ? "Terms-Metadaten werden aktualisiert …" : "Nutzungsbedingungen auswählen und registrieren …",
      async () => {
        const item = dialog.evidenceId
          ? await this.api.updateGlobalTermsEvidenceMetadata(dialog.evidenceId, metadata)
          : await this.api.importGlobalTermsEvidence(metadata, this.language);
        if (!item) return null;
        return {
          globalEvidence: await this.api.listGlobalEvidence(),
          track: currentTrackId ? await this.api.loadTrack(currentTrackId) : null
        };
      }
    );
    if (!outcome) return;
    this.state.globalEvidence = outcome.globalEvidence;
    if (outcome.track) this.applyTrack(outcome.track);
    this.state.termsMetadataDialog = null;
    await this.refreshTracks();
    this.showToast(
      "success",
      dialog.evidenceId ? "Terms-Metadaten aktualisiert" : "Globale Nutzungsbedingungen registriert",
      "Bearbeitbare Projektkopien wurden aktualisiert; finalisierte Snapshots bleiben unverändert."
    );
    this.render();
  }

  protected profileFrom(form: HTMLFormElement): GlobalProfile {
    const data = new FormData(form);
    return {
      artistName: String(data.get("artistName") ?? ""),
      sunoProfileName: String(data.get("sunoProfileName") ?? ""),
      sunoHandle: String(data.get("sunoHandle") ?? ""),
      sunoPlan: String(data.get("sunoPlan") ?? ""),
      subscriptionStartDate: String(data.get("subscriptionStartDate") ?? ""),
      defaultCommercialUse: data.get("defaultCommercialUse") === "true",
      defaultAiImageService: String(data.get("defaultAiImageService") ?? ""),
      artworkTransparencyPolicy: String(
        data.get("artworkTransparencyPolicy")
      ) as GlobalProfile["artworkTransparencyPolicy"],
      disclosureText: String(data.get("disclosureText") ?? "AI-assisted"),
      certificateLanguage: data.get("certificateLanguage") === "de" ? "de" : "en"
    };
  }

  protected async submitProfileForm(form: HTMLFormElement): Promise<void> {
    const profile = this.profileFrom(form);
    const saved = await this.withBusy("Einstellungen werden gespeichert …", async () => ({
      profile: await this.api.updateProfile(profile),
      timestamp: await this.updateTimestampSettingsFromForm(form),
      audioScreening: await this.updateAudioScreeningSettingsFromForm(form)
    }));
    if (!saved) return;
    this.state.profile = saved.profile;
    this.state.timestampSettings = saved.timestamp.settings;
    this.state.timestampProviderTest = null;
    this.state.audioScreeningSettings = saved.audioScreening.settings;
    this.state.audioScreeningProviderTest = null;
    await this.refreshTracks();
    this.showToast(
      "success",
      "Einstellungen gespeichert",
      "Offene Tracks wurden aktualisiert; finalisierte Track-Snapshots bleiben unverändert."
    );
  }

  protected async submitTrackStepForm(form: HTMLFormElement): Promise<void> {
    if (this.rejectLockedContentMutation()) return;
    const emptyRequiredChoice = [...form.querySelectorAll<HTMLElement>("[data-multi-choice-required]")].find(
      (group) => !group.querySelector("input[data-multi-choice]:checked")
    );
    if (emptyRequiredChoice) {
      this.showToast("error", "Auswahl fehlt", "Wähle mindestens einen tatsächlich ausgeführten Schritt aus.");
      return;
    }
    await this.saveTrackDraft();
  }

  protected async handleSubmit(event: SubmitEvent): Promise<void> {
    const form = event.target as HTMLFormElement;
    event.preventDefault();
    switch (form.id) {
      case "new-track-form":
        await this.submitNewTrackForm(form);
        return;
      case "track-library-form":
        await this.submitTrackLibraryForm(form);
        return;
      case "subscription-evidence-form":
        await this.submitSubscriptionEvidenceForm(form);
        return;
      case "terms-metadata-form":
        await this.submitTermsMetadataForm(form);
        return;
      case "profile-form":
        await this.submitProfileForm(form);
        return;
      case "track-step-form":
        await this.submitTrackStepForm(form);
    }
  }
}
