import { convexToJson, jsonToConvex } from "convex/values";
import { client } from "./websocket";

export {
  getOutgoingMessages,
  receiveIncomingMessages,
  getMaxObservedTimestamp,
} from "./websocket";

const subscriptions = new Map<string, () => void>();

export function addQuery(args: { udfPath: string; udfArgsJson: string }) {
  const udfArgs = jsonToConvex(JSON.parse(args.udfArgsJson)) as any;
  const { queryToken, unsubscribe } = client.subscribe(args.udfPath, udfArgs);
  subscriptions.set(queryToken, unsubscribe);
  return queryToken;
}

export function queryResult(token: string) {
  const result = (client as any).localQueryResultByToken(token);
  if (result === undefined) {
    return null;
  } else {
    return JSON.stringify(convexToJson(result));
  }
}

export function removeQuery(token: string) {
  const unsubscribe = subscriptions.get(token);
  if (!unsubscribe) {
    throw new Error(`Unknown query token: ${token}`);
  }
  unsubscribe();
}
