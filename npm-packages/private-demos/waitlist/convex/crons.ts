import { cronJobs } from "convex/server";
import { internal } from "./_generated/api";

const crons = cronJobs();
crons.interval(
  "add to conversation",
  { seconds: 10 },
  internal.messages.updateConversation,
);

export default crons;
