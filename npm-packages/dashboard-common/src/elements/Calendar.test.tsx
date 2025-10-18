import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Calendar, CalendarProps } from "./Calendar";

describe("Calendar", () => {
  describe("beforeStartTooltip", () => {
    const MESSAGE = "Cannot select dates before start";

    const currentMonth = new Date(2025, 5); // June 2025
    const minDate = new Date(2025, 5, 10);
    const maxDate = new Date(2025, 5, 20);

    const calendarProps = {
      mode: "single",
      selected: new Date(2025, 5, 15),
      defaultMonth: currentMonth,
      startMonth: minDate,
      endMonth: maxDate,
      disabled: { before: minDate, after: maxDate },
      beforeStartTooltip: MESSAGE,
      onSelect: () => {},
    } satisfies Partial<CalendarProps>;

    describe("on day buttons", () => {
      it("should show tooltip on dates before the start of allowed dates", async () => {
        const user = userEvent.setup();

        render(<Calendar {...calendarProps} />);

        const day9Button = screen.getByRole("button", {
          name: "Monday, June 9th, 2025",
        });

        expect(day9Button).toBeDisabled();

        await user.hover(day9Button);

        expect(
          screen.getByRole("tooltip", {
            name: MESSAGE,
          }),
        ).toBeInTheDocument();
      });

      it("should show tooltip on dates in the range of allowed dates", async () => {
        const user = userEvent.setup();

        render(<Calendar {...calendarProps} />);

        const day10Button = screen.getByRole("button", {
          name: "Tuesday, June 10th, 2025",
        });

        expect(day10Button).toBeEnabled();

        await user.hover(day10Button);

        expect(screen.queryByText(MESSAGE)).not.toBeInTheDocument();
      });

      it("should not show tooltip on dates after the end of allowed dates", async () => {
        const user = userEvent.setup();

        render(<Calendar {...calendarProps} />);

        const day21Button = screen.getByRole("button", {
          name: "Saturday, June 21st, 2025",
        });

        expect(day21Button).toBeDisabled();

        await user.hover(day21Button);
        expect(screen.queryByText(MESSAGE)).not.toBeInTheDocument();
      });
    });

    describe("on previous month button", () => {
      it("should show a tooltip on the previous month button", async () => {
        const user = userEvent.setup();

        render(<Calendar {...calendarProps} />);

        const prevButton = screen.getByRole("button", {
          name: "Go to the Previous Month",
        });
        expect(prevButton).toHaveAttribute("aria-disabled", "true");

        // Hover over the disabled button - should show tooltip
        await user.hover(prevButton);

        expect(
          screen.getByRole("tooltip", {
            name: MESSAGE,
          }),
        ).toBeInTheDocument();
      });

      it("should not show the tooltip on the next month button", async () => {
        const user = userEvent.setup();

        render(<Calendar {...calendarProps} />);

        const nextButton = screen.getByRole("button", {
          name: "Go to the Next Month",
        });
        expect(nextButton).toHaveAttribute("aria-disabled", "true");

        await user.hover(nextButton);
        expect(
          screen.queryByRole("tooltip", {
            name: MESSAGE,
          }),
        ).not.toBeInTheDocument();
      });
    });
  });
});
