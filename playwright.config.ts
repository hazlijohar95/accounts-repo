import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:5179",
    trace: "on-first-retry",
  },
  webServer: [
    {
      command: "cargo run -p accounts-repo-backend",
      url: "http://127.0.0.1:8080/health",
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: "pnpm --dir frontend dev --host 127.0.0.1 --port 5179 --strictPort",
      url: "http://127.0.0.1:5179",
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
