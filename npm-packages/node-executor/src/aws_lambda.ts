import { Context } from "aws-lambda";
import { invoke } from "./executor";
import { setDebugLogging } from "./log";
import { populatePrebuildPackages } from "./source_package";
import { Writable } from "node:stream";

const warmupPromise = populatePrebuildPackages();

declare const awslambda: any;

// eslint-disable-next-line  @typescript-eslint/no-unused-vars
export const handler = awslambda.streamifyResponse(
  async (event: any, responseStream: Writable, context: Context) => {
    setDebugLogging(true);
    await warmupPromise;
    event.requestId = context.awsRequestId;
    await invoke(event, responseStream);
    responseStream.end();
  },
);
