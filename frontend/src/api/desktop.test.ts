import { describe, expect, it } from "vitest";

import { DesktopCommandError, isTauriRuntime, toUserMessage } from "./desktop";

describe("error presentation", () => {
  it("presents structured, string and unknown errors in German UI-safe text", () => {
    expect(toUserMessage({ message: "Dateikonflikt erkannt" })).toBe("Dateikonflikt erkannt");
    expect(toUserMessage("Pfad liegt außerhalb des Workspace")).toBe("Pfad liegt außerhalb des Workspace");
    expect(toUserMessage({ unexpected: true })).toBe("Die lokale Aktion konnte nicht abgeschlossen werden.");
  });

  it("preserves controlled native command messages", () => {
    const error = new DesktopCommandError("import_evidence", { detail: "Dateityp nicht zulässig" });
    expect(error.message).toBe("Dateityp nicht zulässig");
    expect(error.commandName).toBe("import_evidence");
  });
});

describe("runtime selection", () => {
  it("distinguishes Tauri from the clearly labeled browser demo", () => {
    expect(isTauriRuntime({} as Window)).toBe(false);
    expect(isTauriRuntime({ __TAURI_INTERNALS__: {} } as unknown as Window)).toBe(true);
  });
});
