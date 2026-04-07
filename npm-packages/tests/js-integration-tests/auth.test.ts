import { decodeJwt, importPKCS8, SignJWT } from "jose";
import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";
import { privateKeyPEM, kid as correctKid } from "./authCredentials";

async function createSignedJWT(
  payload: any,
  options: {
    issuer?: string | null;
    audience?: string | null;
    expiresIn?: string;
    subject?: string;
    issuedAt?: string | null;
    alg?: "RS256" | "ES256" | (string & { ignore_me?: never });
    useKid?: "wrong kid" | "missing kid" | "correct kid";
  } = {},
) {
  const privateKey = await importPKCS8(privateKeyPEM, "RS256");
  const {
    issuer = "https://issuer.example.com/1",
    audience = "App 1",
    expiresIn = "1h",
    subject = "The Subject",
    alg = "RS256",
    issuedAt = "10 sec ago",
    useKid = "correct kid",
  } = options;

  const kid: string | undefined =
    useKid === "correct kid"
      ? correctKid
      : useKid === "wrong kid"
        ? "key-2 (oops, this is the wrong kid!)"
        : undefined;

  let jwtBuilder = new SignJWT(payload).setProtectedHeader({
    kid,
    alg,
  });

  if (issuedAt !== null) {
    jwtBuilder = jwtBuilder.setIssuedAt(issuedAt);
  }

  if (issuer !== null) {
    jwtBuilder = jwtBuilder.setIssuer(issuer);
  }

  if (audience !== null) {
    jwtBuilder = jwtBuilder.setAudience(audience);
  }

  const jwt = await jwtBuilder
    .setSubject(subject)
    .setExpirationTime(expiresIn)
    .sign(privateKey);

  const decodedPayload = decodeJwt(jwt);
  console.log(decodedPayload);

  return jwt;
}

class Logger {
  logs: any[][];
  constructor() {
    this.logs = [];
  }

  logVerbose() {}

  log(...args: any[]) {
    this.logs.push(args);
  }

  warn(...args: any[]) {
    this.logs.push(args);
  }

  error(...args: any[]) {
    this.logs.push(args);
  }
}

describe("auth debugging", () => {
  describe("http client", () => {
    let logger: Logger;
    let httpClient: ConvexHttpClient;

    async function getErrorFromJwt(jwt: string): Promise<{
      code: string;
      message: string;
    }> {
      httpClient.setAuth(jwt);
      let err: any;
      try {
        await httpClient.query(api.auth.q);
        throw new Error("expected an error to be thrown");
      } catch (e: any) {
        err = e as { code: string; message: string; name: string };
      }
      const error = JSON.parse(err.message) as {
        code: string;
        message: string;
      };
      return error;
    }

    beforeEach(() => {
      logger = new Logger();
      httpClient = new ConvexHttpClient(deploymentUrl, {
        logger: new Logger() as any,
      });
    });

    test("jwt working", async () => {
      httpClient.setAuth(await createSignedJWT({ name: "Presley" }));
      const result = await httpClient.query(api.auth.q);
      expect(result?.name).toEqual("Presley");
      expect(logger.logs).toEqual([]);
    });

    test("no auth provider found - enhanced error message", async () => {
      const error = await getErrorFromJwt(
        await createSignedJWT(
          { name: "Presley" },
          {
            issuer: "https://unknown-issuer.example.com",
            audience: "Unknown App",
          },
        ),
      );
      expect(error.code).toEqual("NoAuthProvider");
      // Check that the enhanced error message includes configured providers
      expect(error.message).toContain(
        "No auth provider found matching the given token",
      );
      expect(error.message).toContain("configured providers");
      expect(error.message).toContain(
        "CustomJWT(issuer=https://issuer.example.com/1, app_id=App 1)",
      );
      expect(error.message).toContain(
        "CustomJWT(issuer=https://issuer.example.com/no-aud-specified, app_id=none)",
      );
      expect(error.message).toContain(
        "CustomJWT(issuer=https://issuer.example.com/3, app_id=App 3)",
      );
      expect(logger.logs).toEqual([]);
    });

    test("missing issuer claim", async () => {
      const jwt = await createSignedJWT(
        { name: "Presley" },
        {
          issuer: null,
        },
      );
      const error = await getErrorFromJwt(jwt);
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("issuer");
      expect(error.message).toContain("iss");
    });

    test("missing audience claim", async () => {
      const jwt = await createSignedJWT(
        { name: "Presley" },
        {
          audience: null,
        },
      );
      const error = await getErrorFromJwt(jwt);
      expect(error.code).toEqual("NoAuthProvider");
    });

    test("missing kid", async () => {
      const error = await getErrorFromJwt(
        await createSignedJWT({ name: "Presley" }, { useKid: "missing kid" }),
      );
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("missing a 'kid'");
      expect(logger.logs).toEqual([]);
    });

    test("wrong audience claim", async () => {
      const jwt = await createSignedJWT(
        { name: "Presley" },
        {
          audience: "asdf",
        },
      );
      const error = await getErrorFromJwt(jwt);
      expect(error.code).toEqual("NoAuthProvider");
    });

    test("audience claim allowed when none required", async () => {
      const jwt = await createSignedJWT(
        { name: "Presley" },
        {
          issuer: "https://issuer.example.com/no-aud-specified",
          audience: "asdf",
        },
      );
      httpClient.setAuth(jwt);
      const result = await httpClient.query(api.auth.q);
      expect(result?.name).toEqual("Presley");
    });

    test("missing audience claim allowed when none required", async () => {
      const jwt = await createSignedJWT(
        { name: "Presley" },
        {
          issuer: "https://issuer.example.com/no-aud-specified",
          audience: null,
        },
      );
      httpClient.setAuth(jwt);
      const result = await httpClient.query(api.auth.q);
      expect(result?.name).toEqual("Presley");
    });

    test("wrong kid", async () => {
      const error = await getErrorFromJwt(
        await createSignedJWT({ name: "Presley" }, { useKid: "wrong kid" }),
      );
      // Should get some kind of verification or decoding error
      expect(["InvalidAuthHeader", "NoAuthProvider"].includes(error.code)).toBe(
        true,
      );
      expect(error.message).toContain("Could not decode token");
      expect(error.message).toContain("kid");
      expect(error.message).toContain("key-2 (oops, this is the wrong kid!)");
    });

    test("malformed JWT", async () => {
      const error = await getErrorFromJwt("not.a.jwt");
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("JWT");
      expect(error.message).toContain("three base64-encoded parts");
    });

    // Integration tests that hit real APIs are a bummer, TODO hit something else
    // eslint-disable-next-line jest/no-disabled-tests
    test.skip("unreachable JWKS URL", async () => {
      // Use App 3 which has a non-existent JWKS URL
      const error = await getErrorFromJwt(
        await createSignedJWT(
          { name: "Presley" },
          {
            issuer: "https://issuer.example.com/3",
            audience: "App 3",
          },
        ),
      );
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("JWKS");
      expect(error.message).toContain("URL");
      expect(error.message).toContain("accessible");
    });

    test("invalid JWKS data URI", async () => {
      // Use App 4 which has an invalid data URI for JWKS
      const error = await getErrorFromJwt(
        await createSignedJWT(
          { name: "Presley" },
          {
            issuer: "https://issuer.example.com/4",
            audience: "App 4",
          },
        ),
      );
      expect(error.code).toEqual("InvalidAuthHeader");
      // Should get some kind of JWKS or parsing error
      expect(error.message).toContain("Invalid JWKS response body");
      expect(error.message).toContain("not valid JSON");
    });

    test("token expired 10 seconds ago", async () => {
      const error = await getErrorFromJwt(
        await createSignedJWT(
          { name: "Presley" },
          { issuedAt: "20 sec ago", expiresIn: "10 sec ago" },
        ),
      );
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("Token expired");
      expect(error.message).toContain("seconds ago");
    });

    test("token issued 3 seconds in future", async () => {
      // Should succeed with 5-second tolerance
      httpClient.setAuth(
        await createSignedJWT(
          { name: "Presley" },
          { issuedAt: "3 sec from now" },
        ),
      );
      const result = await httpClient.query(api.auth.q);
      expect(result?.subject).toEqual("The Subject");
      expect(result?.name).toEqual("Presley");
    });

    test("token issued 10 seconds in future", async () => {
      const error = await getErrorFromJwt(
        await createSignedJWT(
          { name: "Presley" },
          { issuedAt: "10 sec from now" },
        ),
      );
      expect(error.code).toEqual("InvalidAuthHeader");
      expect(error.message).toContain("will be valid in");
    });

    // Not recommended (some client logic may expect an iat) but not required.
    test("missing iat", async () => {
      httpClient.setAuth(
        await createSignedJWT({ name: "Presley" }, { issuedAt: null }),
      );
      const result = await httpClient.query(api.auth.q);
      expect(result?.subject).toEqual("The Subject");
      expect(result?.name).toEqual("Presley");
    });
  });
});
