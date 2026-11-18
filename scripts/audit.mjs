#!/usr/bin/env node
// Auditoria de dependências (T025, FR-008, FR-009, research R4).
// Rust: cargo-deny (licenças, avisos, fontes). npm: audit-ci (alto/crítico) e checagem de
// licenças com a lista única de security/licenses-allow.txt. Exceções em
// security/vuln-exceptions.toml precisam de motivo e de data de revisão futura.
import { spawnSync } from "node:child_process";
import { readFileSync, realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse as parseToml } from "smol-toml";

const ECOSYSTEMS = new Set(["cargo", "npm"]);
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;
// O id vai para a linha de comando do audit-ci: só formatos de aviso conhecidos.
const ADVISORY_ID = /^(GHSA(-[0-9a-z]{4}){3}|RUSTSEC-\d{4}-\d{4}|CVE-\d{4}-\d{4,})$/;

export function parseAllowlist(text) {
  return text
    .split(/\r?\n/)
    .map((l) => l.replace(/#.*/, "").trim())
    .filter(Boolean);
}

export function denyAllowList(tomlText) {
  return parseToml(tomlText).licenses?.allow ?? [];
}

export function denyAdvisoryIgnores(tomlText) {
  const ignore = parseToml(tomlText).advisories?.ignore ?? [];
  return ignore.map((e) => (typeof e === "string" ? e : e.id));
}

export function parseExceptions(tomlText) {
  return parseToml(tomlText).exception ?? [];
}

export function validateExceptions(list, today) {
  const errors = [];
  for (const e of list) {
    const id = e.id || "(sem id)";
    if (!e.id) errors.push(`exceção sem id`);
    else if (!ADVISORY_ID.test(e.id)) errors.push(`${id}: id fora do formato GHSA/RUSTSEC/CVE`);
    if (!ECOSYSTEMS.has(e.ecosystem)) errors.push(`${id}: ecosystem deve ser cargo ou npm`);
    if (!e.reason || !String(e.reason).trim()) errors.push(`${id}: reason vazio`);
    const date = typeof e.review_by === "string" && ISO_DATE.test(e.review_by) ? new Date(e.review_by) : null;
    if (!date || Number.isNaN(date.getTime())) errors.push(`${id}: review_by ausente ou inválido`);
    else if (date < startOfDay(today)) errors.push(`${id}: review_by ${e.review_by} vencido`);
  }
  return errors;
}

function startOfDay(d) {
  return new Date(d.toISOString().slice(0, 10));
}

export function exceptionIds(list, ecosystem) {
  return list.filter((e) => e.ecosystem === ecosystem).map((e) => e.id);
}

/** Expressão SPDX simples: OR de alternativas, cada uma um AND de licenças. */
function allowed(expr, allow) {
  const clean = expr.replace(/[()]/g, " ").trim();
  if (!clean || clean === "UNKNOWN" || clean === "UNLICENSED") return false;
  return clean
    .split(/\s+OR\s+/)
    .some((alt) => alt.split(/\s+AND\s+/).every((lic) => allow.includes(lic.trim())));
}

export function licenseViolations(packages, allow) {
  return packages.filter((p) => !allowed(String(p.licenses ?? ""), allow));
}

// ---------------------------------------------------------------------------------------

function run(name, cmd, opts = {}) {
  console.log(`\n▶ audit ${name}: ${cmd}`);
  const r = spawnSync(cmd, { stdio: "inherit", shell: true, ...opts });
  return r.status === 0;
}

function npmLicenses(root) {
  const r = spawnSync("npx --no-install license-checker-rseidelsohn --json --excludePrivatePackages --production", {
    cwd: root,
    shell: true,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (r.status !== 0) throw new Error(`license-checker falhou: ${r.stderr}`);
  return Object.entries(JSON.parse(r.stdout)).map(([name, info]) => ({
    name,
    licenses: Array.isArray(info.licenses) ? info.licenses.join(" AND ") : info.licenses,
  }));
}

function main() {
  const root = fileURLToPath(new URL("..", import.meta.url));
  process.chdir(root);
  const read = (p) => readFileSync(p, "utf8");
  const allow = parseAllowlist(read("security/licenses-allow.txt"));
  const exceptions = parseExceptions(read("security/vuln-exceptions.toml"));
  const results = [];

  const errors = validateExceptions(exceptions, new Date());
  for (const e of errors) console.error(`exceção inválida: ${e}`);
  results.push(["exceções", errors.length === 0]);

  const denyAllow = denyAllowList(read("src-tauri/deny.toml"));
  const inSync = [...denyAllow].sort().join() === [...allow].sort().join();
  if (!inSync) console.error("src-tauri/deny.toml [licenses].allow difere de security/licenses-allow.txt");
  results.push(["lista de licenças em sincronia", inSync]);

  results.push(["cargo-deny", run("cargo-deny", "cargo deny --manifest-path src-tauri/Cargo.toml check")]);

  // Só ids validados chegam ao shell; com exceção inválida, nenhuma é aplicada.
  const allowlist = errors.length === 0 ? exceptionIds(exceptions, "npm") : [];
  const auditCi = `npx --no-install audit-ci --high${allowlist.map((id) => ` --allowlist ${id}`).join("")}`;
  results.push(["npm vulnerabilidades", run("npm", auditCi)]);

  let violations = [];
  try {
    violations = licenseViolations(npmLicenses(root), allow);
  } catch (err) {
    console.error(String(err));
    violations = [{ name: "(license-checker)", licenses: "falhou" }];
  }
  for (const v of violations) console.error(`licença não permitida: ${v.name} → ${v.licenses || "(sem licença)"}`);
  results.push(["npm licenças", violations.length === 0]);

  console.log("");
  for (const [name, ok] of results) console.log(`${ok ? "ok   " : "FALHA"}  audit ${name}`);
  process.exitCode = results.every(([, ok]) => ok) ? 0 : 1;
}

const isMain = (() => {
  try {
    return realpathSync(fileURLToPath(import.meta.url)) === realpathSync(process.argv[1] ?? "");
  } catch {
    return false;
  }
})();
if (isMain) main();
