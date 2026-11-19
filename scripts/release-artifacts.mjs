#!/usr/bin/env node
// Instaladores da release (spec 002, contracts/release-scripts.md, data-model.md "Instalador").
// Comandos: collect <dir> <versão> <saída>.
import { copyFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
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

function main(argv) {
  const [cmd, ...args] = argv;
  let errors;
  if (cmd === "collect" && args.length === 3) {
    errors = collect(args[0], args[1], args[2]);
  } else {
    errors = ["release: uso: release-artifacts.mjs collect <dir> <versão> <saída>"];
  }
  for (const e of errors) console.error(e);
  return errors.length === 0 ? 0 : 1;
}

if (isMain(import.meta.url, process.argv[1])) {
  process.exitCode = main(process.argv.slice(2));
}
