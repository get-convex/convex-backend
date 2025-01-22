import React from "react";
import { render } from "@testing-library/react";
import { functionIdentifierValue, UdfLog } from "dashboard-common";
import { LogListItem } from "./LogListItem";

describe("LogListItem render benchmark", () => {
  it("should render LogListItem within acceptable time", () => {
    const start = performance.now();
    const log: UdfLog = {
      id: "1",
      kind: "log",
      timestamp: new Date().getTime(),
      localizedTimestamp: new Date().toISOString(),
      udfType: "Mutation",
      call: functionIdentifierValue("mutateData"),
      output: { level: "DEBUG", messages: ["Log!"], isTruncated: false },
      requestId: `request`,
      executionId: `id`,
    };
    render(<LogListItem log={log} setShownLog={jest.fn()} />);
    const end = performance.now();
    const renderTime = end - start;

    // eslint-disable-next-line no-console
    console.log(`LogListItem render time: ${renderTime}ms`);
  });
});
