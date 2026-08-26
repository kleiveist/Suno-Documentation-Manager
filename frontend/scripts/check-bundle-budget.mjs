import { readdir, readFile, stat } from "node:fs/promises";
import { dirname, extname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const FRONTEND_DIR = resolve(SCRIPT_DIR, "..");
const DIST_DIR = resolve(FRONTEND_DIR, "dist");
const BUDGET_PATH = resolve(FRONTEND_DIR, "bundle-budget.json");
const METRIC_NAMES = ["entryHtmlBytes", "javascriptBytes", "stylesheetBytes", "totalBytes"];

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(path)));
    } else if (entry.isFile()) {
      const details = await stat(path);
      files.push({ path: relative(DIST_DIR, path).split(sep).join("/"), bytes: details.size });
    }
  }
  return files;
}

function summarize(files) {
  const sizeForExtension = (extension) =>
    files.filter((file) => extname(file.path) === extension).reduce((total, file) => total + file.bytes, 0);
  return {
    entryHtmlBytes: files.find((file) => file.path === "index.html")?.bytes ?? 0,
    javascriptBytes: sizeForExtension(".js"),
    stylesheetBytes: sizeForExtension(".css"),
    totalBytes: files.reduce((total, file) => total + file.bytes, 0)
  };
}

function parseBudget(text) {
  const budget = JSON.parse(text);
  if (budget.schemaVersion !== 1 || typeof budget.limits !== "object" || budget.limits === null) {
    throw new Error("bundle-budget.json must declare schemaVersion 1 and a limits object");
  }
  const actualNames = Object.keys(budget.limits).sort();
  if (actualNames.join(",") !== [...METRIC_NAMES].sort().join(",")) {
    throw new Error(`bundle-budget.json limits must be exactly: ${METRIC_NAMES.join(", ")}`);
  }
  for (const name of METRIC_NAMES) {
    if (!Number.isSafeInteger(budget.limits[name]) || budget.limits[name] <= 0) {
      throw new Error(`bundle-budget.json limit ${name} must be a positive integer`);
    }
  }
  return budget.limits;
}

export function evaluateBudget(metrics, limits) {
  return METRIC_NAMES.map((name) => ({
    name,
    actual: metrics[name],
    limit: limits[name],
    passed: metrics[name] <= limits[name]
  }));
}

async function main() {
  const limits = parseBudget(await readFile(BUDGET_PATH, "utf8"));
  const checks = evaluateBudget(summarize(await collectFiles(DIST_DIR)), limits);
  for (const check of checks) {
    const status = check.passed ? "OK" : "FAIL";
    console.log(`${status} ${check.name}: ${check.actual} bytes (limit ${check.limit})`);
  }
  if (checks.some((check) => !check.passed)) process.exitCode = 1;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Bundle budget check failed: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  });
}
