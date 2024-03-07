import { ConvexError } from "convex/values";
import { mutation, query } from "./_generated/server";

export const queryThrows = query(() => {
  throw new ConvexError(true);
});

export const queryThrowsAsync = query(async () => {
  throw new ConvexError(true);
});

export const queryThrowsAfterPromise = query(async () => {
  await Promise.resolve();
  throw new ConvexError(true);
});

export const mutationThrows = mutation(async () => {
  throw new ConvexError(true);
});

export const mutationThrowsNull = mutation(async () => {
  throw new ConvexError(null);
});

export const mutationThrowsString = mutation(async () => {
  throw new ConvexError("Hell yeah");
});

export const mutationThrowsObject = mutation(async () => {
  throw new ConvexError({ foo: "Mike" });
});

export const queryThrowsMessage = query(() => {
  throw new ConvexError("Hello James");
});

export const queryThrowsObjectWithMessage = query(() => {
  throw new ConvexError({ message: "Hello James" });
});

export const queryThrowsCustomSubclass = query(() => {
  class MyFancyError extends ConvexError<string> {
    name = "MyFancyError";

    constructor(message: string) {
      super(message);
    }
  }
  throw new MyFancyError("Hello James");
});

export const queryThrowsCustomSubclassWithObject = query(() => {
  class MyFancyError extends ConvexError<{ message: string; code: string }> {
    name = "MyFancyError";

    constructor(message: string) {
      super({ message, code: "bad boy" });
    }
  }
  throw new MyFancyError("Hello James");
});

export const queryThrowsNotCustomSubclass = query(() => {
  class NotConvexError extends Error {
    ConvexError = true;
    data = "garbage";
  }
  throw new NotConvexError();
});

export const queryThrowsNotCustom = query(() => {
  throw { ConvexError: true, data: "garbage" };
});
