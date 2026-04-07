import { ConvexError } from "convex/values";
import { mutation, query } from "./_generated/server";

export const queryThrows = query({
  args: {},
  handler: () => {
    throw new ConvexError(true);
  },
});

export const queryThrowsAsync = query({
  args: {},
  handler: async () => {
    throw new ConvexError(true);
  },
});

export const queryThrowsAfterPromise = query({
  args: {},
  handler: async () => {
    await Promise.resolve();
    throw new ConvexError(true);
  },
});

export const mutationThrows = mutation({
  args: {},
  handler: async () => {
    throw new ConvexError(true);
  },
});

export const mutationThrowsNull = mutation({
  args: {},
  handler: async () => {
    throw new ConvexError(null);
  },
});

export const mutationThrowsString = mutation({
  args: {},
  handler: async () => {
    throw new ConvexError("Hell yeah");
  },
});

export const mutationThrowsObject = mutation({
  args: {},
  handler: async () => {
    throw new ConvexError({ foo: "Mike" });
  },
});

export const queryThrowsMessage = query({
  args: {},
  handler: () => {
    throw new ConvexError("Hello James");
  },
});

export const queryThrowsObjectWithMessage = query({
  args: {},
  handler: () => {
    throw new ConvexError({ message: "Hello James" });
  },
});

export const queryThrowsCustomSubclass = query({
  args: {},
  handler: () => {
    class MyFancyError extends ConvexError<string> {
      name = "MyFancyError";

      constructor(message: string) {
        super(message);
      }
    }
    throw new MyFancyError("Hello James");
  },
});

export const queryThrowsCustomSubclassWithObject = query({
  args: {},
  handler: () => {
    class MyFancyError extends ConvexError<{ message: string; code: string }> {
      name = "MyFancyError";

      constructor(message: string) {
        super({ message, code: "bad boy" });
      }
    }
    throw new MyFancyError("Hello James");
  },
});

export const queryThrowsNotCustomSubclass = query({
  args: {},
  handler: () => {
    class NotConvexError extends Error {
      ConvexError = true;
      data = "garbage";
    }
    throw new NotConvexError();
  },
});

export const queryThrowsNotCustom = query({
  args: {},
  handler: () => {
    throw { ConvexError: true, data: "garbage" };
  },
});
