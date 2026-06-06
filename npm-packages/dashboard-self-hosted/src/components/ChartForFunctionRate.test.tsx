import { fireEvent, render, screen } from "@testing-library/react";
import { ConvexProvider } from "convex/react";
import { useState, type ReactNode } from "react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { ChartData } from "@common/lib/charts/types";
import { ChartForFunctionRate } from "@common/features/health/components/ChartForFunctionRate";
import { SingleGraph } from "@common/features/functions/components/SingleGraph";

let mockLineChartMounts = 0;
let mockResponsiveContainerProps: Array<{
  initialDimension?: { width: number; height: number };
}> = [];

jest.mock("recharts", () => ({
  ResponsiveContainer: (props: {
    children: ReactNode;
    initialDimension?: { width: number; height: number };
  }) => {
    mockResponsiveContainerProps.push(props);
    return <div data-testid="responsive-container">{props.children}</div>;
  },
  LineChart: ({ children }: { children: ReactNode }) => {
    const React = jest.requireActual("react");
    React.useEffect(() => {
      mockLineChartMounts += 1;
    }, []);
    return <div>{children}</div>;
  },
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

beforeEach(() => {
  mockLineChartMounts = 0;
  mockResponsiveContainerProps = [];
});

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
    expect(mockResponsiveContainerProps[0].initialDimension).toEqual({
      width: 384,
      height: 208,
    });
  });
});

describe("SingleGraph", () => {
  test("renders charts inside a positive-size frame", () => {
    const { container } = render(
      <SingleGraph title="Function Calls" data={chartData} />,
    );

    expect(screen.getByTestId("responsive-container")).toBeInTheDocument();
    expect(container.querySelector(".h-52.min-w-0.w-full")).toBeInTheDocument();
    expect(mockResponsiveContainerProps[0].initialDimension).toEqual({
      width: 384,
      height: 208,
    });
  });

  test("keeps the chart mounted across parent rerenders", () => {
    function Wrapper() {
      const [, setCount] = useState(0);
      return (
        <>
          <button type="button" onClick={() => setCount((count) => count + 1)}>
            rerender
          </button>
          <SingleGraph title="Function Calls" data={chartData} />
        </>
      );
    }

    render(<Wrapper />);
    fireEvent.click(screen.getByRole("button", { name: "rerender" }));

    expect(mockLineChartMounts).toBe(1);
  });
});
