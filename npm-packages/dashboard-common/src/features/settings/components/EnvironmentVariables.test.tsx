import { parseEnvVars } from "@common/features/settings/components/EnvironmentVariables";

describe("EnvironmentVariables", () => {
  describe("parseEnvVars", () => {
    it("filters comments and empty lines", () => {
      const input = `
      # Empty line for readability

      DEBUG=true
      SECRET_KEY="mysecretkey"  # Quotes can be used around any value
      `;

      const result = parseEnvVars(input);
      expect(result).toEqual([
        { name: "DEBUG", value: "true" },
        { name: "SECRET_KEY", value: "mysecretkey" },
      ]);
    });

    it("parses for one line", () => {
      const input = "DEBUG=true";

      const result = parseEnvVars(input);
      expect(result).toEqual([{ name: "DEBUG", value: "true" }]);
    });

    it("filters when lines are invalid", () => {
      const input = `
      # Empty line for readability

      no equals on this line
      `;
      const result = parseEnvVars(input);
      expect(result).toEqual(null);
    });
  });
});
