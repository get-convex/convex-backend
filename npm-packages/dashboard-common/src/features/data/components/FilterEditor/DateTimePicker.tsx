import { format, parse } from "date-fns";
import { cn } from "@common/lib/cn";
import { useEffect, useRef, useState } from "react";
import { usePopper } from "react-popper";
import { Calendar } from "@common/elements/Calendar";
import { useInteractOutside } from "@common/features/data/lib/useInteractOutside";
import { TextInput } from "@common/elements/TextInput";

const dateTimeFormat = "M/d/yyyy, h:mm:ss aa";

type DateTimePickerProps = {
  date: Date;
  onChange: (date: Date) => void;
  minDate?: Date;
  maxDate?: Date;
  disabled?: boolean;
  className?: string;
};

export function DateTimePicker({
  date,
  onChange,
  minDate,
  maxDate,
  disabled = false,
  className,
}: DateTimePickerProps) {
  const [open, setOpen] = useState(false);
  const [dateTime, setDateTime] = useState(date);
  const [inputValue, setInputValue] = useState(format(date, dateTimeFormat));
  const [visibleMonth, setVisibleMonth] = useState(date);
  const inputRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);

  const { styles, attributes, update } = usePopper(
    inputRef.current,
    popoverRef.current,
    {
      placement: "bottom-start",
      modifiers: [
        {
          name: "offset",
          options: { offset: [0, 8] },
        },
      ],
    },
  );

  const handleDateChange = (newDate: Date | undefined) => {
    if (newDate) {
      setDateTime((prevDateTime) => {
        const updatedDateTime = new Date(newDate);
        updatedDateTime.setHours(
          prevDateTime.getHours(),
          prevDateTime.getMinutes(),
          prevDateTime.getSeconds(),
          prevDateTime.getMilliseconds(),
        );
        // Call onChange directly with the updated date
        onChange(updatedDateTime);
        return updatedDateTime;
      });
    }
  };

  const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const [hours, minutes, seconds] = e.target.value.split(":").map(Number);

    setDateTime((prevDateTime) => {
      const newDateTime = new Date(prevDateTime);
      newDateTime.setHours(hours, minutes, seconds);

      // If invalid, return the previous valid date.
      if (Number.isNaN(newDateTime.getTime())) {
        return prevDateTime;
      }

      // Call onChange directly with the new valid time
      onChange(newDateTime);
      return newDateTime;
    });
  };

  const handleTextInputBlur = () => {
    const parsedDate = parse(inputValue, dateTimeFormat, new Date());
    if (!Number.isNaN(parsedDate.getTime())) {
      setDateTime(parsedDate);
      onChange(parsedDate);
    } else {
      setInputValue(format(dateTime, dateTimeFormat));
    }
  };

  // Update the input and the visible month.
  useEffect(() => {
    setInputValue(format(dateTime, dateTimeFormat));
    setVisibleMonth(dateTime);
  }, [dateTime]);

  // Close the popover when clicking/touching outside.
  useInteractOutside(wrapperRef, () => {
    if (open) {
      setOpen(false);
    }
  });

  // Re-calculate popper position when opening.
  useEffect(() => {
    if (open && update) {
      void update?.();
    }
  }, [open, update]);

  // Close the popover when the user presses the escape key.
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && open) {
        setOpen(false);
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  const handleFocus = () => {
    if (!disabled) {
      setOpen(true);
    }
  };

  return (
    <div ref={wrapperRef}>
      <TextInput
        ref={inputRef}
        id="dateTime"
        type="text"
        value={inputValue}
        onChange={(e) => setInputValue(e.target.value)}
        onBlur={handleTextInputBlur}
        onFocus={handleFocus}
        labelHidden
        aria-label="Date and time"
        aria-haspopup="dialog"
        aria-expanded={open}
        className={cn("rounded-none", className, open && "z-20")}
        size="sm"
        disabled={disabled}
      />
      <div
        ref={popoverRef}
        className={cn(
          "z-50 bg-background-secondary shadow-md border rounded-lg p-2 flex flex-col",
          open && !disabled ? "block" : "hidden",
        )}
        {...attributes.popper}
        style={styles.popper}
        role="dialog"
        aria-label="Date and time picker"
      >
        <Calendar
          mode="single"
          selected={dateTime}
          onSelect={handleDateChange}
          // Necessary so the calendar updates when changing the date via the text input.
          month={visibleMonth}
          onMonthChange={(newDate) => setVisibleMonth(newDate)}
          fromDate={minDate}
          toDate={maxDate}
        />
        <input
          type="time"
          step="1"
          value={format(dateTime, "HH:mm:ss")}
          onChange={handleTimeChange}
          className="mt-2 w-full cursor-text rounded-md border bg-transparent p-2 text-right text-sm"
          aria-label="Set time"
        />
      </div>
    </div>
  );
}
