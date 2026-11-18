// WebDriver ponta a ponta (T010) via tauri-driver: só Linux e Windows (research R3).
import { spawn, type ChildProcess } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const exe = process.platform === "win32" ? "prelo.exe" : "prelo";
const application = join(root, "src-tauri", "target", "debug", exe);
const driverBin = join(homedir(), ".cargo", "bin", process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver");

let driver: ChildProcess | undefined;

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./app.e2e.ts"],
  maxInstances: 1,
  hostname: "127.0.0.1",
  port: 4444,
  capabilities: [
    {
      // Capability própria do tauri-driver.
      "tauri:options": { application },
    } as WebdriverIO.Capabilities,
  ],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { ui: "bdd", timeout: 60_000 },
  beforeSession: () => {
    // No Windows: PRELO_NATIVE_DRIVER (wrapper do CI com log detalhado) ou o msedgedriver
    // em EDGEWEBDRIVER.
    const edge = process.env.EDGEWEBDRIVER;
    const native = process.env.PRELO_NATIVE_DRIVER ?? (edge ? join(edge, "msedgedriver.exe") : undefined);
    const args = process.platform === "win32" && native ? ["--native-driver", native] : [];
    driver = spawn(driverBin, args, { stdio: [null, process.stdout, process.stderr] });
  },
  afterSession: () => {
    driver?.kill();
  },
};
