import { reportSelfTest } from "./self-test";

const startedAt = performance.now();
const root = document.querySelector<HTMLElement>("#app");
if (!root) {
  throw new Error("elemento #app ausente em index.html");
}

const title = document.createElement("h1");
title.textContent = "Prelo";
root.replaceChildren(title);

void reportSelfTest(root, startedAt);
