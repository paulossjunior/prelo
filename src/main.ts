import { invoke } from "@tauri-apps/api/core";
import { mountAppInfo } from "./app-info";
import { reportSelfTest } from "./self-test";

const root = document.querySelector<HTMLElement>("#app");
if (!root) {
  throw new Error("elemento #app ausente em index.html");
}

void mountAppInfo(root, (cmd) => invoke(cmd)).then(() => reportSelfTest(root));
