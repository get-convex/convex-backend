import { format, parse } from "date-fns";
import { cn } from "@ui/cn";
import { useEffect, useRef, useState } from "react";
import { usePopper } from "react-popper";
import { Calendar } from "@common/elements/Calendar";
import { TextInput } from "@ui/TextInput";
import { Matcher } from "react-day-picker";

const dateTimeFormat = "M/d/yyyy, h:mm:ss aa";

type DateTimePickerProps = {
  date: Date;
  onChange: (date: Date) => void;
  minDate?: Date;
  maxDate?: Date;
  disabled?: boolean;
  className?: string;
  mode?: "popup" | "text-only";
  onError?: (error: string | undefined) => void;
  onKeyDown?: (
    event: React.KeyboardEvent<HTMLInputElement>,
    date: Date | undefined,
  ) => void;
};

export function DateTimePicker({
  date,
  onChange,
  minDate,
  maxDate,
  disabled = false,
  className,
  mode = "popup",
  onError,
  onKeyDown,
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

  // Validate date format and call onError callback
  const validateAndSetError = (value: string) => {
    const parsedDate = parse(value, dateTimeFormat, new Date());
    const isValid = !Number.isNaN(parsedDate.getTime());

    const newError = isValid
      ? undefined
      : `Invalid date format. Example: ${format(new Date(), dateTimeFormat)}`;
    onError?.(newError);

    return isValid;
  };

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

  const handleTextInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setInputValue(newValue);

    // Validate the input as user types
    if (newValue.trim()) {
      validateAndSetError(newValue);
    } else {
      onError?.(undefined);
    }
  };

  const handleTextInputBlur = () => {
    const parsedDate = parse(inputValue, dateTimeFormat, new Date());

    // Don’t fire event if there was no change.
    const timeAtSecondPrecision = (d: Date) => Math.floor(d.getTime() / 1000);
    if (timeAtSecondPrecision(parsedDate) === timeAtSecondPrecision(dateTime)) {
      return;
    }

    if (!Number.isNaN(parsedDate.getTime())) {
      setDateTime(parsedDate);
      onChange(parsedDate);
      onError?.(undefined);
    } else {
      setInputValue(format(dateTime, dateTimeFormat));
      onError?.(undefined);
    }
  };

  // Update the input and the visible month.
  useEffect(() => {
    setInputValue(format(dateTime, dateTimeFormat));
    setVisibleMonth(dateTime);
  }, [dateTime]);

  // Close the popover when clicking/touching outside.
  useEffect(() => {
    function handleOutsideInteraction(event: MouseEvent | TouchEvent) {
      if (open) {
        const target = event.target as Node;
        // Don't close if clicking inside the wrapper or the popover
        if (
          (wrapperRef.current && wrapperRef.current.contains(target)) ||
          (popoverRef.current && popoverRef.current.contains(target))
        ) {
          return;
        }
        setOpen(false);
      }
    }

    function handleKeyboardFocus(event: FocusEvent) {
      if (open) {
        const target = event.target as Node;
        // Don't close if focusing inside the wrapper or the popover
        if (
          (wrapperRef.current && wrapperRef.current.contains(target)) ||
          (popoverRef.current && popoverRef.current.contains(target))
        ) {
          return;
        }
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handleOutsideInteraction);
    document.addEventListener("touchstart", handleOutsideInteraction);
    document.addEventListener("focusin", handleKeyboardFocus);

    return () => {
      document.removeEventListener("mousedown", handleOutsideInteraction);
      document.removeEventListener("touchstart", handleOutsideInteraction);
      document.removeEventListener("focusin", handleKeyboardFocus);
    };
  }, [open]);

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
    if (!disabled && mode === "popup") {
      setOpen(true);
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (!onKeyDown) return;

    const parsedDate = parse(inputValue, dateTimeFormat, new Date());
    if (!Number.isNaN(parsedDate.getTime())) {
      onKeyDown(event, parsedDate);
    } else if (inputValue.trim() === "" && event.key === "Enter") {
      onKeyDown(event, undefined);
    }
  };

  return (
    <div>
      <div ref={wrapperRef}>
        <TextInput
          ref={
            mode === "popup"
              ? inputRef
              : (r) => {
                  r?.querySelector("input")?.focus();
                }
          }
          id="dateTime"
          type="text"
          value={inputValue}
          onChange={handleTextInputChange}
          onBlur={handleTextInputBlur}
          onFocus={handleFocus}
          onKeyDown={handleKeyDown}
          labelHidden
          autoFocus={mode === "text-only"}
          aria-label="Date and time"
          aria-haspopup={mode === "popup" ? "dialog" : undefined}
          aria-expanded={mode === "popup" ? open : undefined}
          className={cn("rounded-none", className, open && "z-20")}
          size="sm"
          disabled={disabled}
        />
      </div>
      {mode === "popup" && (
        <div
          ref={popoverRef}
          className={cn(
            "z-50 flex flex-col rounded-lg border bg-background-secondary p-2 shadow-md",
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
            startMonth={minDate}
            endMonth={maxDate}
            disabled={disabledFromRange({ minDate, maxDate })}
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
      )}
    </div>
  );
}

// This is necessary to make the type checker happy
export function disabledFromRange({
  minDate,
  maxDate,
}: {
  minDate: Date | undefined;
  maxDate: Date | undefined;
}): Matcher | undefined {
  if (minDate && maxDate) {
    return { before: minDate, after: maxDate };
  }
  if (minDate) {
    return { before: minDate };
  }
  if (maxDate) {
    return { after: maxDate };
  }
  return undefined;
}
