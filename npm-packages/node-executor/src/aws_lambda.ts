import { Context } from "aws-lambda";
import { invoke } from "./executor";
import { setDebugLogging } from "./log";
import { populatePrebuildPackages } from "./source_package";

const warmupPromise = populatePrebuildPackages();

// eslint-disable-next-line  @typescript-eslint/no-unused-vars
export const handler = async (event: any, context: Context) => {
  setDebugLogging(true);
  await warmupPromise;
  event.requestId = context.awsRequestId;
  return invoke(event);
};
