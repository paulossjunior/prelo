// Gitflow (decisão do dono, 2026-11-19): feature/* → dev → main.
// PR para a dev roda o verify só no macOS; PR para a main, push na main e a release rodam nos 3.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (...p: string[]) => readFileSync(join(root, ...p), "utf8");
const ci = () => parse(read(".github", "workflows", "ci.yml"));

const FULL = ["ubuntu-22.04", "windows-latest", "macos-latest"];
const DEV = ["macos-latest"];
const MATRIX =
  "${{ fromJSON((github.base_ref == 'dev' || github.ref == 'refs/heads/dev') && " +
  `'${JSON.stringify(DEV)}' || '${JSON.stringify(FULL)}') }}`;

/** Avalia a expressão da matriz para um contexto do GitHub, sem depender do runner. */
function matrixFor(expr: string, ctx: { base_ref?: string; ref: string }): string[] {
  const m = expr.match(/^\$\{\{ fromJSON\(\(github\.base_ref == '(\w+)' \|\| github\.ref == 'refs\/heads\/(\w+)'\) && '(\[[^']*\])' \|\| '(\[[^']*\])'\) \}\}$/);
  if (!m) throw new Error(`matriz fora do formato esperado: ${expr}`);
  const isDev = ctx.base_ref === m[1] || ctx.ref === `refs/heads/${m[2]}`;
  return JSON.parse(isDev ? m[3] : m[4]);
}

describe("ci.yml no gitflow", () => {
  it("roda em PR e push para main e dev", () => {
    const on = ci().on;
    expect(on.pull_request).toEqual({ branches: ["main", "dev"] });
    expect(on.push).toEqual({ branches: ["main", "dev"] });
    expect(on).toHaveProperty("workflow_call");
  });

  it("usa a matriz que depende do destino", () => {
    expect(ci().jobs.verify.strategy.matrix.os).toBe(MATRIX);
  });

  it.each([
    ["PR para a main", { base_ref: "main", ref: "refs/pull/1/merge" }, FULL],
    ["push na main", { ref: "refs/heads/main" }, FULL],
    ["tag de release (workflow_call)", { ref: "refs/tags/v1.2.3" }, FULL],
    ["PR para a dev", { base_ref: "dev", ref: "refs/pull/2/merge" }, DEV],
    ["push na dev", { ref: "refs/heads/dev" }, DEV],
  ])("%s roda em %j", (_, ctx, expected) => {
    expect(matrixFor(MATRIX, ctx)).toEqual(expected);
  });
});

describe("proteção das branches", () => {
  const checks = (os: string[]) => os.map((o) => `verify (${o})`);

  it("main exige os 3 sistemas", () => {
    const p = JSON.parse(read(".github", "branch-protection.json"));
    expect(p.required_status_checks.contexts).toEqual(checks(FULL));
    expect(p.enforce_admins).toBe(true);
  });

  it("dev exige PR e o verify do macOS, sem push direto nem force-push", () => {
    const p = JSON.parse(read(".github", "branch-protection-dev.json"));
    expect(p.required_status_checks).toEqual({ strict: true, contexts: checks(DEV) });
    expect(p.enforce_admins).toBe(true);
    expect(p.required_pull_request_reviews.required_approving_review_count).toBe(0);
    expect(p.allow_force_pushes).toBe(false);
    expect(p.allow_deletions).toBe(false);
  });
});
