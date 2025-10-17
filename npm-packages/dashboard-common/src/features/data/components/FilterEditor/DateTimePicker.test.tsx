import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";

describe("DateTimePicker", () => {
  const mockOnChange = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("should update when a valid datetime is entered", async () => {
    const dateTimeString = "10/7/2024, 2:35:32 PM";

    // Render the component.
    const initialDate = new Date();
    render(<DateTimePicker date={initialDate} onChange={mockOnChange} />);

    // Clear the input and enter a new datetime.
    const dateTimeInput = screen.getByLabelText("Date and time");
    const user = userEvent.setup();
    await user.clear(dateTimeInput);
    await user.type(dateTimeInput, dateTimeString);
    await user.tab(); // Trigger blur event

    // Check that the datetime was updated correctly.
    expect(dateTimeInput).toHaveValue(dateTimeString);
    expect(mockOnChange).toHaveBeenCalledWith(new Date(dateTimeString));
  });

  it("should reject an invalid datetime", async () => {
    // Render the component.
    const initialDate = new Date();
    render(<DateTimePicker date={initialDate} onChange={mockOnChange} />);

    // Store the initial date.
    const dateTimeInput = screen.getByLabelText("Date and time");
    const initialDateString = (dateTimeInput as HTMLInputElement).value;

    // Clear the input and enter an invalid datetime.
    const user = userEvent.setup();
    await user.clear(dateTimeInput);
    await user.type(dateTimeInput, "invalid datetime");
    await user.tab(); // Trigger blur event

    // Check that the datetime was not changed..
    expect(dateTimeInput).toHaveValue(initialDateString);
    expect(mockOnChange).toBeCalledTimes(0);
  });

  it("should open popup when focused and close when clicking outside", async () => {
    // Render the component in popup mode (default).
    const initialDate = new Date();
    render(<DateTimePicker date={initialDate} onChange={mockOnChange} />);

    const dateTimeInput = screen.getByLabelText("Date and time");
    const user = userEvent.setup();

    // Initially, the popup should be hidden.
    const popup = screen.queryByRole("dialog");
    expect(popup).toHaveClass("hidden");

    // Focus the input to open the popup.
    await user.click(dateTimeInput);

    // The popup should now be visible (no hidden class).
    expect(screen.getByRole("dialog")).not.toHaveClass("hidden");

    // Click outside the component to close the popup.
    await user.click(document.body);

    // The popup should be hidden again.
    expect(screen.queryByRole("dialog")).toHaveClass("hidden");
  });

  it("should allow selecting a date by clicking on the calendar", async () => {
    // Use a specific initial date to make the test predictable
    const initialDate = new Date("2024-01-15T10:30:00");
    render(<DateTimePicker date={initialDate} onChange={mockOnChange} />);

    const dateTimeInput = screen.getByLabelText("Date and time");
    const user = userEvent.setup();

    // Focus the input to open the popup.
    await user.click(dateTimeInput);

    // The popup should now be visible.
    expect(screen.getByRole("dialog")).not.toHaveClass("hidden");

    // Find a different date button in the calendar (e.g., day 20)
    const dayButton = screen.getByRole("button", {
      name: "Saturday, January 20th, 2024",
    });

    // Click on the date button
    await user.click(dayButton);

    // Verify that onChange was called with a date that has day 20
    // The time portion should remain the same (10:30:00)
    expect(mockOnChange).toHaveBeenCalledWith(new Date("2024-01-20T10:30:00"));

    // The popup should still be open (only closes on outside click or escape)
    expect(screen.getByRole("dialog")).not.toHaveClass("hidden");
  });
});
