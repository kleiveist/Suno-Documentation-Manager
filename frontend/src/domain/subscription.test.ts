import { describe, expect, it } from "vitest";

import { subscriptionCoverageEnd } from "./subscription";

describe("subscription coverage", () => {
  it("materializes one monthly or annual billing period inclusively", () => {
    expect(subscriptionCoverageEnd("2026-08-01", "monthly")).toBe("2026-08-31");
    expect(subscriptionCoverageEnd("2026-08-01", "annual")).toBe("2027-07-31");
  });

  it("handles month ends and leap years without overflowing", () => {
    expect(subscriptionCoverageEnd("2026-01-31", "monthly")).toBe("2026-02-27");
    expect(subscriptionCoverageEnd("2024-02-29", "annual")).toBe("2025-02-27");
  });

  it("rejects missing or impossible dates", () => {
    expect(subscriptionCoverageEnd("", "monthly")).toBeNull();
    expect(subscriptionCoverageEnd("2026-02-30", "monthly")).toBeNull();
    expect(subscriptionCoverageEnd("14.08.2026", "annual")).toBeNull();
  });
});
