import { CronSchedule } from "system-udfs/convex/_system/frontend/common";

export function scheduleAsCron(
  s: CronSchedule & {
    type: "hourly" | "daily" | "weekly" | "monthly" | "cron";
  },
): string {
  // TypeScript should catch, check just in case it doesn't.
  if ((s.type as any) === "interval") {
    throw new Error("Can't format an interval schedule as a cron");
  }
  switch (s.type) {
    case "hourly": {
      return `${s.minuteUTC} * * * *`;
    }
    case "daily": {
      return `${s.minuteUTC} ${s.hourUTC} * * *`;
    }
    case "weekly": {
      return `${s.minuteUTC} ${s.hourUTC} * * ${s.dayOfWeek}`;
    }
    case "monthly": {
      return `${s.minuteUTC} ${s.hourUTC} ${s.day} * *`;
    }
    case "cron": {
      return s.cronExpr;
    }
    default: {
      s satisfies never;
      throw new Error(`Value type not supported: ${s}`);
    }
  }
}

/**
 * Add a reminder that this is UTC
 */
export function prettierSaffron(s: string) {
  return s.replaceAll("AM", "AM UTC").replaceAll("PM", "PM UTC");
}

export function scheduleLiteral(s: CronSchedule): string {
  return s.type === "interval"
    ? `interval({ seconds: ${s.seconds} })`
    : s.type === "hourly"
      ? `hourly({ minutesUTC: ${s.minuteUTC} })`
      : s.type === "daily"
        ? `daily({
  hourUTC: ${s.hourUTC},
  minuteUTC: ${s.minuteUTC}
})`
        : s.type === "weekly"
          ? `weekly({
  dayOfWeek: ${s.dayOfWeek},
  hourUTC: ${s.hourUTC},
  minuteUTC: ${s.minuteUTC}
})`
          : s.type === "monthly"
            ? `monthly({
  day: ${s.day},
  hourUTC: ${s.hourUTC},
  minuteUTC: ${s.minuteUTC}
})`
            : s.type === "cron"
              ? `${s.cronExpr}`
              : `Unknown Cron Schedule`;
}
