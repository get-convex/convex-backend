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
    // Using `streamifyResponse()` changes the behavior of the Lambda VM
    // as compared to promise/async API (and actually makes it more like the
    // original callback Lambda API): instead of always freezing after the
    // promise of this function resolves it remaining active if there are any
    // setTimeout timers, etc. pending.
    //
    // Setting `callbackWaitsForEmptyEventLoop` to false restores that behavior
    // of freezing the Firecracker VM until the next function invocation at
    // which point such scheduled functions may run.
    //
    // The main benefit of this is not being billed for 10 minutes of
    // Lambda execution when the streamed response was returned long
    // before that, often in a couple seconds.
    //
    // The downside is loss of isolation between invocations:
    // timers may fire the next time the runtime is used.
    context.callbackWaitsForEmptyEventLoop = false;

    setDebugLogging(true);
    await warmupPromise;
    event.requestId = context.awsRequestId;
    await invoke(event, responseStream);
    responseStream.end();
  },
);
