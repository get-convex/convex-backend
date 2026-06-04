// BetterAuth server instance for dashboard-orchestrator.
// Backed by the same Postgres database the orchestrator uses; BetterAuth
// owns the `user`, `session`, `account`, and `verification` tables.

import { betterAuth } from "better-auth";
import { Pool } from "pg";

const databaseUrl =
  process.env.BETTER_AUTH_DATABASE_URL ??
  process.env.CONVEX_ORCHESTRATOR_DATABASE_URL;

if (!databaseUrl) {
  throw new Error(
    "Missing BETTER_AUTH_DATABASE_URL (or CONVEX_ORCHESTRATOR_DATABASE_URL) — required for BetterAuth's user store.",
  );
}

const baseURL = process.env.BETTER_AUTH_URL || "http://localhost:6792";

// Best-effort email send. If the operator has set BETTER_AUTH_SMTP_URL we
// dispatch through nodemailer; otherwise we log the link to stdout so a dev
// can copy-paste it. Production operators are expected to wire up SMTP.
async function sendAuthEmail(opts: {
  to: string;
  subject: string;
  text: string;
}): Promise<void> {
  const smtpUrl = process.env.BETTER_AUTH_SMTP_URL;
  if (!smtpUrl) {
    console.warn(
      `[dashboard-orchestrator] No BETTER_AUTH_SMTP_URL set; emailing ${opts.to} via console:\n${opts.subject}\n${opts.text}`,
    );
    return;
  }
  try {
    // Dynamic import keeps nodemailer out of the bundle when unused. The
    // package is optional — operators install it locally when they want
    // SMTP delivery — so we cast through any to avoid forcing every build
    // to install the @types/nodemailer types.
    const mod = (await import(
      /* webpackIgnore: true */ "nodemailer" as any
    )) as {
      default: {
        createTransport(url: string): {
          sendMail(opts: {
            from?: string;
            to: string;
            subject: string;
            text: string;
          }): Promise<unknown>;
        };
      };
    };
    const transport = mod.default.createTransport(smtpUrl);
    await transport.sendMail({
      from: process.env.BETTER_AUTH_SMTP_FROM ?? "no-reply@orchestrator",
      to: opts.to,
      subject: opts.subject,
      text: opts.text,
    });
    // No-op consumer of nodemailer.createTransport's return so eslint
    // doesn't flag the ignored unused identifier.
  } catch (err) {
    console.error("[dashboard-orchestrator] SMTP send failed", err);
  }
}

export const auth = betterAuth({
  database: new Pool({
    connectionString: databaseUrl,
    // PlanetScale + most managed Postgres providers require TLS.
    ssl: databaseUrl.includes("sslmode=require")
      ? { rejectUnauthorized: false }
      : undefined,
  }),
  baseURL,
  secret: process.env.BETTER_AUTH_SECRET,
  // Default `useSecureCookies` follows NODE_ENV — true in production, which
  // makes the session cookie `Secure` and the browser silently drops it on
  // `http://localhost`. We trust the URL scheme instead so HTTP self-host
  // setups (and Firefox private mode) get a usable cookie. HTTPS deployments
  // still get hardened cookies because baseURL is https://.
  advanced: {
    useSecureCookies: baseURL.startsWith("https://"),
  },
  emailAndPassword: {
    enabled: true,
    autoSignIn: true,
    minPasswordLength: 8,
    // Off by default so first-time setup doesn't lock the operator out of
    // their own dashboard. Set BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION=1 to
    // turn this on once SMTP is wired up.
    requireEmailVerification:
      process.env.BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION === "1",
    sendResetPassword: async ({ user, url }) => {
      await sendAuthEmail({
        to: user.email,
        subject: "Reset your Convex orchestrator password",
        text: `Hi ${user.name ?? user.email},\n\nClick the link below to reset your password. The link expires in 1 hour.\n\n${url}\n\nIf you didn't request this, you can ignore the message.`,
      });
    },
  },
  emailVerification: {
    sendOnSignUp: process.env.BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION === "1",
    autoSignInAfterVerification: true,
    sendVerificationEmail: async ({ user, url }) => {
      await sendAuthEmail({
        to: user.email,
        subject: "Verify your email for Convex orchestrator",
        text: `Hi ${user.name ?? user.email},\n\nClick the link below to verify your email address.\n\n${url}\n\n`,
      });
    },
  },
  socialProviders: {
    ...(process.env.GITHUB_CLIENT_ID && process.env.GITHUB_CLIENT_SECRET
      ? {
          github: {
            clientId: process.env.GITHUB_CLIENT_ID,
            clientSecret: process.env.GITHUB_CLIENT_SECRET,
          },
        }
      : {}),
    ...(process.env.GOOGLE_CLIENT_ID && process.env.GOOGLE_CLIENT_SECRET
      ? {
          google: {
            clientId: process.env.GOOGLE_CLIENT_ID,
            clientSecret: process.env.GOOGLE_CLIENT_SECRET,
          },
        }
      : {}),
  },
  trustedOrigins: [baseURL],
});

export type Session = typeof auth.$Infer.Session;
