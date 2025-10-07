import React from "react";
import { render } from "@testing-library/react";
import { LogListItem } from "@common/features/logs/components/LogListItem";
import { UdfLog } from "@common/lib/useLogs";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";

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
    render(<LogListItem log={log} setShownLog={jest.fn()} focused={false} />);
    const end = performance.now();
    const renderTime = end - start;

    // eslint-disable-next-line no-console
    console.log(`LogListItem render time: ${renderTime}ms`);
  });
});
