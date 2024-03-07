import { cronJobs } from "convex/server";
import { api } from "./_generated/api";
import { mutation } from "./_generated/server";

const crons = cronJobs();

crons.weekly(
  "weekly re-engagement email",
  {
    dayOfWeek: "tuesday",
    hourUTC: 17,
    minuteUTC: 30,
  },
  api.crons.addOne,
  { x: 1 },
);

const hours = 24 * 7;
crons.interval("add one every hour", { hours }, api.crons.addOne, { x: 1 });
crons.interval("clear presence data", { seconds: 5 * 60 }, api.crons.addOne, {
  x: 1,
});

export const addOne = mutation(async (_: any, { x }: { x: number }) => {
  return x + 1;
});

export default crons;
