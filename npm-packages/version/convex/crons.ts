import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();

crons.interval(
  "update NPM rules",
  {
    hours: 1,
  },
  internal.npm.refresh,
);
crons.interval(
  "update Cursor rules",
  {
    hours: 1,
  },
  internal.cursorRules.refresh,
);

export default crons;
