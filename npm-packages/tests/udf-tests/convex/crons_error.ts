import { anyApi } from "convex/server";
import { cronJobs } from "convex/server";
const crons = cronJobs();
crons.interval(
  "run imaginary function",
  { seconds: 5 },
  anyApi.doesNotExist.addOne,
  { value: 1 },
);
export default crons;
