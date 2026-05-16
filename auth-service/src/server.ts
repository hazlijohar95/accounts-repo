import { serve } from "@hono/node-server";
import { app } from "./app";

const port = Number(process.env.AUTH_PORT ?? 8081);

serve({ fetch: app.fetch, port }, (info) => {
  console.info(`accounts repo auth service listening on http://127.0.0.1:${info.port}`);
});
