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

describe("auth debugging and insecure features", () => {
  describe("getUserIdentityDebug functionality", () => {
    let logger: Logger;
    let httpClient: ConvexHttpClient;

    beforeEach(() => {
      logger = new Logger();
      httpClient = new ConvexHttpClient(deploymentUrl, {
        logger: new Logger() as any,
      });
    });

    test("getUserIdentityDebug returns UserIdentity for valid JWT", async () => {
      const validJwt = await createSignedJWT({ name: "TestUser", email: "test@example.com" });
      httpClient.setAuth(validJwt);
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      // Should return a UserIdentity object, not an error
      expect(result).toBeDefined();
      expect(result?.name).toEqual("TestUser");
      expect(result?.email).toEqual("test@example.com");
      expect(result?.subject).toEqual("The Subject");
    });

    test("getUserIdentityDebug returns AuthError for expired JWT", async () => {
      const expiredJwt = await createSignedJWT(
        { name: "TestUser" },
        { issuedAt: "20 sec ago", expiresIn: "10 sec ago" }
      );
      httpClient.setAuth(expiredJwt);
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      // Should return structured AuthError
      expect(result).toBeDefined();
      expect(result?.code).toEqual("InvalidAuthHeader");
      expect(result?.message).toContain("Token expired");
      expect(result?.details).toBeDefined();
    });

    test("getUserIdentityDebug returns AuthError for malformed JWT", async () => {
      httpClient.setAuth("not.a.valid.jwt");
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      expect(result).toBeDefined();
      expect(result?.code).toEqual("InvalidAuthHeader");
      expect(result?.message).toContain("JWT");
      expect(result?.message).toContain("three base64-encoded parts");
    });

    test("getUserIdentityDebug returns AuthError for wrong issuer", async () => {
      const wrongIssuerJwt = await createSignedJWT(
        { name: "TestUser" },
        { issuer: "https://unknown-issuer.example.com", audience: "Unknown App" }
      );
      httpClient.setAuth(wrongIssuerJwt);
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      expect(result).toBeDefined();
      expect(result?.code).toEqual("NoAuthProvider");
      expect(result?.message).toContain("No auth provider found");
      expect(result?.message).toContain("configured providers");
    });

    test("getUserIdentityDebug returns AuthError for missing kid", async () => {
      const noKidJwt = await createSignedJWT(
        { name: "TestUser" },
        { useKid: "missing kid" }
      );
      httpClient.setAuth(noKidJwt);
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      expect(result).toBeDefined();
      expect(result?.code).toEqual("InvalidAuthHeader");
      expect(result?.message).toContain("missing a 'kid'");
    });

    test("getUserIdentityDebug returns null for no authentication", async () => {
      // Don't set any auth
      httpClient.clearAuth();
      
      const result = await httpClient.query(api.auth.getUserIdentityDebug);
      
      expect(result).toBeNull();
    });
  });

  describe("getUserIdentityInsecure functionality", () => {
    let logger: Logger;
    let httpClient: ConvexHttpClient;

    beforeEach(() => {
      logger = new Logger();
      httpClient = new ConvexHttpClient(deploymentUrl, {
        logger: new Logger() as any,
      });
    });

    test("getUserIdentityInsecure returns plaintext token for PlaintextUser", async () => {
      const plaintextToken = "my-plaintext-auth-token-12345";
      
      // This would need to be implemented - setAuthInsecure for HTTP client
      // For now, test through the direct query that uses PlaintextUser identity
      const result = await httpClient.query(api.auth.testPlaintextUserIdentity, { 
        token: plaintextToken 
      });
      
      expect(result).toEqual(plaintextToken);
    });

    test("getUserIdentityInsecure returns null for regular User identity", async () => {
      const validJwt = await createSignedJWT({ name: "TestUser" });
      httpClient.setAuth(validJwt);
      
      const result = await httpClient.query(api.auth.getUserIdentityInsecure);
      
      expect(result).toBeNull();
    });

    test("getUserIdentityInsecure returns null for System identity", async () => {
      // Test with system/admin identity (would need special test setup)
      const result = await httpClient.query(api.auth.testSystemIdentityInsecure);
      
      expect(result).toBeNull();
    });

    test("getUserIdentityInsecure returns null for no authentication", async () => {
      httpClient.clearAuth();
      
      const result = await httpClient.query(api.auth.getUserIdentityInsecure);
      
      expect(result).toBeNull();
    });
  });

  describe("PlaintextUser admin access restrictions", () => {
    let httpClient: ConvexHttpClient;

    beforeEach(() => {
      httpClient = new ConvexHttpClient(deploymentUrl, {
        logger: new Logger() as any,
      });
    });

    test("PlaintextUser cannot access admin-protected endpoints", async () => {
      // This would test that PlaintextUser identities are properly rejected
      // by admin functions - testing the change made to must_be_admin_internal
      const result = await httpClient.query(api.auth.testPlaintextUserAdminRestriction);
      
      expect(result.canAccessAdmin).toBe(false);
      expect(result.errorType).toEqual("BadDeployKey");
    });
  });
});