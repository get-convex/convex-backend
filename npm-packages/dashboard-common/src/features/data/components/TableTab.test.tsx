import { render, screen, fireEvent } from "@testing-library/react";
import mockRouter from "next-router-mock";
import { TableTab } from "features/data/components/TableTab";

jest.mock("next/router", () => jest.requireActual("next-router-mock"));

describe("TableTab", () => {
  test("renders table name", () => {
    const table = "users";
    render(<TableTab selectedTable={null} table={table} />);
    const tableElement = screen.getByText(table);
    expect(tableElement).toBeInTheDocument();
  });

  test("calls onSelectTable when table is clicked", () => {
    const onSelectTable = jest.fn();
    const table = "users";
    render(
      <TableTab
        selectedTable={null}
        table={table}
        onSelectTable={onSelectTable}
      />,
    );
    const tableElement = screen.getByText(table);
    fireEvent.click(tableElement);
    expect(onSelectTable).toHaveBeenCalled();
  });

  test("renders missing schema indicator when isMissingFromSchema is true", () => {
    const table = "users";
    render(<TableTab selectedTable={null} table={table} isMissingFromSchema />);
    const missingSchemaIndicator = screen.getByText("*");
    expect(missingSchemaIndicator).toBeInTheDocument();
  });

  test("href does not include existing filters from query", () => {
    const table = "users";
    const team = "my-team";
    const project = "my-project";
    const deploymentName = "my-deployment";
    mockRouter.setCurrentUrl(
      // team, project, and deploymentName are usually encoded in th page path,
      // but since this test is not in the context of a page, we need to set them in the query
      // for the test to work with the mock router.
      `/?team=${team}&project=${project}&deploymentName=${deploymentName}&filters=abc`,
    );
    render(<TableTab selectedTable={null} table={table} />);
    const tableElement = screen.getByRole("link");

    expect(tableElement).toHaveAttribute(
      "href",
      `/?team=${team}&project=${project}&deploymentName=${deploymentName}&table=${table}`,
    );
  });

  test("href includes component if there is a component", () => {
    const table = "users";
    const team = "my-team";
    const project = "my-project";
    const deploymentName = "my-deployment";
    const component = "my-component";
    mockRouter.setCurrentUrl(
      `/?team=${team}&project=${project}&deploymentName=${deploymentName}&component=${component}`,
    );
    render(<TableTab selectedTable={null} table={table} />);
    const tableElement = screen.getByRole("link");

    expect(tableElement).toHaveAttribute(
      "href",
      `/?team=${team}&project=${project}&deploymentName=${deploymentName}&component=${component}&table=${table}`,
    );
  });
});
