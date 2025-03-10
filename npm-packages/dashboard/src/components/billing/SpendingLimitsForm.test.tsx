import React from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SpendingLimitsForm } from "./SpendingLimits";

jest.mock("api/billing", () => ({
  useSetSpendingLimit: jest.fn(),
}));

describe("SpendingLimitsForm", () => {
  const defaultValue = {
    spendingLimitEnabled: true,
    spendingLimitDisableThresholdUsd: null,
    spendingLimitWarningThresholdUsd: null,
  };
  const mockOnSubmit = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("should allow setting a zero spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    const spendLimitInput = screen.getByLabelText("Spend Limit");
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
        spendingLimitEnabled: true,
        spendingLimitDisableThresholdUsd: 0,
        spendingLimitWarningThresholdUsd: null,
      });
    });
  });

  it("should auto-populate a warning threshold when setting a spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");

    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "11");

    // Blur the input
    await userEvent.click(document.body);

    // The warning threshold field should appear and be auto-populated with 80% of the spend limit
    const warningThresholdInput = screen.getByLabelText(
      "Warn when spending exceeds",
    );
    const expectedThreshold = 8; // 80% of 11, floored

    await waitFor(() => {
      expect(warningThresholdInput).toHaveValue(expectedThreshold);
    });

    // The form should be valid and the submit button should be enabled
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    expect(submitButton).not.toBeDisabled();

    // Click the submit button
    await userEvent.click(submitButton);

    // Verify that onSubmit was called with the correct values
    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitEnabled: true,
        spendingLimitDisableThresholdUsd: 11,
        spendingLimitWarningThresholdUsd: expectedThreshold,
      });
    });
  });

  it("should not auto-populate a warning threshold when it already has a value", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          ...defaultValue,
          spendingLimitWarningThresholdUsd: 10,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "100");
    await userEvent.click(document.body);

    // The warning threshold field’s value should not change
    const warningThresholdInput = screen.getByLabelText(
      "Warn when spending exceeds",
    );
    expect(warningThresholdInput).toHaveValue(10);
  });

  it("should not allow submission when spend limit is not a number", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");

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
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");

    // Enter a value higher than fixed costs
    const higherValue = 50;
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, higherValue.toString());

    // Find the warning threshold input
    const warningThresholdInput = screen.getByLabelText(
      "Warn when spending exceeds",
    );

    // Enter a value higher than the spend limit
    const higherThreshold = 60;
    await userEvent.clear(warningThresholdInput);
    await userEvent.type(warningThresholdInput, higherThreshold.toString());

    // The form should not be valid and the submit button should be disabled
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    expect(submitButton).toBeDisabled();
  });

  it("should reset the warning threshold to null when spend limit is set again to zero", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");

    // Enter a value higher than zero
    const higherValue = 50;
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, higherValue.toString());

    // Enter a value in the warning threshold input
    const warningThresholdInput = screen.getByLabelText(
      "Warn when spending exceeds",
    );
    const warningThresholdValue = 40;
    await userEvent.clear(warningThresholdInput);
    await userEvent.type(
      warningThresholdInput,
      warningThresholdValue.toString(),
    );

    // Set the spend limit to zero
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "0");

    // The warning threshold field should not be visible
    expect(warningThresholdInput).not.toBeInTheDocument();

    // When submitting the form, the warning threshold should be null
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    await userEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitEnabled: true,
        spendingLimitDisableThresholdUsd: 0,
        spendingLimitWarningThresholdUsd: null,
      });
    });
  });

  it("should not allow submission when spend limit is less than current spending", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={1000}
      />,
    );

    // Find the spend limit input
    const spendLimitInput = screen.getByLabelText("Spend Limit");

    // Enter a value less than the current spending
    await userEvent.type(spendLimitInput, "99");
    await userEvent.click(document.body);

    expect(
      screen.getByText(
        "The spend limit must be greater than the spending in the current billing cycle ($1,000).",
      ),
    ).toBeInTheDocument();
  });

  it("does not allow setting a negative spend limit", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={defaultValue}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={undefined}
      />,
    );

    const spendLimitInput = screen.getByLabelText("Spend Limit");
    await userEvent.clear(spendLimitInput);
    await userEvent.type(spendLimitInput, "-1");
    await userEvent.click(document.body);

    expect(
      screen.getByText("Please enter a positive number."),
    ).toBeInTheDocument();
  });

  it("should remove the existing disable spend limit when disabling spending limits", async () => {
    render(
      <SpendingLimitsForm
        defaultValue={{
          spendingLimitEnabled: true,
          spendingLimitDisableThresholdUsd: 100,
          spendingLimitWarningThresholdUsd: 80,
        }}
        onSubmit={mockOnSubmit}
        onCancel={jest.fn()}
        currentSpendingUsd={0}
      />,
    );

    // Disable spending limits
    const checkbox = screen.getByRole("checkbox");
    await userEvent.click(checkbox);
    expect(checkbox).not.toBeChecked();

    // Submit the form
    const submitButton = screen.getByRole("button", {
      name: "Save Spending Limits",
    });
    await userEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith({
        spendingLimitEnabled: false,
        spendingLimitDisableThresholdUsd: null,
        spendingLimitWarningThresholdUsd: 80,
      });
    });
  });
});
