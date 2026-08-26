import { describe, expect, it } from "vitest";

import { evaluateBudget } from "./check-bundle-budget.mjs";

const LIMITS = {
  entryHtmlBytes: 100,
  javascriptBytes: 200,
  stylesheetBytes: 150,
  totalBytes: 500
};

describe("bundle budget evaluation", () => {
  it("accepts metrics at their inclusive limits", () => {
    expect(evaluateBudget(LIMITS, LIMITS).every((check) => check.passed)).toBe(true);
  });

  it("reports only metrics that exceed a limit", () => {
    const checks = evaluateBudget({ ...LIMITS, javascriptBytes: 201 }, LIMITS);

    expect(checks.filter((check) => !check.passed).map((check) => check.name)).toEqual(["javascriptBytes"]);
  });
});
