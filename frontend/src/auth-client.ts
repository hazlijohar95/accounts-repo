import { createAuthClient } from "better-auth/react";

export const authClient = createAuthClient({
  baseURL: import.meta.env.VITE_AUTH_BASE_URL ?? "",
});

export function useAuthSession() {
  if (import.meta.env.MODE === "test" || import.meta.env.VITE_DEV_AUTH_EMAIL) {
    return {
      data: {
        user: {
          id: import.meta.env.VITE_DEV_AUTH_ID ?? "test-preparer",
          name: import.meta.env.VITE_DEV_AUTH_NAME ?? "Aina Rahman",
          email: import.meta.env.VITE_DEV_AUTH_EMAIL ?? "aina@ahadvisory.test",
        },
      },
      error: null,
      isPending: false,
      refetch: async () => undefined,
    };
  }

  return authClient.useSession();
}
