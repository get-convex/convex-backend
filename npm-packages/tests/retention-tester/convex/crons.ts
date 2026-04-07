import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();

// 1000 * 6 * 60 = 360k / hour
crons.interval(
  "Generate fake users",
  { seconds: 10 },
  internal.generateDeleteData.generateUsers,
);

// 1000 * 6 * 60 = 360k / hour
crons.interval(
  "Generate fake messages",
  { seconds: 10 },
  internal.generateDeleteData.generateMessages,
);

crons.interval(
  "Clean users",
  { minutes: 20 },
  internal.generateDeleteData.cleanTable,
  { cursor: null, timestamp: null, table: "users" },
);

crons.interval(
  "Clean messages",
  { minutes: 20 },
  internal.generateDeleteData.cleanTable,
  { cursor: null, timestamp: null, table: "messages" },
);

export default crons;
