import {
  InitAuth0,
  WithPageAuthRequiredPageRouterOptions,
  WithPageAuthRequiredPageRouter,
  initAuth0,
} from "@auth0/nextjs-auth0";

let instance: ReturnType<InitAuth0>;

// This is a custom config, on top of things configured via environment variables.
// See https://auth0.github.io/nextjs-auth0/types/config.ConfigParameters.html
export function auth0() {
  if (instance !== undefined) {
    return instance;
  }

  instance = initAuth0({
    session: {
      rollingDuration: 7 * 24 * 60 * 60, // 7 days in seconds
    },
  });
  return instance;
}

// We need this wrapper so that auth0() isn't called during
// static build, which would require all the environment variables
// we currently only require in prod at runtime (and store in Vercel/CI only).
export function withPageAuthRequired(
  options: WithPageAuthRequiredPageRouterOptions,
): ReturnType<WithPageAuthRequiredPageRouter> {
  return async (ctx) => auth0().withPageAuthRequired(options)(ctx);
}
