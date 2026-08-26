import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts", "scripts/**/*.test.mjs"],
    coverage: {
      include: ["src/api/**/*.ts", "src/project-profile.ts"],
      provider: "v8",
      reporter: ["text", "json-summary", "html"],
      reportsDirectory: "coverage",
      thresholds: {
        branches: 50,
        functions: 58,
        lines: 61,
        statements: 57,
        perFile: true
      }
    }
  }
});
