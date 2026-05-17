interface AuthEmailMessage {
  to: string;
  subject: string;
  text: string;
}

const isProduction = process.env.NODE_ENV === "production";

export function assertEmailDeliveryConfigured() {
  const mode = emailMode();

  if (isProduction && mode === "log") {
    throw new Error("ACCOUNTS_REPO_EMAIL_MODE=log is not allowed in production");
  }

  if (mode === "resend") {
    requireEnv("RESEND_API_KEY");
    requireEnv("ACCOUNTS_REPO_EMAIL_FROM");
  }
}

export async function sendAuthEmail(message: AuthEmailMessage) {
  const mode = emailMode();

  if (mode === "log") {
    console.info(`${message.subject} for ${message.to}: ${message.text}`);
    return;
  }

  if (mode === "resend") {
    await sendWithResend(message);
    return;
  }

  throw new Error(`Unsupported ACCOUNTS_REPO_EMAIL_MODE: ${mode}`);
}

function emailMode() {
  return process.env.ACCOUNTS_REPO_EMAIL_MODE ?? (isProduction ? "resend" : "log");
}

function requireEnv(name: string) {
  if (!process.env[name]?.trim()) {
    throw new Error(`${name} is required for email delivery`);
  }
}

async function sendWithResend(message: AuthEmailMessage) {
  const apiKey = process.env.RESEND_API_KEY?.trim();
  const from = process.env.ACCOUNTS_REPO_EMAIL_FROM?.trim();
  if (!apiKey || !from) throw new Error("Resend email delivery is not configured");

  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from,
      to: [message.to],
      subject: message.subject,
      text: message.text,
      reply_to: process.env.ACCOUNTS_REPO_EMAIL_REPLY_TO?.trim() || undefined,
    }),
  });

  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new Error(`Resend email delivery failed with ${response.status}: ${body}`);
  }
}
