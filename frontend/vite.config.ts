import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";

function parsePort(value: string | undefined, fallback: number): number {
  const port = Number(value ?? fallback);
  return Number.isInteger(port) && port >= 1 && port <= 65535 ? port : fallback;
}

function readRootDotenv(path: string): Record<string, string> {
  if (!existsSync(path)) return {};
  const values: Record<string, string> = {};
  for (const [index, rawLine] of readFileSync(path, "utf8").split(/\r?\n/).entries()) {
    let line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (line.startsWith("export ")) line = line.slice(7).trimStart();
    const separator = line.indexOf("=");
    if (separator < 1) throw new Error(`Invalid dotenv entry at ${path}:${index + 1}`);
    const name = line.slice(0, separator).trim();
    let value = line.slice(separator + 1).trim();
    if (!/^[A-Z][A-Z0-9_]*$/.test(name)) {
      throw new Error(`Invalid dotenv variable name at ${path}:${index + 1}`);
    }
    if (value.startsWith('"') && value.endsWith('"')) {
      value = JSON.parse(value) as string;
    } else if (value.startsWith("'") && value.endsWith("'")) {
      value = value.slice(1, -1);
    } else {
      value = value.split(" #", 1)[0].trim();
    }
    values[name] = value;
  }
  return values;
}

export default defineConfig(() => {
  const projectRoot = fileURLToPath(new URL("..", import.meta.url));
  const env = { ...readRootDotenv(resolve(projectRoot, ".env")), ...process.env };
  const frontendHost = env.FRONTEND_HOST || "127.0.0.1";
  const frontendPort = parsePort(env.FRONTEND_PORT, 5173);
  return {
    // Vite's implicit mode files are disabled so every adapter uses only the root .env contract.
    envDir: resolve(projectRoot, ".vite-env-disabled"),
    // No environment variables are exposed to the offline-only renderer.
    envPrefix: [],
    server: {
      host: frontendHost,
      port: frontendPort,
      strictPort: false
    },
    preview: {
      host: frontendHost,
      port: 4173
    }
  };
});
