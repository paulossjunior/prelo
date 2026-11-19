// Licença do Prelo (decisão do dono, 2026-11-19): MIT OR Apache-2.0.
// A pipeline de release (feature 002) recusa publicar sem estes arquivos.
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const root = fileURLToPath(new URL("..", import.meta.url));
const SPDX = "MIT OR Apache-2.0";

describe("licença do projeto", () => {
  it("tem LICENSE-MIT e LICENSE-APACHE na raiz", () => {
    expect(existsSync(join(root, "LICENSE-MIT"))).toBe(true);
    expect(existsSync(join(root, "LICENSE-APACHE"))).toBe(true);
    expect(readFileSync(join(root, "LICENSE-APACHE"), "utf8")).toContain("Apache License");
    expect(readFileSync(join(root, "LICENSE-MIT"), "utf8")).toContain("Permission is hereby granted");
  });

  it("package.json declara a mesma licença SPDX", () => {
    const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    expect(pkg.license).toBe(SPDX);
  });

  it("src-tauri/Cargo.toml declara a mesma licença SPDX", () => {
    const toml = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
    expect(toml).toMatch(new RegExp(`^license = "${SPDX}"$`, "m"));
  });
});
