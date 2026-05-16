import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:5179",
    trace: "on-first-retry",
  },
  webServer: [
    {
      command: "ACCOUNTS_REPO_AUTH_DISABLED_DEV=1 ACCOUNTS_REPO_BIND_ADDR=127.0.0.1:18080 CORS_ALLOWED_ORIGIN=http://127.0.0.1:5179 cargo run -p accounts-repo-backend",
      url: "http://127.0.0.1:18080/health",
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: "BACKEND_URL=http://127.0.0.1:18080 VITE_DEV_AUTH_EMAIL=aina@ahadvisory.test VITE_DEV_AUTH_NAME='Aina Rahman' VITE_DEV_AUTH_ID=seed-preparer pnpm --dir frontend dev --host 127.0.0.1 --port 5179 --strictPort",
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
    {
      name: "mobile-chrome",
      use: { ...devices["Pixel 7"] },
    },
  ],
});
