import { describe, expect, it } from "vitest";

import { formatBytes, formatDate } from "./format";

const timestamp = "2026-08-20T12:34:00.000Z";

describe("locale-aware UI formatting", () => {
  it("keeps German as the compatibility default", () => {
    expect(formatDate()).toBe("Noch nicht");
    expect(formatBytes(1536)).toBe("1,5 KB");
  });

  it("formats dates in the selected locale while retaining the time option", () => {
    const value = new Date(timestamp);
    const german = new Intl.DateTimeFormat("de-DE", {
      dateStyle: "medium",
      timeStyle: "short"
    }).format(value);
    const english = new Intl.DateTimeFormat("en-US", {
      dateStyle: "medium",
      timeStyle: "short"
    }).format(value);

    expect(formatDate(timestamp, true, "de")).toBe(german);
    expect(formatDate(timestamp, true, "en")).toBe(english);
    expect(formatDate(undefined, false, "en")).toBe("Not yet");
  });

  it("uses locale-specific decimal separators for byte values", () => {
    expect(formatBytes(1536, "de")).toBe("1,5 KB");
    expect(formatBytes(1536, "en")).toBe("1.5 KB");
  });
});
