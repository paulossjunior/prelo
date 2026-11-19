#!/usr/bin/env node
// Pré-checagem da release (spec 002, contracts/release-scripts.md, data-model.md).
// Uso, na raiz do repositório: node scripts/release-check.mjs <tag>
// Confere o formato da tag, se o HEAD está na main, as três versões declaradas e a licença.
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { isMain } from "./verify.mjs";

const TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const SHA = /^([0-9a-f]{40}|[0-9a-f]{64})$/;
const LICENSE_NAMES = ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING", "LICENSE-MIT", "LICENSE-APACHE"];
const MISSING = "(ausente)";
// O repositório é o de `cwd`: GIT_DIR e afins herdados (ex.: de um hook) não podem desviar.
const gitEnv = () => Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_")));

export function parseTag(tag) {
  const m = TAG.exec(String(tag));
  if (!m) throw new Error(`tag "${tag}" fora do formato vX.Y.Z`);
  return { version: `${m[1]}.${m[2]}.${m[3]}` };
}

/** O commit está no histórico de origin/main? Só aceita SHA completo (nada vira opção do git). */
export function isOnMain(commit, cwd) {
  if (!SHA.test(String(commit))) return false;
  const r = spawnSync("git", ["merge-base", "--is-ancestor", commit, "refs/remotes/origin/main"], { cwd, env: gitEnv() });
  return r.status === 0;
}

function jsonVersion(path) {
  const v = JSON.parse(readFileSync(path, "utf8")).version;
  return typeof v === "string" ? v : MISSING;
}

/** `version` da seção [package] do Cargo.toml (sem tabelas inline nem herança de workspace). */
function cargoVersion(path) {
  let inPackage = false;
  for (const raw of readFileSync(path, "utf8").split(/\r?\n/)) {
    const line = raw.replace(/#.*/, "").trim();
    if (line.startsWith("[")) {
      inPackage = line === "[package]";
      continue;
    }
    const m = inPackage && /^version\s*=\s*"([^"]*)"$/.exec(line);
    if (m) return m[1];
  }
  return MISSING;
}

export function readVersions(root) {
  return [
    { file: "package.json", version: jsonVersion(join(root, "package.json")) },
    { file: "src-tauri/tauri.conf.json", version: jsonVersion(join(root, "src-tauri", "tauri.conf.json")) },
    { file: "src-tauri/Cargo.toml", version: cargoVersion(join(root, "src-tauri", "Cargo.toml")) },
  ];
}

export function versionMismatches(version, versions) {
  return versions.filter((v) => v.version !== version);
}

/** Primeiro arquivo de licença não vazio na raiz, ou null. */
export function findLicense(root) {
  for (const name of LICENSE_NAMES) {
    const path = join(root, name);
    if (existsSync(path) && readFileSync(path, "utf8").trim() !== "") return name;
  }
  return null;
}

/** Erros da release; com tag fora do formato, nada mais é conferido. */
export function checkRelease({ tag, root, commit }) {
  let version;
  try {
    ({ version } = parseTag(tag));
  } catch (err) {
    return [`release: ${err.message}`];
  }
  const errors = [];
  if (!isOnMain(commit, root)) errors.push(`release: o commit ${commit} da tag não pertence à main`);
  for (const m of versionMismatches(version, readVersions(root))) {
    errors.push(`release: ${m.file} declara versão ${m.version}, mas a tag é ${tag}`);
  }
  if (!findLicense(root)) {
    errors.push("release: falta arquivo de licença na raiz (LICENSE); escolha a licença antes de publicar");
  }
  return errors;
}

function main(argv) {
  if (argv.length !== 1) {
    console.error("release: uso: release-check.mjs <tag>");
    return 1;
  }
  const root = process.cwd();
  const head = spawnSync("git", ["rev-parse", "HEAD"], { cwd: root, encoding: "utf8", env: gitEnv() });
  const commit = head.status === 0 ? head.stdout.trim() : "(desconhecido)";
  const errors = checkRelease({ tag: argv[0], root, commit });
  for (const e of errors) console.error(e);
  if (errors.length === 0) console.log(`release: ${argv[0]} pode ser publicada (commit ${commit})`);
  return errors.length === 0 ? 0 : 1;
}

if (isMain(import.meta.url, process.argv[1])) {
  process.exitCode = main(process.argv.slice(2));
}
