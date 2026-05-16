import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const backendUrl = process.env.BACKEND_URL ?? "http://127.0.0.1:8080";
const authUrl = process.env.AUTH_SERVICE_URL ?? "http://127.0.0.1:8081";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api/auth": authUrl,
      "/api": backendUrl,
      "/health": backendUrl,
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./tests/setup.ts",
  },
});
