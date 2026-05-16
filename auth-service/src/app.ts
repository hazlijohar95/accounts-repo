import { Hono } from "hono";
import { cors } from "hono/cors";
import { auth } from "./auth";

type AuthInstance = typeof auth;

export function createApp(authInstance: AuthInstance = auth) {
  const app = new Hono();
  const frontendOrigins = (process.env.BETTER_AUTH_TRUSTED_ORIGINS ?? "http://127.0.0.1:5173,http://127.0.0.1:5179")
    .split(",")
    .map((origin) => origin.trim())
    .filter(Boolean);
  const internalToken = process.env.AUTH_INTERNAL_TOKEN ?? "development-internal-token";

  app.use(
    "*",
    cors({
      origin: (origin) => (frontendOrigins.includes(origin) ? origin : frontendOrigins[0]),
      credentials: true,
      allowHeaders: ["content-type", "cookie", "x-internal-auth-token"],
      allowMethods: ["GET", "POST", "OPTIONS"],
    }),
  );

  app.on(["GET", "POST"], "/api/auth/*", (context) => authInstance.handler(context.req.raw));

  app.get("/internal/session", async (context) => {
    if (context.req.header("x-internal-auth-token") !== internalToken) {
      return context.json({ error: "Unauthorized" }, 401);
    }

    const session = await authInstance.api.getSession({ headers: context.req.raw.headers });
    if (!session) return context.json({ error: "No active session" }, 401);

    return context.json({
      user: {
        id: session.user.id,
        name: session.user.name,
        email: session.user.email,
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
