import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SpendingLimitsForm } from "./SpendingLimits";

jest.mock("api/billing", () => ({
  useSetSpendingLimit: jest.fn(),
}));

describe("SpendingLimitsForm", () => {
  const mockOnSubmit = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("should allow setting a zero spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: 42,
          spendingLimitDisableThresholdUsd: null,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    const spendLimitCheckbox = screen.getByLabelText("Limit usage spending to");
    await userEvent.click(spendLimitCheckbox);
    expect(spendLimitCheckbox).toBeChecked();

    const spendLimitInput = screen.getByLabelText("Disable Threshold");
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "0");

    // The form should be valid and the submit button should be enabled
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    expect(submitButton).not.toBeDisabled();

    // The warning threshold field should still not be visible
    expect(
      screen.queryByLabelText("Warn when spending exceeds"),
    ).not.toBeInTheDocument();

    // Click the submit button
    await userEvent.click(submitButton);

    // Verify that onSubmit was called with the correct values
    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitWarningThresholdUsd: null,
        spendingLimitDisableThresholdUsd: 0,
      });
    });
  });

  it("should not allow submission when spend limit is not a number", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: null,
          spendingLimitDisableThresholdUsd: undefined,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Disable Threshold");

    // Enter a non-numeric value
    const nonNumericValue = "not a number";
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, nonNumericValue);

    // Blur the input
    await userEvent.click(document.body);

    // The form should not be valid and the submit button should be disabled
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    expect(submitButton).toBeDisabled();
  });

  it("should not allow submission when warning threshold is higher than spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: null,
          spendingLimitDisableThresholdUsd: null,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Enable both checkboxes
    const spendLimitCheckbox = screen.getByLabelText("Limit usage spending to");
    await userEvent.click(spendLimitCheckbox);
    expect(spendLimitCheckbox).toBeChecked();

    const warningThresholdCheckbox = screen.getByLabelText(
      "Warn when spending exceeds",
    );
    await userEvent.click(warningThresholdCheckbox);
    expect(warningThresholdCheckbox).toBeChecked();

    // Enter two values that don’t match
    const spendLimitInput = screen.getByLabelText("Disable Threshold");
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "100");

    const warningThresholdInput = screen.getByLabelText("Warning Threshold");
    await userEvent.clear(warningThresholdInput);
    await userEvent.type(warningThresholdInput, "101");

    // Blur the inputs
    await userEvent.click(document.body);

    // Error message should be visible
    expect(
      screen.getByText(
        "The warning threshold must be less than the spend limit.",
      ),
    ).toBeInTheDocument();
  });

  it("should not allow submission when spend limit is less than current spending", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: null,
          spendingLimitDisableThresholdUsd: undefined,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={1000}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Disable Threshold");

    // Enter a value less than the current spending
    await userEvent.type(spendLimitInput, "99");
    await userEvent.click(document.body);

    expect(
      screen.getByText(
        "The spend limit must be greater than the spending in the current billing cycle ($1,000).",
      ),
    ).toBeInTheDocument();
  });

  it("allows setting a spend limit that is equal to the current spending", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: null,
          spendingLimitDisableThresholdUsd: undefined,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    const spendLimitInput = screen.getByLabelText("Disable Threshold");
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "0");
    await userEvent.click(document.body);

    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    await userEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitWarningThresholdUsd: null,
        spendingLimitDisableThresholdUsd: 0,
      });
    });
  });

  it("does not allow setting a negative spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: undefined,
          spendingLimitDisableThresholdUsd: null,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={undefined}
      />,
    );

    const spendLimitInput = screen.getByLabelText("Warning Threshold");
    await userEvent.type(spendLimitInput, "-1");
    await userEvent.click(document.body);

    expect(
      screen.getByText("Please enter a positive number."),
    ).toBeInTheDocument();
  });

  it("should erase the existing values when disabling spending limits", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitWarningThresholdUsd: 1234,
          spendingLimitDisableThresholdUsd: 5678,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Disable both spending limits
    const spendLimitCheckbox = screen.getByLabelText("Limit usage spending to");
    await userEvent.click(spendLimitCheckbox);
    expect(spendLimitCheckbox).not.toBeChecked();
    expect(screen.getByLabelText("Disable Threshold")).toBeDisabled();

    const warningThresholdCheckbox = screen.getByLabelText(
      "Warn when spending exceeds",
    );
    await userEvent.click(warningThresholdCheckbox);
    expect(warningThresholdCheckbox).not.toBeChecked();
    expect(screen.getByLabelText("Warning Threshold")).toBeDisabled();

    // Submit the form
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    await userEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitWarningThresholdUsd: null,
        spendingLimitDisableThresholdUsd: null,
      });
    });
  });
});
