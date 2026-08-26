import { subscriptionCoverageEnd } from "../domain/subscription";
import { trackLibraryAssignment } from "../domain/track-library";
import type { SubscriptionBillingCycle, TrackFields, TrackLibraryAssignment } from "../domain/types";
import { serializeMultiChoiceValue } from "./choices";
import { AppSubmitEvents } from "./class-submit-events";
import { isTrackContentLocked } from "./state";

export abstract class AppFormEvents extends AppSubmitEvents {
  protected handleProfileConfigurationChange(
    input: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
  ): boolean {
    const profileForm = input.closest<HTMLFormElement>("#profile-form");
    if (!profileForm) return false;
    if (input.name === "timestampProvider" || input.name === "timestampAuthenticationMode") {
      this.syncTimestampProviderConfiguration(profileForm);
      return true;
    }
    const intensityFields = [
      "audioScreeningIntensityPercent",
      "audioScreeningDynamicByTrackDuration",
      "audioScreeningReferenceDurationMinutes"
    ];
    if (!intensityFields.includes(input.name)) return false;
    this.syncAudioScreeningIntensityPresentation(profileForm);
    return true;
  }

  protected handleAuxiliaryFormChange(input: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement): boolean {
    const libraryForm = input.closest<HTMLFormElement>("#new-track-form, #track-library-form");
    if (libraryForm && input.name === "librarySection") {
      this.syncTrackLibraryFields(libraryForm);
      return true;
    }
    const subscriptionForm = input.closest<HTMLFormElement>("#subscription-evidence-form");
    if (!subscriptionForm) return false;
    this.updateSubscriptionEvidencePreview(subscriptionForm);
    return true;
  }

  protected restoreLockedTrackDraft(): boolean {
    if (!this.state.track || !isTrackContentLocked(this.state.track.status)) return false;
    this.state.trackDraft = structuredClone(this.state.track.fields);
    this.draftDirty = false;
    return true;
  }

  protected handleMultiChoiceChange(input: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement): boolean {
    if (!input.matches("[data-multi-choice]")) return false;
    const group = input.closest<HTMLElement>("[data-multi-choice-group]");
    const checked = [...(group?.querySelectorAll<HTMLInputElement>("input[data-multi-choice]:checked") ?? [])].map(
      (item) => item.value
    );
    const storesArray = group?.hasAttribute("data-choice-array") ?? false;
    (this.state.trackDraft as unknown as Record<string, unknown>)[input.name] = storesArray
      ? checked
      : serializeMultiChoiceValue(checked);
    this.draftDirty = true;
    if (storesArray) this.render();
    return true;
  }

  protected trackDraftInputValue(
    input: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
  ): string | boolean | null {
    if (!(input instanceof HTMLInputElement)) return input.value;
    if (input.matches("[data-documentation-boolean]")) {
      if (input.value === "yes") return true;
      if (input.value === "no") return false;
      return null;
    }
    if (input.type === "radio" && (input.value === "true" || input.value === "false")) {
      return input.value === "true";
    }
    return input.value;
  }

  protected handleTrackDraftChange(input: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement): void {
    if (!input.closest("#track-step-form") || !this.state.trackDraft) return;
    if (this.restoreLockedTrackDraft()) return;
    if (this.handleMultiChoiceChange(input)) return;
    const key = input.name as keyof TrackFields;
    if (!key) return;
    (this.state.trackDraft as unknown as Record<string, unknown>)[key] = this.trackDraftInputValue(input);
    this.draftDirty = true;
    this.render();
  }

  protected handleChange(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    if (this.handleProfileConfigurationChange(input)) return;
    if (this.handleAuxiliaryFormChange(input)) return;
    this.handleTrackDraftChange(input);
  }

  protected handleInput(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLTextAreaElement;
    if (input.name === "albumTitle" && input.closest("#new-track-form, #track-library-form")) {
      input.setCustomValidity("");
      return;
    }
    const audioScreeningForm = input.closest<HTMLFormElement>("#profile-form");
    if (
      audioScreeningForm &&
      ["audioScreeningIntensityPercent", "audioScreeningReferenceDurationMinutes"].includes(input.name)
    ) {
      this.syncAudioScreeningIntensityPresentation(audioScreeningForm);
      return;
    }
    const subscriptionForm = input.closest<HTMLFormElement>("#subscription-evidence-form");
    if (subscriptionForm) {
      this.updateSubscriptionEvidencePreview(subscriptionForm);
      return;
    }
    if (input.matches("[data-track-search]")) {
      this.state.query = input.value;
      this.render();
      const next = this.root.querySelector<HTMLInputElement>("[data-track-search]");
      next?.focus();
      next?.setSelectionRange(next.value.length, next.value.length);
      return;
    }
    if (input.closest("#track-step-form") && this.state.trackDraft && input.name) {
      if (this.state.track && isTrackContentLocked(this.state.track.status)) {
        this.state.trackDraft = structuredClone(this.state.track.fields);
        this.draftDirty = false;
        return;
      }
      (this.state.trackDraft as unknown as Record<string, unknown>)[input.name] = input.value;
      this.draftDirty = true;
    }
  }

  protected syncTrackLibraryFields(form: HTMLFormElement): void {
    const albumSelected =
      form.querySelector<HTMLInputElement>('input[name="librarySection"]:checked')?.value === "album";
    const field = form.querySelector<HTMLElement>("[data-library-album-field]");
    const input = form.elements.namedItem("albumTitle") as HTMLInputElement | null;
    if (!field || !input) return;
    field.hidden = !albumSelected;
    input.disabled = !albumSelected;
    input.required = albumSelected;
    input.setCustomValidity("");
    if (albumSelected) input.focus();
  }

  protected readTrackLibraryAssignment(form: HTMLFormElement): TrackLibraryAssignment | null {
    const section = form.querySelector<HTMLInputElement>('input[name="librarySection"]:checked')?.value ?? "";
    const albumInput = form.elements.namedItem("albumTitle") as HTMLInputElement | null;
    const library = trackLibraryAssignment(section, albumInput?.value ?? "");
    if (library) return library;
    if (albumInput) {
      const field = form.querySelector<HTMLElement>("[data-library-album-field]");
      field?.removeAttribute("hidden");
      albumInput.disabled = false;
      albumInput.required = true;
      albumInput.setCustomValidity(
        this.t(
          albumInput.value.trim()
            ? "Der Albumtitel darf höchstens 200 Zeichen und keine Pfadtrenner, Steuerzeichen oder reservierten Ordnernamen enthalten."
            : "Gib für einen Album-Track einen Albumtitel an."
        )
      );
      albumInput.reportValidity();
      albumInput.focus();
    }
    return null;
  }

  protected updateSubscriptionEvidencePreview(form: HTMLFormElement): void {
    const coverageStart = form.elements.namedItem("coverageStart") as HTMLInputElement | null;
    const selectedCycle = form.querySelector<HTMLInputElement>('input[name="billingCycle"]:checked');
    const coverageEnd = form.elements.namedItem("coverageEnd") as HTMLInputElement | null;
    if (!coverageStart || !selectedCycle || !coverageEnd) return;
    const cycle = selectedCycle.value as SubscriptionBillingCycle;
    coverageEnd.value = subscriptionCoverageEnd(coverageStart.value, cycle) ?? "";
  }
}
