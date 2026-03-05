import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  EnvironmentVariables,
  parseEnvVars,
  BaseEnvironmentVariable,
} from "@common/features/settings/components/EnvironmentVariables";
import { copyTextToClipboard, toast } from "@common/lib/utils";

jest.mock("@common/lib/utils", () => ({
  copyTextToClipboard: jest.fn(),
  toast: jest.fn(),
}));

const mockCopyTextToClipboard = jest.mocked(copyTextToClipboard);
const mockToast = jest.mocked(toast);

describe("EnvironmentVariables", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockCopyTextToClipboard.mockResolvedValue(undefined);
  });

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

  describe("Copy All", () => {
    async function renderAndCopy(
      environmentVariables: BaseEnvironmentVariable[],
    ) {
      const user = userEvent.setup();
      render(
        <EnvironmentVariables<BaseEnvironmentVariable>
          environmentVariables={environmentVariables}
          updateEnvironmentVariables={async () => {}}
          hasAdminPermissions
          initEnvVar={(envVar) => envVar}
        />,
      );

      await user.click(screen.getByRole("button", { name: "Copy All" }));
    }

    it("shows a warning toast when formatter warnings are returned", async () => {
      await renderAndCopy([{ name: "CRLF_VAR", value: "line1\r\nline2" }]);

      expect(mockCopyTextToClipboard).toHaveBeenCalledWith(
        "CRLF_VAR='line1\r\nline2'",
      );
      expect(mockToast).toHaveBeenCalledWith(
        "success",
        "Environment variables copied to the clipboard.",
      );
      expect(mockToast).toHaveBeenCalledWith(
        "warning",
        expect.stringContaining("CRLF_VAR"),
      );
    });

    it("does not show a warning toast when no formatter warnings are returned", async () => {
      await renderAndCopy([{ name: "SAFE_VAR", value: "safe-value" }]);

      expect(mockCopyTextToClipboard).toHaveBeenCalledWith(
        "SAFE_VAR=safe-value",
      );
      expect(mockToast).toHaveBeenCalledWith(
        "success",
        "Environment variables copied to the clipboard.",
      );
      expect(mockToast).not.toHaveBeenCalledWith("warning", expect.anything());
    });
  });
});
