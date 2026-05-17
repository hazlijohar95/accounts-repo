import { Hono } from "hono";
import { cors } from "hono/cors";
import { auth } from "./auth";

type AuthInstance = typeof auth;

const DEVELOPMENT_INTERNAL_TOKEN = "development-internal-token";

function proxyTokenFromEnv() {
  return process.env.ACCOUNTS_REPO_PROXY_TOKEN?.trim() || null;
}

export function internalTokenFromEnv() {
  const internalToken = process.env.AUTH_INTERNAL_TOKEN?.trim();
  if (!internalToken) {
    throw new Error("AUTH_INTERNAL_TOKEN is required for the Better Auth service");
  }
  if (internalToken === DEVELOPMENT_INTERNAL_TOKEN) {
    throw new Error("AUTH_INTERNAL_TOKEN must not use the development placeholder value");
  }
  if (internalToken.startsWith("replace-with-")) {
    throw new Error("AUTH_INTERNAL_TOKEN must be replaced before starting the Better Auth service");
  }
  if (process.env.NODE_ENV === "production" && internalToken.length < 32) {
    throw new Error("AUTH_INTERNAL_TOKEN must be 32+ characters in production");
  }

  return internalToken;
}

export function createApp(authInstance: AuthInstance = auth) {
  const app = new Hono();
  const frontendOrigins = (process.env.BETTER_AUTH_TRUSTED_ORIGINS ?? "http://127.0.0.1:5173,http://127.0.0.1:5179")
    .split(",")
    .map((origin) => origin.trim())
    .filter(Boolean);
  const internalToken = internalTokenFromEnv();

  app.use(
    "*",
    cors({
      origin: (origin) => (frontendOrigins.includes(origin) ? origin : frontendOrigins[0]),
      credentials: true,
      allowHeaders: ["content-type", "cookie", "x-accounts-repo-proxy-token", "x-internal-auth-token"],
      allowMethods: ["GET", "POST", "OPTIONS"],
    }),
  );

  app.use("/api/auth/*", async (context, next) => {
    const proxyToken = proxyTokenFromEnv();
    if (proxyToken && context.req.header("x-accounts-repo-proxy-token") !== proxyToken) {
      return context.json({ error: "Forbidden origin" }, 403);
    }

    await next();
  });

  app.on(["GET", "POST"], "/api/auth/*", (context) => authInstance.handler(context.req.raw));

  app.get("/internal/session", async (context) => {
    if (context.req.header("x-internal-auth-token") !== internalToken) {
      return context.json({ error: "Unauthorized" }, 401);
    }

    const session = await authInstance.api.getSession({ headers: context.req.raw.headers });
    if (!session) return context.json({ error: "No active session" }, 401);
    if (!session.user.emailVerified) return context.json({ error: "Email verification required" }, 401);

    return context.json({
      user: {
        id: session.user.id,
        name: session.user.name,
        email: session.user.email,
        emailVerified: session.user.emailVerified,
      },
      session: {
        id: session.session.id,
        expiresAt: session.session.expiresAt,
      },
    });
  });

  app.get("/health", (context) => context.json({ status: "ok" }));

  return app;
}

export const app = createApp();
