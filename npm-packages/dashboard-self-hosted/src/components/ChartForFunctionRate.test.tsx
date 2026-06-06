import { render, screen } from "@testing-library/react";
import { ConvexProvider } from "convex/react";
import type { ReactNode } from "react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { ChartData } from "@common/lib/charts/types";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import { SingleGraph } from "@common/features/functions/components/SingleGraph";

jest.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  LineChart: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  XAxis: () => null,
  YAxis: () => null,
  Legend: () => null,
  Tooltip: () => null,
  Line: () => null,
  ReferenceLine: () => null,
  CartesianGrid: () => null,
}));

const mockClient = mockConvexReactClient().registerQueryFake(
  udfs.paginatedDeploymentEvents.default,
  () => ({
    page: [],
    isDone: true,
    continueCursor: "",
  }),
);

const chartData: ChartData = {
  xAxisKey: "time",
  data: [{ time: "12:00 PM", query: 1 }],
  lineKeys: [{ key: "query", name: "query", color: "var(--chart-line-1)" }],
};

describe("ChartForFunctionRate", () => {
  test("renders charts inside a positive-size frame", () => {
    const { container } = render(
      <ConvexProvider client={mockClient}>
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ChartForFunctionRate chartData={chartData} kind="functionCalls" />
        </DeploymentInfoContext.Provider>
      </ConvexProvider>,
    );

    expect(screen.getByTestId("responsive-container")).toBeInTheDocument();
    expect(container.querySelector(".h-52.min-w-0.w-full")).toBeInTheDocument();
  });
});

describe("SingleGraph", () => {
  test("renders charts inside a positive-size frame", () => {
    const { container } = render(
      <SingleGraph title="Function Calls" data={chartData} />,
    );

    expect(screen.getByTestId("responsive-container")).toBeInTheDocument();
    expect(container.querySelector(".h-52.min-w-0.w-full")).toBeInTheDocument();
  });
});
