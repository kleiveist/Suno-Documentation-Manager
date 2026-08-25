import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

async function openSunoDmWelcome(page: Page): Promise<void> {
  await page.addInitScript(() => window.localStorage.clear());
  await page.goto("/", { waitUntil: "networkidle" });

  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page).toHaveTitle("Suno Documentation Manager");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Your music.Documented properly.");
}

test("presents the local-only SunoDM welcome screen", async ({ page }) => {
  await openSunoDmWelcome(page);

  await expect(page.getByRole("button", { name: "Choose workspace" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Create new workspace" })).toBeVisible();
  await expect(page.getByText("No cloud, no login, no telemetry.")).toBeVisible();
  await expect(page.getByText("Browser demo", { exact: true })).toBeVisible();
});

test("has no automatically detectable accessibility violations", async ({ page }) => {
  await openSunoDmWelcome(page);

  const results = await new AxeBuilder({ page }).analyze();

  expect(results.violations).toEqual([]);
});
