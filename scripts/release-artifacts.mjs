#!/usr/bin/env node
// Instaladores da release (spec 002, contracts/release-scripts.md, data-model.md "Instalador").
// Comandos: collect <dir> <versão> <saída> · sums <dir>.
import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { isMain } from "./verify.mjs";

/** Os quatro instaladores da versão, na ordem macOS, Windows, Linux. */
export function expectedInstallers(version) {
  return [
    `Prelo_${version}_universal.dmg`,
    `Prelo_${version}_x64-setup.exe`,
    `Prelo_${version}_amd64.AppImage`,
    `Prelo_${version}_amd64.deb`,
  ];
}

/** Compara nomes de arquivo com o conjunto esperado: tudo que não é esperado sobra. */
export function matchInstallers(files, version) {
  const expected = expectedInstallers(version);
  const names = new Set(files);
  return {
    found: expected.filter((f) => names.has(f)),
    missing: expected.filter((f) => !names.has(f)),
    extra: [...names].filter((f) => !expected.includes(f)).sort(),
  };
}

function listFiles(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? listFiles(path) : [path];
  });
}

function collect(dir, version, out) {
  const paths = listFiles(dir);
  const errors = [];
  const seen = new Set();
  for (const p of paths) {
    const name = basename(p);
    if (seen.has(name)) errors.push(`release: instalador repetido ${name}`);
    seen.add(name);
  }
  const { missing, extra } = matchInstallers(paths.map((p) => basename(p)), version);
  for (const f of missing) errors.push(`release: falta instalador ${f}`);
  for (const f of extra) errors.push(`release: instalador inesperado ${f}`);
  if (errors.length > 0) return errors;
  mkdirSync(out, { recursive: true });
  for (const p of paths) copyFileSync(p, join(out, basename(p)));
  return [];
}

export const SUMS_FILE = "SHA256SUMS.txt";

/** SHA-256 do arquivo em hex minúsculo, lido em fluxo (instaladores têm centenas de MB). */
export function sha256File(path) {
  return new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    createReadStream(path)
      .on("data", (chunk) => hash.update(chunk))
      .on("error", reject)
      .on("end", () => resolve(hash.digest("hex")));
  });
}

const byFile = (a, b) => (a.file < b.file ? -1 : a.file > b.file ? 1 : 0);

/** Formato do sha256sum: `<hex>  <nome>`, ordem alfabética, `\n` final. */
export function formatSums(entries) {
  return [...entries].sort(byFile).map((e) => `${e.sha256}  ${e.file}\n`).join("");
}

/** Arquivos listados cujo hash não confere (ou que não existem, ou que saem do diretório). */
export async function verifySums(text, dir) {
  const bad = [];
  for (const line of text.split(/\r?\n/).filter(Boolean)) {
    const m = /^([0-9a-f]{64}) {2}(.+)$/.exec(line);
    const file = m ? m[2] : line;
    const safe = m && basename(file) === file && file !== "." && file !== "..";
    const path = join(dir, file);
    if (!safe || !existsSync(path) || (await sha256File(path)) !== m[1]) bad.push(file);
  }
  return bad;
}

async function sums(dir) {
  const files = readdirSync(dir)
    .filter((f) => f !== SUMS_FILE && statSync(join(dir, f)).isFile())
    .sort();
  if (files.length === 0) return [`release: nenhum instalador em ${dir} para gerar ${SUMS_FILE}`];
  const entries = [];
  for (const file of files) entries.push({ file, sha256: await sha256File(join(dir, file)) });
  writeFileSync(join(dir, SUMS_FILE), formatSums(entries));
  return [];
}

async function main(argv) {
  const [cmd, ...args] = argv;
  let errors;
  if (cmd === "collect" && args.length === 3) {
    errors = collect(args[0], args[1], args[2]);
  } else if (cmd === "sums" && args.length === 1) {
    errors = await sums(args[0]);
  } else {
    errors = ["release: uso: release-artifacts.mjs collect <dir> <versão> <saída> | sums <dir>"];
  }
  for (const e of errors) console.error(e);
  return errors.length === 0 ? 0 : 1;
}

if (isMain(import.meta.url, process.argv[1])) {
  process.exitCode = await main(process.argv.slice(2));
}
