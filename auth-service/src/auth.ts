import { betterAuth } from "better-auth";
import { organization } from "better-auth/plugins";
import { Pool } from "pg";

const databaseUrl = process.env.DATABASE_URL;

if (!databaseUrl) {
  throw new Error("DATABASE_URL is required for the Better Auth service");
}

export const authPool = new Pool({ connectionString: databaseUrl });

export const auth = betterAuth({
  appName: "Accounts Repo",
  database: authPool,
  emailAndPassword: {
    enabled: true,
    minPasswordLength: 12,
    revokeSessionsOnPasswordReset: true,
  },
  emailVerification: {
    sendVerificationEmail: async ({ user, url }) => {
      console.info(`Verification email for ${user.email}: ${url}`);
    },
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
