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
});
