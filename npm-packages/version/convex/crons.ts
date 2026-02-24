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
crons.interval(
  "update guidelines",
  {
    hours: 1,
  },
  internal.guidelines.refresh,
);
crons.interval(
  "update local backend version",
  {
    hours: 1,
  },
  internal.localBackend.refresh,
);
crons.interval(
  "update agent skills SHA",
  {
    hours: 1,
  },
  internal.agentSkills.refresh,
);

export default crons;
