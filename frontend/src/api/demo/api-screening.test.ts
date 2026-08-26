import { afterEach, describe, expect, it, vi } from "vitest";

import { createDemoApi } from "../demo";

async function settle<T>(promise: Promise<T>): Promise<T> {
  await vi.runAllTimersAsync();
  return promise;
}

async function expectTimedRejection(promise: Promise<unknown>, message: string): Promise<void> {
  const rejection = expect(promise).rejects.toThrow(message);
  await vi.runAllTimersAsync();
  await rejection;
}

afterEach(() => vi.useRealTimers());

describe("demo local audio screening", () => {
  it("requires release audio and records all simulated progress stages", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.runLocalAudioScreening("cosmic-pulse"), "autoritative finale Release-Audiodatei");

    const progress = vi.fn();
    const result = await settle(api.runLocalAudioScreening("gravity", progress));
    expect(progress).toHaveBeenCalledTimes(5);
    expect(progress.mock.calls.map(([entry]) => entry.stage)).toEqual([
      "preparing_audio",
      "fingerprinting_audio",
      "fingerprint_complete",
      "saving_screening_result",
      "complete"
    ]);
    expect(result.track?.audioScreening.local).toEqual(
      expect.objectContaining({
        status: "fingerprint_generated",
        engine: "Chromaprint",
        artifactRelativePath: "03_DOCUMENTATION/AUDIO_SCREENING/LOCAL_FINGERPRINT.json"
      })
    );
    expect(result.track?.documents.current).toBe(false);
    expect(result.track?.integrity.generated).toBe(false);
  });
});

describe("demo external audio screening configuration", () => {
  it("requires release audio and maps disabled configuration to a skipped result", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.runExternalAudioScreening("cosmic-pulse"), "autoritative finale Release-Audiodatei");
    const skipped = await settle(api.runExternalAudioScreening("gravity"));
    expect(skipped.track?.audioScreening.external).toEqual(
      expect.objectContaining({
        status: "skipped_not_configured",
        providerStatus: "disabled",
        matches: []
      })
    );
  });

  it("distinguishes invalid and credential-free provider settings", async () => {
    vi.useFakeTimers();
    const invalidApi = createDemoApi();
    await settle(invalidApi.openWorkspace());
    const invalidSettings = await settle(invalidApi.getAudioScreeningSettings());
    await settle(invalidApi.updateAudioScreeningSettings({ ...invalidSettings, enabled: true, host: "" }));
    const invalid = await settle(invalidApi.runExternalAudioScreening("gravity"));
    expect(invalid.track?.audioScreening.external).toEqual(
      expect.objectContaining({ status: "configuration_invalid", providerStatus: "configuration_invalid" })
    );

    const missingCredentialsApi = createDemoApi();
    await settle(missingCredentialsApi.openWorkspace());
    const settings = await settle(missingCredentialsApi.getAudioScreeningSettings());
    await settle(
      missingCredentialsApi.updateAudioScreeningSettings({
        ...settings,
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com"
      })
    );
    const missingCredentials = await settle(missingCredentialsApi.runExternalAudioScreening("gravity"));
    expect(missingCredentials.track?.audioScreening.external).toEqual(
      expect.objectContaining({ status: "skipped_not_configured", providerStatus: "not_configured" })
    );
  });
});
