import { defineConfig } from "vitest/config";

// Interface em jsdom; scripts de infraestrutura em node (precisam de file: URLs e fs).
export default defineConfig({
  test: {
    projects: [
      { test: { name: "ui", environment: "jsdom", include: ["src/**/*.test.ts"] } },
      { test: { name: "scripts", environment: "node", include: ["scripts/**/*.test.ts"] } },
    ],
  },
});
