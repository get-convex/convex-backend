import { jwksDataUri } from "../authCredentials.js";

export default {
  providers: [
    {
      type: "customJwt",
      applicationID: "App 1",
      issuer: "https://issuer.example.com/1",
      jwks: jwksDataUri,
      algorithm: "RS256",
    },
    {
      type: "customJwt",
      // application ID (aud) is not required
      //applicationID: "App 2",
      issuer: "https://issuer.example.com/2",
      jwks: jwksDataUri,
      algorithm: "RS256",
    },
    {
      type: "customJwt",
      applicationID: "App 3",
      issuer: "https://issuer.example.com/3",
      jwks: "https://example.com/not/a/jwks",
      algorithm: "RS256",
    },
    {
      type: "customJwt",
      applicationID: "App 4",
      issuer: "https://issuer.example.com/4",
      jwks: "data:invalid,not-a-valid-jwks",
      algorithm: "RS256",
    },
  ],
};
