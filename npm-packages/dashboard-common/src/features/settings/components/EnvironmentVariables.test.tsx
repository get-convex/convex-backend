import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { isValidElement } from "react";
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
          envVarKey={(envVar) => `${envVar.name}-${envVar.value}`}
        />,
      );

      await user.click(screen.getByRole("button", { name: "Copy All" }));
    }

    function getWarningToastMessageElement() {
      const warningCall = mockToast.mock.calls.find(
        ([type]) => type === "warning",
      );
      expect(warningCall).toBeDefined();

      const warningMessage = warningCall?.[1];
      expect(isValidElement(warningMessage)).toBe(true);
      if (!isValidElement(warningMessage)) {
        throw new Error("Expected warning toast to render a React element");
      }
      return warningMessage;
    }

    it("shows a warning toast when formatter warnings are returned", async () => {
      await renderAndCopy([{ name: "CRLF_VAR", value: "line1\r\nline2" }]);

      expect(mockCopyTextToClipboard).toHaveBeenCalledWith(
        "CRLF_VAR='line1\r\nline2'",
      );
      expect(mockToast).not.toHaveBeenCalledWith(
        "success",
        "Environment variables copied to the clipboard.",
      );

      const warningMessage = getWarningToastMessageElement();
      render(<>{warningMessage}</>);
      expect(
        screen.getByText(
          "Environment variables copied to the clipboard with the following warnings:",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByText("CRLF_VAR", { selector: "code" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("listitem")).toHaveLength(1);
    });

    it("renders each warning on its own line, including duplicate names", async () => {
      await renderAndCopy([
        { name: "DUP_VAR", value: "line1\r\nline2" },
        { name: "DUP_VAR", value: "line3\rline4" },
      ]);

      const warningMessage = getWarningToastMessageElement();
      const { container } = render(<>{warningMessage}</>);
      expect(
        screen.getByText(
          "Environment variables copied to the clipboard with the following warnings:",
        ),
      ).toBeInTheDocument();
      expect(screen.getAllByText("DUP_VAR", { selector: "code" })).toHaveLength(
        2,
      );
      expect(container.querySelectorAll("code")).toHaveLength(2);
      expect(screen.getAllByRole("listitem")).toHaveLength(2);
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
