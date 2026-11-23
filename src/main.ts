// Monta a área de trabalho (T039): árvore | editor | prévia, com a barra de ações e o status.
import { invoke } from "@tauri-apps/api/core";
import { ipc } from "./ipc";
import { watchCspViolations } from "./layout-check";
import { openE2eFixture, reportSelfTest } from "./self-test";
import { mountWorkspace } from "./workspace";

// Antes de montar qualquer coisa: o autoteste reporta toda violação da CSP vista na página.
const cspViolations = watchCspViolations(document);

const root = document.querySelector<HTMLElement>("#app");
if (!root) {
  throw new Error("elemento #app ausente em index.html");
}

const ws = mountWorkspace(root, { ipc, appInfo: (cmd) => invoke(cmd) });
void reportSelfTest(ws, cspViolations);
void openE2eFixture(ws);
