import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DateTimePicker } from "@common/features/data/components/FilterEditor/DateTimePicker";

describe("DateTimePicker", () => {
  const mockOnChange = jest.fn();
  const mockOnSave = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("should render with the initial date value", () => {
    const date = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={date} onChange={mockOnChange} />,
    );

    const input = container.querySelector('input[type="datetime-local"]');
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute("type", "datetime-local");
  });

  it("should call onChange when a valid datetime is entered", async () => {
    const initialDate = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={initialDate} onChange={mockOnChange} />,
    );

    const input = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    const user = userEvent.setup();

    // Change to a new datetime
    await user.clear(input);
    await user.type(input, "2024-11-15T16:45:00");

    expect(mockOnChange).toHaveBeenCalledWith(new Date("2024-11-15T16:45:00"));
  });

  it("should not call onChange when input value is empty", async () => {
    const initialDate = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={initialDate} onChange={mockOnChange} />,
    );

    const input = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    const user = userEvent.setup();

    await user.clear(input);

    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it("should call onSave when Enter key is pressed", async () => {
    const initialDate = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker
        date={initialDate}
        onChange={mockOnChange}
        onSave={mockOnSave}
      />,
    );

    const input = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    const user = userEvent.setup();

    await user.click(input);
    await user.keyboard("{Enter}");

    expect(mockOnSave).toHaveBeenCalledTimes(1);
  });

  it("should not call onSave when Enter is pressed if onSave is not provided", async () => {
    const initialDate = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={initialDate} onChange={mockOnChange} />,
    );

    const input = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    const user = userEvent.setup();

    // Should not throw an error
    await user.click(input);
    await user.keyboard("{Enter}");

    expect(mockOnSave).not.toHaveBeenCalled();
  });

  it("should render as disabled when disabled prop is true", () => {
    const date = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={date} onChange={mockOnChange} disabled />,
    );

    const input = container.querySelector('input[type="datetime-local"]');
    expect(input).toBeDisabled();
  });

  it("should autofocus when autoFocus prop is true", () => {
    const date = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={date} onChange={mockOnChange} autoFocus />,
    );

    const input = container.querySelector('input[type="datetime-local"]');
    expect(input).toHaveFocus();
  });

  it("should have step=1 to allow seconds precision", () => {
    const date = new Date("2024-10-07T14:35:32");
    const { container } = render(
      <DateTimePicker date={date} onChange={mockOnChange} />,
    );

    const input = container.querySelector('input[type="datetime-local"]');
    expect(input).toHaveAttribute("step", "1");
  });
});
