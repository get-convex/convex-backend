import { AuthConfig } from "convex/server";

export default {
  providers: [
    {
      type: "customJwt",
      applicationID: "react",
      issuer: "http://localhost:3000",
      jwks: "http://localhost:3000/.well-known/jwks.json",
      algorithm: "ES256",
    },
    {
      type: "customJwt",
      // application ID (aud) is not required
      //applicationID: "react",
      issuer: "http://localhost:3000",
      jwks: "http://localhost:3000/.well-known/jwks.json",
      algorithm: "ES256",
    },
  ],
} satisfies AuthConfig;
