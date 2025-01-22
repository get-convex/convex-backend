"use client";

import { CalendarIcon, CheckIcon } from "@radix-ui/react-icons";
import { endOfToday, parse, startOfDay, format } from "date-fns";
import { useRouter } from "next/router";
import * as React from "react";
import { DateRange } from "react-day-picker";
import { Button } from "dashboard-common";
import { Calendar } from "./Calendar";
import { Popover } from "./Popover";

export function DateRangePicker({
  minDate,
  maxDate,
  date,
  setDate,
  shortcuts,
  formatDate = (d: Date) => format(d, "LLL dd, y"),
}: {
  minDate?: Date;
  maxDate?: Date;
  date: {
    from: Date;
    to: Date;
  };
  setDate: (date: DateRange, shortcut?: string) => void;
  shortcuts?: { value: string; label: string; from: Date; to: Date }[];
  formatDate?: (date: Date) => string;
}) {
  const { from, to } = date;
  return (
    <Popover
      placement="bottom-start"
      button={
        <Button
          id="date"
          variant="neutral"
          className="w-fit justify-start text-left font-normal"
        >
          <CalendarIcon className="h-4 w-4" />
          {from ? (
            to ? (
              <>
                {formatDate(from)} – {formatDate(to)}
              </>
            ) : (
              formatDate(from)
            )
          ) : (
            <span>Pick a date</span>
          )}
        </Button>
      }
    >
      {({ close }) => (
        <div className="flex flex-col gap-4 md:flex-row">
          {shortcuts && (
            <div className="flex flex-col gap-2 border-b pb-4 pr-4 md:border-b-0 md:border-r md:pb-0">
              {shortcuts.map((s) => (
                <Button
                  key={s.label}
                  variant="unstyled"
                  onClick={() => {
                    close();
                    setDate(s, s.value);
                  }}
                  className="-ml-4 flex w-fit items-start gap-1 rounded p-1 text-xs hover:bg-background-tertiary"
                  icon={
                    s.from.toDateString() === from.toDateString() &&
                    s.to.toDateString() === to.toDateString() ? (
                      <CheckIcon className="mt-1" />
                    ) : (
                      <div className="size-4" />
                    )
                  }
                >
                  <div className="flex flex-col items-start">
                    {s.label}{" "}
                    <span className="text-xs text-content-secondary">
                      {formatDate(s.from)} – {formatDate(s.to)}
                    </span>
                  </div>
                </Button>
              ))}
            </div>
          )}
          <Calendar
            initialFocus
            mode="range"
            fromDate={minDate}
            toDate={maxDate}
            defaultMonth={from}
            selected={{ from, to }}
            onSelect={(d, selectedDay) => {
              if (!d) return;
              if (selectedDay > from && selectedDay < to) {
                setDate({ from: d.to, to: d.to });
                return;
              }
              setDate(d);
            }}
            numberOfMonths={2}
          />
        </div>
      )}
    </Popover>
  );
}

export const DATE_FORMAT = "yyyy-MM-dd";

export function useDateFilters() {
  // Current day
  const maxEndDate = endOfToday();

  // A week from current day
  const initStartDate = new Date(maxEndDate);
  initStartDate.setDate(initStartDate.getDate() - 7);

  const router = useRouter();
  const startDate = router.query.startDate
    ? parse(router.query.startDate as string, DATE_FORMAT, new Date())
    : initStartDate;
  const endDate = router.query.endDate
    ? parse(router.query.endDate as string, DATE_FORMAT, new Date())
    : maxEndDate;

  const checkAndSetStartDate = React.useCallback(
    async (date: Date) => {
      const start = startOfDay(date);
      router.query.startDate = format(start, DATE_FORMAT);
      await router.replace({
        query: router.query,
      });
    },
    [router],
  );

  const checkAndSetEndDate = React.useCallback(
    async (date: Date) => {
      const end = startOfDay(date);
      router.query.endDate = format(end, DATE_FORMAT);
      await router.replace({
        query: router.query,
      });
    },
    [router],
  );
  return {
    startDate,
    endDate,
    setDate: async (date: DateRange) => {
      date.from && (await checkAndSetStartDate(date.from));
      date.to && (await checkAndSetEndDate(date.to));
    },
  };
}
