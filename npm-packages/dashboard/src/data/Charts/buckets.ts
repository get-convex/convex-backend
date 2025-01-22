import { format } from "date-fns";

export function calcBuckets(
  start: Date,
  end: Date,
): {
  startTime: Date;
  endTime: Date;
  numBuckets: number;
  timeMultiplier: number;
  formatTime(time: Date): string;
} {
  let startMins = start.getTime() / 1000 / 60;
  const endMins = end.getTime() / 1000 / 60;

  const timeDiffMins = endMins - startMins;
  const threeDays = 60 * 72;
  const threeHours = 60 * 3;
  const secondsInMinute = 60;
  const secondsInHour = 60 * 60;
  const secondsInDay = 60 * 60 * 24;
  let numBuckets = 1;
  let timeMultiplier = 60;
  if (timeDiffMins <= threeHours) {
    // Choose minutes for buckets
    numBuckets = Math.max(Math.round(timeDiffMins), 1);
    // Clamp time to exactly numBuckets minutes ago
    startMins = endMins - numBuckets;
    timeMultiplier = secondsInMinute; // a minute
  } else if (timeDiffMins <= threeDays) {
    // choose hours for buckets
    numBuckets = Math.round(timeDiffMins / 60);
    // Clamp start time to exactly numBuckets hours ago
    startMins = endMins - numBuckets * 60;
    timeMultiplier = secondsInHour; // an hour
  } else {
    // more than three days
    // Choose days for buckets
    numBuckets = Math.round(timeDiffMins / 60 / 24);
    // Clamp start time to exactly numBuckets days ago
    startMins = endMins - numBuckets * 60 * 24;
    timeMultiplier = secondsInDay;
  }
  function formatTime(time: Date) {
    if (timeMultiplier === secondsInMinute) {
      return format(time, "h:mm a");
    }
    if (timeMultiplier === secondsInHour) {
      return format(time, "hh a");
    }
    return format(time, "yyyy-MM-dd");
  }

  const startTime = new Date(startMins * 1000 * 60);
  const endTime = new Date(endMins * 1000 * 60);

  return {
    startTime,
    endTime,
    numBuckets,
    timeMultiplier,
    formatTime,
  };
}
