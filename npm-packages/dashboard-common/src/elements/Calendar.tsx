"use client";

import * as React from "react";
import { ChevronLeftIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { DayPicker } from "react-day-picker";

import { cn } from "@ui/cn";

export type CalendarProps = React.ComponentProps<typeof DayPicker>;

function Calendar({ ...props }: CalendarProps) {
  return (
    <DayPicker
      classNames={{
        months: "flex flex-col sm:flex-row space-y-4 sm:space-x-4 sm:space-y-0",
        month: "space-y-4",
        caption: "flex justify-center pt-1 relative items-center",
        caption_label: "text-sm font-medium",
        nav: "space-x-1 flex items-center",
        nav_button: cn(
          "h-7 w-7 bg-transparent p-0 opacity-50 hover:opacity-100",
        ),
        nav_button_previous: "absolute left-1",
        nav_button_next: "absolute right-1",
        table: "w-full border-collapse space-y-1",
        head_row: "flex",
        head_cell: "rounded-md w-8 font-normal text-[0.8rem]",
        row: "flex w-fit mt-2 first:ml-auto",
        cell: cn(
          "p-0 text-center text-sm focus-within:relative focus-within:z-20",
        ),
        day: cn(
          "h-8 w-8 p-0 font-normal aria-selected:opacity-100",
          "hover:rounded hover:bg-background-primary",
        ),
        day_selected: "bg-background-tertiary border-y",
        day_range_start: "rounded-l bg-background-tertiary border",
        day_range_end: "rounded-r bg-background-tertiary border",
        day_range_middle:
          "bg-background-tertiary/30 border-y hover:text-primary-foreground focus:bg-primary",
        day_today: "font-semibold",
        day_outside: "day-outside opacity-30 aria-selected:opacity-50",
        day_disabled: "opacity-60 cursor-not-allowed hover:bg-transparent",
        day_hidden: "invisible",
      }}
      components={{
        IconLeft: () => <ChevronLeftIcon className="h-4 w-4" />,
        IconRight: () => <ChevronRightIcon className="h-4 w-4" />,
      }}
      {...props}
    />
  );
}
Calendar.displayName = "Calendar";

export { Calendar };
