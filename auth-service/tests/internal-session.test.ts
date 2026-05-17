import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { Pool } from "pg";

const execFileAsync = promisify(execFile);
const authServiceDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoDir = resolve(authServiceDir, "..");
const internalToken = "test-internal-token";
const trustedOrigin = "http://127.0.0.1:5179";

function requireDatabaseUrl() {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) throw new Error("DATABASE_URL is required for auth-service integration tests");
  return databaseUrl;
}

function withSearchPath(databaseUrl: string, schema: string) {
  const separator = databaseUrl.includes("?") ? "&" : "?";
  return `${databaseUrl}${separator}options=-c%20search_path%3D${schema}`;
}

async function createSchema(databaseUrl: string, schema: string) {
  const pool = new Pool({ connectionString: databaseUrl });
  try {
    await pool.query(`CREATE SCHEMA "${schema}"`);
  } finally {
    await pool.end();
  }
}

async function dropSchema(databaseUrl: string, schema: string) {
  const pool = new Pool({ connectionString: databaseUrl });
  try {
    await pool.query(`DROP SCHEMA IF EXISTS "${schema}" CASCADE`);
  } finally {
    await pool.end();
  }
}

function sessionCookieFrom(setCookie: string) {
  const sessionCookie = setCookie
    .split(/,(?=[^;,]+=)/)
    .map((cookie) => cookie.trim().split(";")[0])
    .find((cookie) => cookie.includes("session"));
  assert.ok(sessionCookie, "Better Auth sign-up response must set a session cookie");
  return sessionCookie;
}

test("returns an internal session for a real Better Auth email signup to prevent backend/auth contract drift", async () => {
  const databaseUrl = requireDatabaseUrl();
  const schema = `auth_test_${randomUUID().replaceAll("-", "")}`;
  const scopedDatabaseUrl = withSearchPath(databaseUrl, schema);
  const email = `reviewer-${randomUUID()}@ahadvisory.test`;
  const env = {
    ...process.env,
    DATABASE_URL: scopedDatabaseUrl,
    BETTER_AUTH_SECRET: "test-secret-for-auth-service-integration-only",
    BETTER_AUTH_URL: trustedOrigin,
    BETTER_AUTH_TRUSTED_ORIGINS: trustedOrigin,
    AUTH_INTERNAL_TOKEN: internalToken,
  };

  await createSchema(databaseUrl, schema);
  try {
    await execFileAsync("pnpm", ["--dir", authServiceDir, "auth:migrate", "--yes"], {
      cwd: repoDir,
      env,
      timeout: 120_000,
    });

    Object.assign(process.env, env);
    const [{ createApp }, { authPool }] = await Promise.all([import("../src/app"), import("../src/auth")]);
    const app = createApp();
    try {
      const signupResponse = await app.request("/api/auth/sign-up/email", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          origin: trustedOrigin,
          "x-forwarded-for": "203.0.113.10",
        },
        body: JSON.stringify({
          name: "Amjad Salleh",
          email,
          password: "A-real-password-123",
        }),
      });
      assert.equal(signupResponse.status, 200);
      const signupPayload = (await signupResponse.json()) as {
        token: string | null;
        user: { email: string; name: string };
      };
      assert.equal(signupPayload.token, null);
      assert.equal(signupPayload.user.email, email.toLowerCase());
      assert.equal(signupPayload.user.name, "Amjad Salleh");

      const unverifiedSignInResponse = await app.request("/api/auth/sign-in/email", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          origin: trustedOrigin,
          "x-forwarded-for": "203.0.113.10",
        },
        body: JSON.stringify({
          email,
          password: "A-real-password-123",
        }),
      });
      assert.equal(unverifiedSignInResponse.status, 403);

      await authPool.query('UPDATE "user" SET "emailVerified" = true WHERE email = $1', [email.toLowerCase()]);

      const verifiedSignInResponse = await app.request("/api/auth/sign-in/email", {
        method: "POST",
        headers: {
          "content-type": "application/json",
          origin: trustedOrigin,
          "x-forwarded-for": "203.0.113.10",
        },
        body: JSON.stringify({
          email,
          password: "A-real-password-123",
        }),
      });
      assert.equal(verifiedSignInResponse.status, 200);
      const setCookie = verifiedSignInResponse.headers.get("set-cookie");
      assert.ok(setCookie, "Better Auth sign-in must return a Set-Cookie header after verification");
      const cookie = sessionCookieFrom(setCookie);

      const missingInternalTokenResponse = await app.request("/internal/session", {
        headers: { cookie },
      });
      assert.equal(missingInternalTokenResponse.status, 401);

      const sessionResponse = await app.request("/internal/session", {
        headers: {
          cookie,
          "x-internal-auth-token": internalToken,
        },
      });
      assert.equal(sessionResponse.status, 200);
      const sessionPayload = (await sessionResponse.json()) as {
        user: { id: string; name: string; email: string; emailVerified: boolean };
        session: { id: string; expiresAt: string };
      };
      assert.ok(sessionPayload.user.id);
      assert.equal(sessionPayload.user.name, "Amjad Salleh");
      assert.equal(sessionPayload.user.email, email.toLowerCase());
      assert.equal(sessionPayload.user.emailVerified, true);
      assert.ok(sessionPayload.session.id);
      assert.ok(sessionPayload.session.expiresAt);

      const failedSignInStatuses = [];
      for (let attempt = 0; attempt < 6; attempt += 1) {
        const failedSignInResponse = await app.request("/api/auth/sign-in/email", {
          method: "POST",
          headers: {
            "content-type": "application/json",
            origin: trustedOrigin,
            "x-forwarded-for": "203.0.113.10",
          },
          body: JSON.stringify({
            email,
            password: `wrong-password-${attempt}`,
          }),
        });
        failedSignInStatuses.push(failedSignInResponse.status);
      }
      assert.notEqual(failedSignInStatuses[0], 429);
      assert.equal(failedSignInStatuses.at(-1), 429);
    } finally {
      await authPool.end();
    }
  } finally {
    await dropSchema(databaseUrl, schema);
  }
});
