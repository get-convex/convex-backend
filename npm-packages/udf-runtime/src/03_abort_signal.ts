import {
  AbortController,
  AbortSignal,
} from "abortcontroller-polyfill/dist/abortcontroller";

export const setupAbortSignal = (global) => {
  global.AbortController = AbortController;
  global.AbortSignal = AbortSignal;
};
