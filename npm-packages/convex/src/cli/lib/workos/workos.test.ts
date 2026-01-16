import { test, expect, describe, beforeEach, afterEach } from "vitest";
import { resolveTemplate } from "./workos.js";

describe("resolveTemplate", () => {
  const originalEnv = process.env;

  beforeEach(() => {
    // Create a clean environment for each test
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    // Restore original environment
    process.env = originalEnv;
  });

  describe("authEnv templates", () => {
    test("resolves WORKOS_CLIENT_ID when provided", () => {
      const result = resolveTemplate("${authEnv.WORKOS_CLIENT_ID}", {
        clientId: "client_123",
        apiKey: "key_456",
        environmentId: "env_789",
      });
      expect(result).toBe("client_123");
    });

    test("resolves WORKOS_API_KEY when provided", () => {
      const result = resolveTemplate("${authEnv.WORKOS_API_KEY}", {
        clientId: "client_123",
        apiKey: "key_456",
        environmentId: "env_789",
      });
      expect(result).toBe("key_456");
    });

    test("resolves WORKOS_ENVIRONMENT_ID when provided", () => {
      const result = resolveTemplate("${authEnv.WORKOS_ENVIRONMENT_ID}", {
        clientId: "client_123",
        apiKey: "key_456",
        environmentId: "env_789",
      });
      expect(result).toBe("env_789");
    });

    test("throws when WORKOS_CLIENT_ID is not available", () => {
      expect(() => {
        resolveTemplate("${authEnv.WORKOS_CLIENT_ID}", {
          apiKey: "key_456",
        });
      }).toThrow(
        "Cannot resolve template ${authEnv.WORKOS_CLIENT_ID}: WORKOS_CLIENT_ID not available",
      );
    });

    test("throws when WORKOS_API_KEY is not available", () => {
      expect(() => {
        resolveTemplate("${authEnv.WORKOS_API_KEY}", {
          clientId: "client_123",
        });
      }).toThrow(
        "Cannot resolve template ${authEnv.WORKOS_API_KEY}: WORKOS_API_KEY not available",
      );
    });

    test("throws when provisioned object is undefined", () => {
      expect(() => {
        resolveTemplate("${authEnv.WORKOS_CLIENT_ID}");
      }).toThrow(
        "Cannot resolve template ${authEnv.WORKOS_CLIENT_ID}: WORKOS_CLIENT_ID not available",
      );
    });
  });

  describe("buildEnv templates", () => {
    test("resolves environment variable when set", () => {
      process.env.MY_TEST_VAR = "test_value";
      const result = resolveTemplate("${buildEnv.MY_TEST_VAR}");
      expect(result).toBe("test_value");
    });

    test("throws when environment variable is not set", () => {
      delete process.env.NONEXISTENT_VAR;
      expect(() => {
        resolveTemplate("${buildEnv.NONEXISTENT_VAR}");
      }).toThrow(
        "Cannot resolve template ${buildEnv.NONEXISTENT_VAR}: Environment variable NONEXISTENT_VAR is not set",
      );
    });

    test("handles empty string environment variable", () => {
      process.env.EMPTY_VAR = "";
      expect(() => {
        resolveTemplate("${buildEnv.EMPTY_VAR}");
      }).toThrow(
        "Cannot resolve template ${buildEnv.EMPTY_VAR}: Environment variable EMPTY_VAR is not set",
      );
    });
  });

  describe("unknown templates", () => {
    test("throws for unknown prefix", () => {
      expect(() => {
        resolveTemplate("${unknownPrefix.FOO}");
      }).toThrow("Unknown template expression: ${unknownPrefix.FOO}");
    });

    test("throws for no prefix", () => {
      expect(() => {
        resolveTemplate("${hello}");
      }).toThrow("Unknown template expression: ${hello}");
    });

    test("throws for typo in authEnv", () => {
      expect(() => {
        resolveTemplate("${authenv.WORKOS_CLIENT_ID}"); // lowercase 'e'
      }).toThrow("Unknown template expression: ${authenv.WORKOS_CLIENT_ID}");
    });

    test("throws for typo in buildEnv", () => {
      expect(() => {
        resolveTemplate("${buildenv.MY_VAR}"); // lowercase 'e'
      }).toThrow("Unknown template expression: ${buildenv.MY_VAR}");
    });

    test("throws for unknown authEnv variable", () => {
      expect(() => {
        resolveTemplate("${authEnv.UNKNOWN_VAR}", {
          clientId: "client_123",
        });
      }).toThrow("Unknown template expression: ${authEnv.UNKNOWN_VAR}");
    });
  });

  describe("complex templates", () => {
    test("resolves multiple templates in one string", () => {
      process.env.DOMAIN = "example.com";
      const result = resolveTemplate(
        "https://${buildEnv.DOMAIN}/auth/${authEnv.WORKOS_CLIENT_ID}",
        { clientId: "client_123" },
      );
      expect(result).toBe("https://example.com/auth/client_123");
    });

    test("leaves non-template strings unchanged", () => {
      const result = resolveTemplate("https://example.com/callback");
      expect(result).toBe("https://example.com/callback");
    });

    test("handles mixed template and literal text", () => {
      process.env.PORT = "3000";
      const result = resolveTemplate("http://localhost:${buildEnv.PORT}/api");
      expect(result).toBe("http://localhost:3000/api");
    });

    test("handles adjacent templates", () => {
      process.env.PROTO = "https";
      process.env.DOMAIN = "example.com";
      const result = resolveTemplate("${buildEnv.PROTO}://${buildEnv.DOMAIN}");
      expect(result).toBe("https://example.com");
    });
  });

  describe("edge cases", () => {
    test("handles empty string input", () => {
      const result = resolveTemplate("");
      expect(result).toBe("");
    });

    test("handles string with no templates", () => {
      const result = resolveTemplate("plain text without templates");
      expect(result).toBe("plain text without templates");
    });

    test("handles malformed template syntax", () => {
      // Missing closing brace - should not be replaced
      const result = resolveTemplate("${buildEnv.UNCLOSED");
      expect(result).toBe("${buildEnv.UNCLOSED");
    });

    test("handles nested braces incorrectly", () => {
      // Nested braces don't work - the regex matches the first }
      // This actually tries to resolve an env var called "${OTHER_VAR"
      expect(() => {
        resolveTemplate("${buildEnv.${OTHER_VAR}}");
      }).toThrow(
        "Cannot resolve template ${buildEnv.${OTHER_VAR}: Environment variable ${OTHER_VAR is not set.",
      );
    });

    test("handles special characters in environment variable names", () => {
      process.env["MY-VAR-WITH-DASHES"] = "value";
      const result = resolveTemplate("${buildEnv.MY-VAR-WITH-DASHES}");
      expect(result).toBe("value");
    });

    test("handles template at the very end", () => {
      process.env.SUFFIX = "end";
      const result = resolveTemplate("prefix-${buildEnv.SUFFIX}");
      expect(result).toBe("prefix-end");
    });

    test("handles template at the very beginning", () => {
      process.env.PREFIX = "start";
      const result = resolveTemplate("${buildEnv.PREFIX}-suffix");
      expect(result).toBe("start-suffix");
    });

    test("throws with helpful message for common typos", () => {
      const provisioned = { clientId: "client_123" };

      // Wrong case
      expect(() => {
        resolveTemplate("${authEnv.workos_client_id}", provisioned);
      }).toThrow("Unknown template expression");

      // Space in variable name
      expect(() => {
        resolveTemplate("${authEnv.WORKOS CLIENT ID}", provisioned);
      }).toThrow("Unknown template expression");
    });
  });

  describe("error message quality", () => {
    test("provides clear error for missing auth credentials", () => {
      expect(() => {
        resolveTemplate("${authEnv.WORKOS_CLIENT_ID}");
      }).toThrow(/Ensure WorkOS environment is provisioned/);
    });

    test("provides clear error for missing env var", () => {
      expect(() => {
        resolveTemplate("${buildEnv.MISSING_VAR}");
      }).toThrow(/Environment variable MISSING_VAR is not set/);
    });

    test("provides suggestions for unknown templates", () => {
      expect(() => {
        resolveTemplate("${foo.bar}");
      }).toThrow(/Use \$\{buildEnv\.VAR_NAME\} for environment variables/);
    });
  });
});
