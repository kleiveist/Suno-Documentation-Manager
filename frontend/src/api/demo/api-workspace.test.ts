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

describe("demo workspace lifecycle", () => {
  it("exposes the workflow and scans only an open workspace", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();

    const workflow = await api.getWorkflow();
    expect(workflow.steps).toHaveLength(10);
    expect(workflow.steps.map((step) => step.number)).toEqual([
      "01",
      "02",
      "03",
      "04",
      "05",
      "06",
      "07",
      "08",
      "09",
      "10"
    ]);
    await expectTimedRejection(api.scanWorkspace(), "Öffne zuerst einen Workspace");

    expect((await settle(api.createWorkspace()))!.trackCount).toBe(2);
    expect(await settle(api.scanWorkspace())).toEqual(
      expect.objectContaining({ discovered: 2, indexed: 2, unchanged: 2, warnings: [] })
    );
  });

  it("restores the demo workspace and retains its initialized tracks", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();

    const restored = await settle(api.restoreWorkspace("ignored/demo/path"));
    expect(restored).toEqual(expect.objectContaining({ id: "demo-workspace", trackCount: 2 }));
    expect(await settle(api.listTracks())).toHaveLength(2);
    expect((await settle(api.openWorkspace()))!.trackCount).toBe(2);
  });
});

describe("demo provider settings", () => {
  it("updates timestamp secrets and reports disabled and authenticated provider states", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    const disabled = await settle(api.testTimestampProvider());
    expect(disabled.status).toBe("disabled");

    const settings = await settle(api.getTimestampSettings());
    await settle(
      api.updateTimestampSettings({
        ...settings,
        enabled: true,
        provider: "custom_rfc3161",
        custom: {
          ...settings.custom,
          providerName: "Demo TSA",
          endpoint: "https://tsa.example.test",
          authenticationMode: "bearer_token",
          caCertificatePath: "/demo/tsa.pem"
        }
      })
    );
    expect((await settle(api.testTimestampProvider())).status).toBe("authentication_required");
    await settle(api.updateTimestampSecret(" demo-secret "));
    const ready = await settle(api.testTimestampProvider());
    expect(ready).toEqual(expect.objectContaining({ status: "ready", message: expect.stringContaining("RFC 3161") }));
    await settle(api.updateTimestampSecret(null));
    expect((await settle(api.getTimestampSettings())).status).toBe("authentication_required");
  });

  it("stores audio credentials separately and simulates provider unavailability", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    const settings = await settle(api.getAudioScreeningSettings());
    expect((await settle(api.testAudioScreeningProvider())).status).toBe("disabled");

    await settle(
      api.updateAudioScreeningSettings({
        ...settings,
        enabled: true,
        host: "identify-eu-west-1.acrcloud.com"
      })
    );
    await settle(api.updateAudioScreeningSecret({ accessKey: " key " }));
    expect((await settle(api.getAudioScreeningSettings())).credentialsConfigured).toBe(false);
    await settle(api.updateAudioScreeningSecret({ accessSecret: " secret " }));
    const tested = await settle(api.testAudioScreeningProvider());
    expect(tested).toEqual(
      expect.objectContaining({ status: "provider_unavailable", message: expect.stringContaining("Browser demo") })
    );
  });
});

describe("demo global evidence", () => {
  it("imports, lists, attaches, and removes subscription evidence", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    const imported = await settle(api.importGlobalEvidence("subscription_payment", "2026-07-01", "monthly"));
    expect(imported!.coverageEnd).toBe("2026-07-31");
    expect(await settle(api.listGlobalEvidence())).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: imported!.id })])
    );

    const attached = await settle(api.attachGlobalEvidence("gravity", imported!.id));
    expect(attached.evidence.some((item) => item.sourceGlobalEvidenceId === imported!.id)).toBe(true);
    await settle(api.removeGlobalEvidence(imported!.id));
    expect((await settle(api.listGlobalEvidence())).some((item) => item.id === imported!.id)).toBe(false);
    await expectTimedRejection(api.attachGlobalEvidence("gravity", "missing"), "globale Nachweis wurde nicht gefunden");
  });

  it("validates global Terms metadata updates", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(
      api.updateGlobalTermsEvidenceMetadata("missing", { documentTitle: "Suno Terms" }),
      "Terms-Nachweis wurde nicht gefunden"
    );
    const terms = await settle(
      api.importGlobalTermsEvidence({
        documentTitle: "Suno Terms",
        provider: "Suno",
        retrievalDate: "2026-08-20"
      })
    );
    await expectTimedRejection(
      api.updateGlobalTermsEvidenceMetadata(terms!.id, { documentTitle: "" }),
      "Dokumenttitel, Anbieter und Abrufdatum"
    );
  });
});
