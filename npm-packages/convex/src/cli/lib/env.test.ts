import { test, expect, describe } from "vitest";
import * as dotenv from "dotenv";
import { formatEnvValueForDotfile } from "./env.js";

/**
 * Attempts a round-trip: format the value for dotenv, then parse it back.
 */
function roundTrip(
  originalValue: string,
  name = "TEST_VAR",
): {
  success: boolean;
  parsedValue: string | undefined;
  formatted: string;
  warning: string | undefined;
  envFileContent: string;
} {
  const { formatted, warning } = formatEnvValueForDotfile(originalValue);
  const envFileContent = `${name}=${formatted}`;
  const parsed = dotenv.parse(envFileContent);
  const parsedValue = parsed[name];
  const success = parsedValue === originalValue;
  return { success, parsedValue, formatted, warning, envFileContent };
}

/** Values that should round-trip successfully through formatEnvValueForDotfile -> dotenv.parse */
const ROUND_TRIP_CASES: [string, string][] = [
  // Basic values
  ["simple string", "hello"],
  ["string with spaces", "hello world"],
  ["empty string", ""],
  ["numeric string", "12345"],

  // Newlines
  ["newline", "first\nsecond"],
  ["newline with trailing", "line1\nline2\n"],
  ["multiple newlines", "line1\nline2\nline3"],
  ["only newlines", "\n\n\n"],
  ["newline + single quotes", "first's\nsecond's"],
  ["newline + literal \\n", "first\\n\nsec\\nond"],

  // Quotes
  ["wrapped in single quotes", "'single'"],
  ["wrapped in double quotes", '"double"'],
  ["nested quotes: single containing double", "'single \"and\" double'"],
  ["nested quotes: double containing single", "\"double 'and' single\""],
  ["single quote in middle", "it's a test"],
  ["double quote in middle", 'say "hello"'],
  ["starts with single quote", "'starts"],
  ["ends with single quote", "ends'"],
  ["starts with double quote", '"starts'],
  ["ends with double quote", 'ends"'],
  ["both quote types", 'it\'s a "test"'],
  ["both quote types + newline", 'it\'s a "test"\nline2'],

  // Hash/comment character
  ["hash in middle", "before # after"],
  ["hash + newlines", "first # after\nsecond # after"],
  ["hash at start", "#hashtag"],
  ["hash + single quote", "before#'after'"],
  ["hash + double quote", 'before#"after"'],
  ["hash + newline + single quote", "first # 'after'\nsecond # 'after'"],

  // Tabs and whitespace
  ["tab", "hello\tworld"],
  ["whitespace padding + newline", "  both  \n  sides  "],

  // Control characters
  ["formfeed", "\f"],
  ["vertical tab", "\v"],
  ["escape char", "\x1b"],
  ["bell", "\x07"],
  ["DEL", "\x7f"],
  ["ANSI color sequence", "\x1b[31mred?\x1b[0m"],
  ["null byte", "before\x00after"],
  ["mixed control bytes", "\x01\x02\x03ABC\x7f\x1b"],

  // Backticks
  ["backticks wrapping", "`command`"],
  ["backticks in middle", "run `command` here"],

  // Backslashes
  ["backslash path", "path\\to\\file"],
  ["literal \\n", "hello\\nworld"],
  ["literal \\n + single quote", "it's\\nhello"],
  ["literal escape sequences", "backslash: \\n \\t \\r \\0 \\x41"],

  // Dollar signs
  ["dollar sign variable", "$HOME/path"],
  ["dollar sign braces", "${HOME}/path"],

  // Equals sign
  ["equals in value", "key=value=extra"],

  // Unicode
  ["unicode", "hello 世界 🌍"],
  ["emoji", "🎉🎊🎁"],

  // JSON
  ["JSON object", '{"key": "value", "nested": {"a": 1}}'],
  ["JSON multiline", '{\n  "pretty": true,\n  "indent": 2\n}\n'],
  ["JSON with \\n in string", '{"multiline":"line1\\nline2"}'],

  // Config formats
  ["INI format", "key=value\nother=two\n"],
  ["YAML", 'a: 1\nb: "two"\nc:\n  - x\n  - y\n'],

  // Real-world
  [
    "PEM private key",
    "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n-----END PRIVATE KEY-----",
  ],
  ["URL with hash anchor", "https://example.com/path?key=value&foo=bar#anchor"],
  ["URL with encoded chars", "redis://:p%40ss@127.0.0.1:6379/0"],
  ["base64", "SGVsbG8gV29ybGQh"],
  [
    "JWT",
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
  ],
  ["SQL with quotes", "SELECT * FROM users WHERE name = 'John' AND age > 18"],
  ["regex", "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"],
];

/**
 * Known limitations - these cannot round-trip due to dotenv parser constraints.
 * Format: [name, value, reason]
 */
const KNOWN_FAILURES: [string, string, string][] = [
  // Carriage returns - dotenv strips \r
  ["CRLF", "line1\r\nline2", "dotenv strips \\r"],
  ["CR only", "classicmac\rline2\r", "dotenv strips \\r"],
  ["CRLF multi-line", "windows\r\nline2\r\n", "dotenv strips \\r"],
  ["mixed \\n\\t\\r", "escapes:\n\t\r", "dotenv strips \\r"],

  // Impossible quoting scenarios
  [
    "newline + literal \\n + single quote",
    "it's\\n\nsec\\nond",
    "conflicting escape requirements",
  ],
  [
    "hash + newline + both quotes",
    "first # 'after'\nsecond # \"after\"",
    "no quoting strategy protects hash with both quote types",
  ],
  [
    "hash + literal \\n + single quote",
    "value # 'test'\\n",
    "single quotes needed for \\n but break on inner quotes",
  ],
];

describe("formatEnvValueForDotfile", () => {
  describe("round-trip tests", () => {
    describe("values that round-trip", () => {
      test.each(ROUND_TRIP_CASES)("%s", (_, value) => {
        expect(roundTrip(value).success).toBe(true);
      });
    });

    describe("known limitations", () => {
      test.each(KNOWN_FAILURES)("%s (%s)", (_, value) => {
        expect(roundTrip(value).success).toBe(false);
      });
    });
  });

  describe("formatted output", () => {
    test("newline uses single quotes", () => {
      expect(roundTrip("first\nsecond").formatted).toBe("'first\nsecond'");
    });

    test("newline + single quotes uses double quotes with escaped newlines", () => {
      expect(roundTrip("first's\nsecond's").formatted).toBe(
        "\"first's\\nsecond's\"",
      );
    });

    test("wrapped single quotes uses double quote wrapper", () => {
      expect(roundTrip("'single'").formatted).toBe("\"'single'\"");
    });

    test("wrapped double quotes uses single quote wrapper", () => {
      expect(roundTrip('"double"').formatted).toBe("'\"double\"'");
    });

    test("hash uses single quotes", () => {
      expect(roundTrip("before # after").formatted).toBe("'before # after'");
    });

    test("hash + single quote uses double quotes", () => {
      expect(roundTrip("before#'after'").formatted).toBe("\"before#'after'\"");
    });
  });

  describe("warnings", () => {
    test("no warning for simple hash", () => {
      expect(roundTrip("api_key # secret").warning).toBeUndefined();
    });

    test("warns about newline + single quotes + literal \\n", () => {
      expect(formatEnvValueForDotfile("it's\ncomplex\\n").warning).toContain(
        "may not round-trip",
      );
    });

    test("warns about unprotectable hash", () => {
      expect(
        formatEnvValueForDotfile("first # 'a'\nsecond # \"b\"").warning,
      ).toContain("#");
    });

    test("warns about carriage return", () => {
      expect(formatEnvValueForDotfile("line1\r\nline2").warning).toContain(
        "carriage return",
      );
    });
  });

  describe("permutation matrix", () => {
    interface Flags {
      newline: boolean;
      hash: boolean;
      slashN: boolean;
      single: boolean;
      double: boolean;
    }

    const build = (f: Flags): string => {
      let v = "value";
      if (f.newline) v += "\n";
      if (f.hash) v += " # comment";
      if (f.slashN) v += "\\n";
      if (f.single) v += "'q'";
      if (f.double) v += '"q"';
      return v;
    };

    const describe_ = (f: Flags): string => {
      const p: string[] = [];
      if (f.newline) p.push("newline");
      if (f.hash) p.push("hash");
      if (f.slashN) p.push("\\n");
      if (f.single) p.push("'");
      if (f.double) p.push('"');
      return p.length ? p.join("+") : "plain";
    };

    // Patterns that cannot round-trip
    const badPatterns: Partial<Flags>[] = [
      { newline: true, slashN: true, single: true },
      { hash: true, single: true, double: true },
      { hash: true, slashN: true, single: true },
    ];

    const matches = (f: Flags, p: Partial<Flags>) =>
      (!p.newline || f.newline) &&
      (!p.hash || f.hash) &&
      (!p.slashN || f.slashN) &&
      (!p.single || f.single) &&
      (!p.double || f.double);

    const isBad = (f: Flags) => badPatterns.some((p) => matches(f, p));

    const all: Flags[] = [];
    for (let i = 0; i < 32; i++) {
      all.push({
        newline: !!(i & 1),
        hash: !!(i & 2),
        slashN: !!(i & 4),
        single: !!(i & 8),
        double: !!(i & 16),
      });
    }

    describe("supported", () => {
      test.each(all.filter((f) => !isBad(f)).map((f) => [describe_(f), f]))(
        "%s",
        (_, f) => expect(roundTrip(build(f as Flags)).success).toBe(true),
      );
    });

    describe("unsupported", () => {
      test.each(all.filter(isBad).map((f) => [describe_(f), f]))("%s", (_, f) =>
        expect(roundTrip(build(f as Flags)).success).toBe(false),
      );
    });
  });
});
