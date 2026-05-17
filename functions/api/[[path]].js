export async function onRequest({ request, env }) {
  const requestUrl = new URL(request.url);
  const targetOrigin = requestUrl.pathname.startsWith("/api/auth/")
    ? env.ACCOUNTS_REPO_AUTH_ORIGIN
    : env.ACCOUNTS_REPO_API_ORIGIN;

  if (!targetOrigin) {
    return new Response("Cloudflare proxy origin is not configured", { status: 500 });
  }

  const upstreamUrl = new URL(`${requestUrl.pathname}${requestUrl.search}`, normalizeOrigin(targetOrigin));
  const headers = new Headers(request.headers);
  headers.delete("host");
  headers.set("x-forwarded-host", requestUrl.host);
  headers.set("x-forwarded-proto", "https");

  if (env.ACCOUNTS_REPO_PROXY_TOKEN) {
    headers.set("x-accounts-repo-proxy-token", env.ACCOUNTS_REPO_PROXY_TOKEN);
  }

  const init = {
    method: request.method,
    headers,
    redirect: "manual",
  };

  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = request.body;
  }

  return fetch(new Request(upstreamUrl, init));
}

function normalizeOrigin(origin) {
  return origin.endsWith("/") ? origin : `${origin}/`;
}
