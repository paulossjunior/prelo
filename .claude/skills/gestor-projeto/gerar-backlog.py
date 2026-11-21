#!/usr/bin/env python3
"""Gera docs/backlog.md a partir do GitHub (gh) e dos tasks.md do spec-kit.

Uso, na raiz do repositório:  git fetch origin && python3 .claude/skills/gestor-projeto/gerar-backlog.py [saida] [data]
Requer gh autenticado. Só lê o GitHub; não cria nem altera issues.
"""
import datetime, json, re, subprocess, sys
from collections import Counter

REPO = "paulossjunior/prelo"
URL = f"https://github.com/{REPO}"
OUT = sys.argv[1] if len(sys.argv) > 1 else "docs/backlog.md"
DATE = sys.argv[2] if len(sys.argv) > 2 else datetime.date.today().isoformat()


def sh(*a):
    return subprocess.run(a, capture_output=True, text=True, check=True).stdout


issues = {i["number"]: i for i in json.loads(sh(
    "gh", "issue", "list", "--repo", REPO, "--state", "all", "--limit", "1000",
    "--json", "number,title,state,labels,body,milestone"))}
for i in issues.values():
    i["lab"] = [l["name"] for l in i["labels"]]


def level(i):
    for l in ("epic", "user-story", "task"):
        if l in i["lab"]:
            return l
    return None


def subs(n):
    return [s["number"] for s in json.loads(sh(
        "gh", "api", "--paginate", f"repos/{REPO}/issues/{n}/sub_issues"))]


def body_parent(i):
    m = re.search(r"Parte de #(\d+)", i["body"] or "")
    return int(m.group(1)) if m else None


# Mães candidatas: issues com label epic/user-story ou que alguém cita em "Parte de #N"
cited = {body_parent(i) for i in issues.values()} - {None}
parents = {n for n, i in issues.items() if level(i) in ("epic", "user-story")} | cited
sub_of = {}       # filha -> mãe (sub-issue nativa)
for p in sorted(parents):
    for c in subs(p):
        sub_of[c] = p

problems = []
groups = {}

def grp(cat, n):
    groups.setdefault(cat, []).append(n)

parent = {}
for n, i in issues.items():
    bp, sp = body_parent(i), sub_of.get(n)
    parent[n] = sp or bp
    if bp and not sp:
        grp(f"Citam `Parte de #{bp}` no corpo, mas não são sub-issues de #{bp} (vincular pela API)", n)
    elif sp and bp and sp != bp:
        problems.append(f"#{n}: sub-issue de #{sp}, mas o corpo diz `Parte de #{bp}`")
    elif sp and not bp and level(i) != "epic":
        problems.append(f"#{n}: sub-issue de #{sp}, mas o corpo não cita `Parte de #{sp}`")

children = {}
for n, p in parent.items():
    if p:
        children.setdefault(p, []).append(n)

# Raízes: label epic, ou issue sem mãe que tem filhas (epic de fato, sem label)
roots = sorted(n for n, i in issues.items()
               if level(i) == "epic" or (not parent[n] and n in children))
for r in roots:
    if level(issues[r]) != "epic":
        problems.append(f"#{r}: tem {len(children[r])} filhas mas não tem o label `epic` "
                        "(tratada como Epic legado abaixo)")


def ref(n):
    return f"[#{n}]({URL}/issues/{n})"


def meta(i):
    st = "aberta" if i["state"] == "OPEN" else "fechada"
    pr = next((l for l in i["lab"] if re.fullmatch(r"P[0-3]", l)), "—")
    ag = ", ".join(l.removeprefix("agente:") for l in i["lab"] if l.startswith("agente:")) or "—"
    extra = " · duplicada" if "duplicate" in i["lab"] else ""
    return st, pr, ag, extra


def short(t, k=95):
    t = t.replace("|", "\\|").replace("`", "")
    return t if len(t) <= k else t[: k - 1] + "…"


def week(i):
    """Milestone semanal (\"Semana AAAA-Www ...\") da issue, ou 'sem semana'."""
    ms = (i.get("milestone") or {}).get("title") or ""
    return ms if ms.startswith("Semana ") else "sem semana"


def tline(n):
    i = issues[n]
    st, pr, ag, ex = meta(i)
    box = "x" if i["state"] == "CLOSED" else " "
    wk = f" · {week(i)}" if i["state"] == "OPEN" else ""
    return f"- [{box}] {ref(n)} {short(i['title'])} · {pr} · {ag}{wk}{ex}"


def tid(n):
    m = re.match(r"T(\d+)", issues[n]["title"])
    return int(m.group(1)) if m else 10**6 + n


def spec_of(epic):
    """Feature spec-kit do epic, a partir dos corpos das filhas e netas."""
    c = Counter()
    stack = list(children.get(epic, [])) + [epic]
    while stack:
        n = stack.pop()
        c.update(re.findall(r"specs/(\d{3}-[a-z0-9-]+)", issues[n]["body"] or ""))
        if n != epic:
            stack += children.get(n, [])
    if not c:
        return None, None
    feat = c.most_common(1)[0][0]
    path = f"specs/{feat}/tasks.md"
    ok = subprocess.run(["git", "cat-file", "-e", f"origin/dev:{path}"],
                        capture_output=True).returncode == 0
    return feat, (path, "dev" if ok else feat)


def tasks_md_state(feat, branch):
    try:
        txt = sh("git", "show", f"origin/{branch}:specs/{feat}/tasks.md")
    except subprocess.CalledProcessError:
        return {}
    return {int(m.group(2)): m.group(1).lower() == "x"
            for m in re.finditer(r"^- \[([ xX])\] T(\d+)", txt, re.M)}


def task_block(ns, indent=""):
    ns = sorted(ns, key=tid)
    op = [n for n in ns if issues[n]["state"] == "OPEN"]
    cl = [n for n in ns if issues[n]["state"] == "CLOSED"]
    out = [indent + tline(n) for n in op]
    if cl:
        out += ["", f"<details><summary>{len(cl)} task(s) fechada(s)</summary>", ""]
        out += [tline(n) for n in cl]
        out += ["", "</details>"]
    return out


tot = Counter()
body = []
for e in roots:
    ei = issues[e]
    st, pr, ag, ex = meta(ei)
    feat, src = spec_of(e)
    tot["epic_" + st] += 1
    legacy = "" if level(ei) == "epic" else " _(Epic legado, sem label `epic`)_"
    body += [f"## {ref(e)} {short(ei['title'], 120)}{legacy}", ""]
    rel = ", ".join(l.removeprefix("release:") for l in ei["lab"] if l.startswith("release:"))
    ms = (ei.get("milestone") or {}).get("title") or ""
    legacy_ms = ms and not ms.startswith("Semana ")
    if not rel:
        rel = f"— (milestone de fase: {ms})" if legacy_ms else "—"
        grp("Epics com milestone de fase e sem label `release:*` (migrar, ver Milestones semanais)"
            if legacy_ms else "Epics sem label `release:*`", e)
    srcl = f"[`{src[0]}`]({URL}/blob/{src[1]}/{src[0]}) (branch `{src[1]}`)" if src else "—"
    body += [f"Estado: **{st}** · Prioridade: {pr} · Agente: {ag} · Release: {rel}{ex}  ",
             f"Fonte: {srcl}", ""]
    kids = children.get(e, [])
    us = sorted((k for k in kids if level(issues[k]) == "user-story"),
                key=lambda k: issues[k]["title"])
    direct = [k for k in kids if k not in us]
    alltasks = direct + [t for u in us for t in children.get(u, [])]
    done = sum(issues[t]["state"] == "CLOSED" for t in alltasks)
    body += [f"Progresso: {len(us)} US · {done}/{len(alltasks)} tasks fechadas", ""]
    for u in us:
        tot["us_" + meta(issues[u])[0]] += 1
    for t in alltasks:
        tot["task_" + meta(issues[t])[0]] += 1
        if level(issues[t]) != "task":
            grp(f"Filhas de #{e} sem o label `task`", t)
        if issues[t]["state"] == "OPEN" and ei["state"] == "CLOSED":
            grp(f"Abertas, mas o Epic #{e} está fechado", t)
    if src:
        md = tasks_md_state(feat, src[1])
        for t in alltasks:
            k = tid(t)
            if k in md and md[k] != (issues[t]["state"] == "CLOSED"):
                grp(f"Estado da issue diverge do checkbox em `{src[0]}` (branch `{src[1]}`)", t)
        missing = sorted(set(md) - {tid(t) for t in alltasks})
        if missing:
            problems.append(f"Epic #{e}: tasks do tasks.md sem issue: "
                            + ", ".join(f"T{k:03d}" for k in missing))
    if direct:
        body += ["### Tasks direto no Epic (Setup, Foundational, Polish)", ""]
        body += task_block(direct) + [""]
    for u in us:
        ui = issues[u]
        st, pr, ag, ex = meta(ui)
        ts = children.get(u, [])
        dn = sum(issues[t]["state"] == "CLOSED" for t in ts)
        body += [f"### {ref(u)} {short(ui['title'], 110)}", "",
                 f"Estado: **{st}** · {pr} · {ag}{ex} · {dn}/{len(ts)} tasks fechadas", ""]
        if ts:
            body += task_block(ts) + [""]
        elif "duplicate" not in ui["lab"]:
            problems.append(f"#{u}: User Story sem tasks")
        if st == "fechada" and dn < len(ts):
            problems.append(f"#{u}: US fechada com {len(ts) - dn} task(s) aberta(s)")

# Órfãos: US/Task sem mãe que leve a um Epic
placed = set(roots)
for r in roots:
    stack = list(children.get(r, []))
    while stack:
        n = stack.pop()
        placed.add(n)
        stack += children.get(n, [])
for n, i in sorted(issues.items()):
    if n in placed:
        continue
    lv = level(i)
    if lv == "user-story":
        problems.append(f"{ref(n)} User Story sem Epic: {short(i['title'], 70)}")
    elif lv == "task":
        problems.append(f"{ref(n)} Task sem User Story nem Epic: {short(i['title'], 70)}")
    elif i["state"] == "OPEN":
        problems.append(f"{ref(n)} issue aberta fora da hierarquia (sem label de nível): "
                        f"{short(i['title'], 70)}")


def fmt_counts(k):
    return f"{tot[k + '_aberta'] + tot[k + '_fechada']} ({tot[k + '_aberta']} abertas, {tot[k + '_fechada']} fechadas)"


def pnum(s):
    m = re.search(r"#(\d+)", s)
    return int(m.group(1)) if m else 0


head = f"""# Backlog do Prelo

Hierarquia **Epic → User Story → Task** do GitHub, gerada pela skill `gestor-projeto`
(operação **backlog**). Cada linha aponta para a issue no GitHub; o GitHub é a fonte da
verdade, este arquivo é um retrato. Para o mapa de versões (release → epics), veja
[release-plan.md](release-plan.md).

- Gerado em: {DATE}, a partir de `gh issue list --state all` e da API de sub-issues.
- Como regenerar: peça `/gestor-projeto backlog` ou siga a seção "Backlog em docs/backlog.md"
  de `.claude/skills/gestor-projeto/SKILL.md`.
- Legenda: `[x]` fechada, `[ ]` aberta · prioridade · agente(s) `agente:*` · semana
  (milestone semanal) nas abertas. A release de cada Epic vem do label `release:*`.

## Resumo

| Nível | Total |
|---|---|
| Epics | {fmt_counts('epic')} |
| User Stories | {fmt_counts('us')} |
| Tasks | {fmt_counts('task')} |

"""
probs = sorted(set(problems), key=pnum)
for cat, ns in groups.items():
    ns = sorted(set(ns))
    probs.append(f"{cat} ({len(ns)}): " + ", ".join(ref(n) for n in ns))
tail = ["## Órfãos / inconsistências", ""]
tail += [f"- {p}" for p in probs] if probs else ["Nenhuma."]
open(OUT, "w").write(head + "\n".join(body).rstrip() + "\n\n" + "\n".join(tail) + "\n")
print(dict(tot), len(probs))
