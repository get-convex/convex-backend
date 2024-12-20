import { jsonToConvex, convexToJson } from "convex/values";
import { client } from "./websocket";
import { outgoingMessages } from "./protocol";

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

export function runMutation(args: {
  mutationId: number;
  udfPath: string;
  udfArgsJson: string;
}) {
  const udfArgs = jsonToConvex(JSON.parse(args.udfArgsJson)) as any;
  client
    .mutation(args.udfPath, udfArgs)
    .then((result) => {
      outgoingMessages.push({
        type: "mutationDone",
        mutationId: args.mutationId,
        result: { type: "success", value: convexToJson(result) as any },
      });
    })
    .catch((error) => {
      outgoingMessages.push({
        type: "mutationDone",
        mutationId: args.mutationId,
        result: { type: "failure", error: error.toString() },
      });
    });
}
