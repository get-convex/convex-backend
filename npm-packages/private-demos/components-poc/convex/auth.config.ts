import { AuthConfig } from "convex/server";

export default {
  providers: [
    {
      domain: "https://dev-1rfqpgu8.us.auth0.com/",
      applicationID: "convex",
    },
  ],
} satisfies AuthConfig;
