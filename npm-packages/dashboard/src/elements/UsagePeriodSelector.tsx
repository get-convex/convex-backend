import { DateRangePicker } from "dashboard-common/elements/DateRangePicker";
import { subDays, subMonths } from "date-fns";

/**
 * The date at which valid usage data starts. Users can’t see the data before.
 */
const USAGE_STATS_START = "2023-08-03";

export type Period = {
  type: "currentBillingPeriod" | "presetPeriod" | "customPeriod";
  from: string; // ISO 8601 date strings
  to: string;
};

export function isoDateString(date: Date) {
  const year = date.getFullYear();
  const month = (date.getMonth() + 1).toString().padStart(2, "0");
  const day = date.getDate().toString().padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * @param dateString e.g. "2023-11-02"
 * @returns e.g. "2023-10-02"
 */
function monthBefore(dateString: string): string {
  const [year, month, day] = dateString.split("-");
  const date = new Date(Number(year), Number(month) - 1, Number(day));
  date.setMonth(date.getMonth() - 1);
  return isoDateString(date);
}

function startOfDayUTC(date: string): Date {
  return new Date(new Date(date).toLocaleString("en-US", { timeZone: "UTC" }));
}

export function UsagePeriodSelector({
  period,
  onChange,
  currentBillingPeriod,
}: {
  period: Period;
  onChange: (newValue: Period) => void;
  currentBillingPeriod: { start: string; end: string };
}) {
  const from = startOfDayUTC(period.from);
  const to = startOfDayUTC(period.to);
  const currentBillingPeriodStart = startOfDayUTC(currentBillingPeriod.start);
  const currentBillingPeriodEnd = startOfDayUTC(currentBillingPeriod.end);
  const lastBillingPeriodStart = startOfDayUTC(
    monthBefore(currentBillingPeriod.start),
  );
  const lastBillingPeriodEnd = startOfDayUTC(currentBillingPeriod.start);

  const today = startOfDayUTC(isoDateString(new Date()));
  const weekAgo = startOfDayUTC(isoDateString(subDays(today, 7)));
  const monthAgo = startOfDayUTC(isoDateString(subDays(today, 30)));
  const quarterAgo = startOfDayUTC(isoDateString(subMonths(today, 3)));

  return (
    <DateRangePicker
      minDate={new Date(USAGE_STATS_START)}
      shortcuts={[
        {
          value: "current",
          label: "Current billing period",
          from: currentBillingPeriodStart,
          to: currentBillingPeriodEnd,
        },
        {
          value: "last",
          label: "Last billing period",
          from: lastBillingPeriodStart,
          to: lastBillingPeriodEnd,
        },
        {
          value: "week",
          label: "Last 7 days",
          from: weekAgo,
          to: today,
        },
        {
          value: "month",
          label: "Last 30 days",
          from: monthAgo,
          to: today,
        },
        {
          value: "quarter",
          label: "Last 3 months",
          from: quarterAgo,
          to: today,
        },
      ]}
      date={{
        from,
        to,
      }}
      setDate={(date, shortcut) => {
        date.from &&
          date.to &&
          onChange(
            shortcut === "current"
              ? {
                  type: "currentBillingPeriod",
                  from: currentBillingPeriod.start,
                  to: currentBillingPeriod.end,
                }
              : shortcut === "last"
                ? {
                    type: "presetPeriod",
                    from: monthBefore(currentBillingPeriod.start),
                    to: currentBillingPeriod.start,
                  }
                : {
                    type: "customPeriod",
                    from: isoDateString(date.from),
                    to: isoDateString(date.to),
                  },
          );
      }}
    />
  );
}
