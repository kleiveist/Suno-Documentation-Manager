import { defineConfig } from "@playwright/test";

const baseURL = process.env.PLAYWRIGHT_BASE_URL?.trim() || "http://127.0.0.1:5173";
const inCi = process.env.CI === "true";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  forbidOnly: inCi,
  retries: inCi ? 2 : 0,
  workers: inCi ? 1 : undefined,
  reporter: inCi ? [["line"], ["html", { open: "never", outputFolder: "coverage/e2e-report" }]] : "list",
  outputDir: "coverage/e2e-results",
  use: {
    baseURL,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure"
  }
});
