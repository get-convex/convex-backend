import { render } from "@testing-library/react";
import { DeploymentResponse } from "generatedApi";
import userEvent from "@testing-library/user-event";
import { useConfigurePeriodicBackup } from "api/backups";
import { BackupScheduleSelector, BackupScheduleSelectorInner } from "./Backups";

const deployment: DeploymentResponse = {
  kind: "cloud",
  id: 1,
  name: "joyful-capybara-123",
  deploymentType: "prod",
  createTime: Date.now(),
  projectId: 1,
  creator: 1,
  previewIdentifier: null,
};

jest.mock("api/profile", () => {});
jest.mock("api/backups", () => ({
  useConfigurePeriodicBackup: jest.fn().mockReturnValue(jest.fn()),
}));
jest.mock("api/vanityDomains", () => ({}));
jest.mock("api/usage", () => ({}));
jest.mock("api/billing", () => ({}));
jest.mock("api/environmentVariables", () => ({}));
jest.mock("api/roles", () => {});
jest.mock("api/teams", () => {});
jest.mock("api/projects", () => {});
jest.mock("api/deployments", () => {});

beforeEach(() => {
  jest.useFakeTimers().setSystemTime(new Date("2024-11-04"));
});

describe("BackupScheduleSelector", () => {
  it("displays time correctly on the button for a negative UTC offset timezone", () => {
    jest.spyOn(global.Intl, "DateTimeFormat").mockImplementationOnce(
      (locale, options) =>
        new Intl.DateTimeFormat(locale, {
          ...options,
          timeZone: "America/New_York", // UTC-4
        }),
    );

    const { getByText } = render(
      <BackupScheduleSelector
        cronspec="41 9 * * *"
        deployment={deployment}
        disabled={false}
      />,
    );

    expect(getByText(/04:41 AM/i)).toBeInTheDocument();
  });

  it("displays time correctly on the button for a negative UTC offset timezone 2", () => {
    jest.spyOn(global.Intl, "DateTimeFormat").mockImplementationOnce(
      (locale, options) =>
        new Intl.DateTimeFormat(locale, {
          ...options,
          timeZone: "Australia/Adelaide", // UTC+10:30
        }),
    );

    const { getByText } = render(
      <BackupScheduleSelector
        cronspec="41 9 * * *"
        deployment={deployment}
        disabled={false}
      />,
    );

    expect(getByText(/08:11 PM/i)).toBeInTheDocument();
  });
});

describe("BackupScheduleSelectorInner", () => {
  it("displays time correctly in the time selector form for a negative UTC offset timezone", () => {
    const date = new Date();
    date.setUTCHours(9, 41);

    jest.spyOn(date, "getHours").mockReturnValue(
      +date.toLocaleTimeString("en-GB", {
        hour: "numeric",
        timeZone: "America/New_York",
      }),
    );

    const { getByDisplayValue } = render(
      <BackupScheduleSelectorInner
        defaultValue={date}
        defaultPeriodicity="daily"
        defaultDayOfWeek={0}
        onClose={() => {}}
        deployment={deployment}
      />,
    );

    expect(getByDisplayValue("04:41")).toBeInTheDocument();
  });

  it("displays time correctly in the time selector form for a negative UTC offset timezone", () => {
    const date = new Date();
    date.setUTCHours(9, 41);

    // New York time
    jest.spyOn(date, "getHours").mockReturnValue(4);
    jest.spyOn(date, "getMinutes").mockReturnValue(41);

    const { getByDisplayValue } = render(
      <BackupScheduleSelectorInner
        defaultValue={date}
        defaultPeriodicity="daily"
        defaultDayOfWeek={0}
        onClose={() => {}}
        deployment={deployment}
      />,
    );

    expect(getByDisplayValue("04:41")).toBeInTheDocument();
  });

  it("displays time correctly in the time selector form for a positive UTC offset timezone", () => {
    const date = new Date();
    date.setUTCHours(9, 41);

    // Adelaide time
    jest.spyOn(date, "getHours").mockReturnValue(20);
    jest.spyOn(date, "getMinutes").mockReturnValue(11);

    const { getByDisplayValue } = render(
      <BackupScheduleSelectorInner
        defaultValue={date}
        defaultPeriodicity="daily"
        defaultDayOfWeek={0}
        onClose={() => {}}
        deployment={deployment}
      />,
    );

    expect(getByDisplayValue("20:11")).toBeInTheDocument();
  });

  it("allows the users to set a time from the local timezone", async () => {
    const user = userEvent.setup({
      // https://github.com/testing-library/user-event/issues/833#issuecomment-1013632841
      delay: null,
    });
    const { getByLabelText, getByRole } = render(
      <BackupScheduleSelectorInner
        defaultValue={new Date()}
        defaultPeriodicity="daily"
        defaultDayOfWeek={0}
        onClose={() => {}}
        deployment={deployment}
      />,
    );

    const input = getByLabelText(/Time \(.+\)/);
    const submit = getByRole("button");

    expect(submit).toBeDisabled();

    await user.clear(input);
    await user.type(input, "09:41");

    expect(submit).toBeEnabled();
    await user.click(submit);

    const [utcHour, utcMinute] = new Date("2024-11-06 09:41")
      .toLocaleTimeString("en-GB", {
        hour: "numeric",
        minute: "numeric",
        timeZone: "UTC",
      })
      .split(":");
    expect(useConfigurePeriodicBackup()).toBeCalledWith({
      cronspec: `${+utcMinute} ${+utcHour} * * *`,
    });
  });

  it("allows the users to set a weekly time including selected day-of-week", async () => {
    const user = userEvent.setup({ delay: null });
    const selectedDow = 2;
    const { getByLabelText, getByText } = render(
      <BackupScheduleSelectorInner
        defaultValue={new Date()}
        defaultPeriodicity="weekly"
        defaultDayOfWeek={selectedDow}
        onClose={() => {}}
        deployment={deployment}
      />,
    );

    const input = getByLabelText(/Time \(.+\)/);
    const submit = getByText("Change");

    await user.clear(input);
    await user.type(input, "09:41");
    await user.click(submit);

    const [utcHour, utcMinute] = new Date("2024-11-06 09:41")
      .toLocaleTimeString("en-GB", {
        hour: "numeric",
        minute: "numeric",
        timeZone: "UTC",
      })
      .split(":");
    expect(useConfigurePeriodicBackup()).toBeCalledWith({
      cronspec: `${+utcMinute} ${+utcHour} * * ${selectedDow}`,
      expirationDeltaSecs: 14 * 24 * 60 * 60, // 14 days
    });
  });
});
