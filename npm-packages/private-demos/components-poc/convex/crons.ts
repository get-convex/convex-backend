import { cronJobs } from "convex/server";
// import { components } from "./_generated/api";

const crons = cronJobs();

/*
crons.interval(
  "send message",
  { seconds: 10 }, // every minute
  components.waitlist.sendMessage,
);
*/
export default crons;
