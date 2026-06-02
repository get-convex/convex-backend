import { cronJobs } from "convex/server";
import { api } from "./_generated/api";

const crons = cronJobs();

crons.interval(
  "audit-logged cron",
  { minutes: 10 },
  api.mutations.loggedMutation,
);

export default crons;
