/* eslint-disable no-restricted-globals */
import { stringifyValue } from "../src/lib/stringifyValue";
import { jsonToConvex, JSONValue } from "convex/values";

self.onmessage = async (message: MessageEvent<JSONValue>) => {
  self.postMessage(stringifyValue(jsonToConvex(message.data), true));
};
