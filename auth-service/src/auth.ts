import { betterAuth } from "better-auth";
import { organization } from "better-auth/plugins";
import { Pool } from "pg";
import { assertEmailDeliveryConfigured, sendAuthEmail } from "./email";

const databaseUrl = process.env.DATABASE_URL;
const isProduction = process.env.NODE_ENV === "production";

if (!databaseUrl) {
  throw new Error("DATABASE_URL is required for the Better Auth service");
}

assertProductionAuthConfig();
assertEmailDeliveryConfigured();

export const authPool = new Pool({ connectionString: databaseUrl });

export const auth = betterAuth({
  appName: "Accounts Repo",
  database: authPool,
  emailAndPassword: {
    enabled: true,
    sendResetPassword: async ({ user, url }) => {
      await sendAuthEmail({
        to: user.email,
        subject: "Reset your Accounts Repo password",
        text: `Reset your Accounts Repo password using this link: ${url}`,
      });
    },
    requireEmailVerification: true,
    autoSignIn: false,
    minPasswordLength: 12,
    revokeSessionsOnPasswordReset: true,
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url }) => {
      await sendAuthEmail({
        to: user.email,
        subject: "Verify your Accounts Repo email",
        text: `Verify your Accounts Repo email using this link: ${url}`,
      });
    },
    sendOnSignUp: true,
    sendOnSignIn: true,
  },
  trustedOrigins: (process.env.BETTER_AUTH_TRUSTED_ORIGINS ?? "http://127.0.0.1:5173,http://127.0.0.1:5179")
    .split(",")
    .map((origin) => origin.trim())
    .filter(Boolean),
  rateLimit: {
    enabled: true,
    storage: "database",
    window: 60,
    max: 120,
    customRules: {
      "/api/auth/sign-in/email": { window: 60, max: 5 },
      "/api/auth/sign-up/email": { window: 60, max: 3 },
    },
  },
  session: {
    expiresIn: 60 * 60 * 24 * 7,
    updateAge: 60 * 60 * 24,
    freshAge: 60 * 60,
    cookieCache: {
      enabled: true,
      maxAge: 60 * 5,
      strategy: "jwe",
    },
  },
  advanced: {
    useSecureCookies: isProduction,
    ipAddress: {
      ipAddressHeaders: ["x-forwarded-for", "x-real-ip", "cf-connecting-ip"],
    },
    cookiePrefix: "accounts-repo",
    defaultCookieAttributes: {
      sameSite: "lax",
      path: "/",
    },
  },
  plugins: [
    organization({
      allowUserToCreateOrganization: true,
      organizationLimit: 25,
      membershipLimit: 250,
      invitationExpiresIn: 60 * 60 * 24 * 7,
      invitationLimit: 100,
      teams: { enabled: true },
    }),
  ],
});

export type AuthSession = typeof auth.$Infer.Session;

function assertProductionAuthConfig() {
  if (!isProduction) return;

  const secret = process.env.BETTER_AUTH_SECRET?.trim();
  if (!secret || secret.length < 32 || secret.startsWith("replace-with-")) {
    throw new Error("BETTER_AUTH_SECRET must be a real 32+ character production secret");
  }

  const baseUrl = process.env.BETTER_AUTH_URL?.trim();
  if (!baseUrl?.startsWith("https://")) {
    throw new Error("BETTER_AUTH_URL must be an HTTPS production URL");
  }

  const trustedOrigins = (process.env.BETTER_AUTH_TRUSTED_ORIGINS ?? "")
    .split(",")
    .map((origin) => origin.trim())
    .filter(Boolean);
  if (trustedOrigins.length === 0 || trustedOrigins.some((origin) => !origin.startsWith("https://"))) {
    throw new Error("BETTER_AUTH_TRUSTED_ORIGINS must contain HTTPS production origins");
  }
}
