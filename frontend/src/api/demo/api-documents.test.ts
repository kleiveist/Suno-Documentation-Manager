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

describe("demo document generation", () => {
  it("reports deterministic document progress and resets prior integrity", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const progress = vi.fn();

    const generated = await settle(api.generateDocuments("gravity", false, progress));
    expect(progress).toHaveBeenCalledTimes(11);
    expect(progress.mock.calls.at(0)?.[0].stage).toBe("preparing_documents");
    expect(progress.mock.calls.at(-1)?.[0].stage).toBe("complete");
    expect(generated.track?.documents).toEqual(
      expect.objectContaining({ generated: true, current: true, templateVersion: "1.11" })
    );
    expect(generated.track?.documents.files).toHaveLength(8);
    expect(generated.track?.integrity.generated).toBe(false);
  });

  it("requires current documents and generated hashes in the correct order", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.calculateHashes("cosmic-pulse"), "aktuellen Dokumente");
    await expectTimedRejection(api.verifyHashes("cosmic-pulse"), "zuerst SHA-256-Prüfsummen");
  });
});

describe("demo artwork disclosure", () => {
  it("requires original AI artwork", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());

    await expectTimedRejection(api.generateArtworkDisclosure("cosmic-pulse", "AI-assisted"), "KI-Artwork");
  });

  it("creates one derived disclosure artifact and reuses it on subsequent calls", async () => {
    vi.useFakeTimers();
    const api = createDemoApi();
    await settle(api.openWorkspace());
    const imported = await settle(api.importEvidence("cosmic-pulse", "ai_artwork_original"));
    expect(imported?.evidence.some((item) => item.role === "ai_artwork_original")).toBe(true);

    const first = await settle(api.generateArtworkDisclosure("cosmic-pulse", "Visible AI notice"));
    const generated = first.track?.evidence.filter((item) => item.role === "ai_artwork_edited") ?? [];
    expect(generated).toHaveLength(1);
    expect(generated[0]).toEqual(
      expect.objectContaining({
        provenance: "generated_disclosure",
        generatedDisclosureText: "Visible AI notice"
      })
    );

    const second = await settle(api.generateArtworkDisclosure("cosmic-pulse", ""));
    expect(second.track?.evidence.filter((item) => item.role === "ai_artwork_edited")).toHaveLength(1);
    expect(second.track?.fields.disclosureText).toBe("Visible AI notice");
  });
});
