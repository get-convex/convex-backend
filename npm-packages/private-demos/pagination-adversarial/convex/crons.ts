import { cronJobs } from "convex/server";
// import { api } from "./_generated/api";

const crons = cronJobs();

// Uncomment this to start inserting numbers.
// crons.interval("count", { seconds: 1 }, api.numbers.count);

export default crons;
