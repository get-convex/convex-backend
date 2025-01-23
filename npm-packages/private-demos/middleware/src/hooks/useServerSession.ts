/**
 * React helpers for adding session data to Convex functions.
 *
 * !Important!: To use these functions, you must wrap your code with
 * ```tsx
 *  <ConvexProvider client={convex}>
 *    <SessionProvider storageLocation={"sessionStorage"}>
 *      <App />
 *    </SessionProvider>
 *  </ConvexProvider>
 * ```
 *
 * With the `SessionProvider` inside the `ConvexProvider` but outside your app.
 */
import React, { useContext, useEffect, useState } from "react";
import { api } from "../../convex/_generated/api";
import { Id } from "../../convex/_generated/dataModel";
import { useQuery, useMutation, useAction, useConvex } from "convex/react";
import { filterApi, FunctionReference, OptionalRestArgs } from "convex/server";

const StoreKey = "ConvexSessionId";

const SessionContext = React.createContext<Id<"sessions"> | null>(null);

/**
 * Context for a Convex session, creating a server session and providing the id.
 *
 * @param props - Where you want your session ID to be persisted. Roughly:
 *  - sessionStorage is saved per-tab
 *  - localStorage is shared between tabs, but not browser profiles.
 * @returns A provider to wrap your React nodes which provides the session ID.
 * To be used with useSessionQuery and useSessionMutation.
 */
export const SessionProvider: React.FC<{
  storageLocation?: "localStorage" | "sessionStorage";
  children?: React.ReactNode;
}> = ({ storageLocation, children }) => {
  const store =
    // If it's rendering in SSR or such.
    typeof window === "undefined"
      ? null
      : window[storageLocation ?? "sessionStorage"];
  const [sessionId, setSession] = useState<Id<"sessions"> | null>(() => {
    const stored = store?.getItem(StoreKey);
    if (stored) {
      return stored as Id<"sessions">;
    }
    return null;
  });
  const createSession = useMutation(api.sessions.create);

  // Get or set the ID from our desired storage location, whenever it changes.
  useEffect(() => {
    if (sessionId) {
      store?.setItem(StoreKey, sessionId);
    } else {
      void (async () => {
        setSession(await createSession());
      })();
    }
  }, [sessionId, createSession, store]);

  return React.createElement(
    SessionContext.Provider,
    { value: sessionId },
    children,
  );
};

/**
 * Hack! This type causes TypeScript to simplify how it renders object types.
 *
 * It is functionally the identity for object types, but in practice it can
 * simplify expressions like `A & B`.
 */
declare type Expand<ObjectType extends Record<any, any>> =
  ObjectType extends Record<any, any>
    ? {
        [Key in keyof ObjectType]: ObjectType[Key];
      }
    : never;

/**
 * An `Omit<>` type that:
 * 1. Applies to each element of a union.
 * 2. Preserves the index signature of the underlying type.
 */
declare type BetterOmit<T, K extends keyof T> = {
  [Property in keyof T as Property extends K ? never : Property]: T[Property];
};

type SessionFunction = FunctionReference<
  any,
  "public",
  { sessionId: Id<"sessions"> | null },
  any
>;

// All the queries that take sessionId as a parameter.
export const justSessionQueries = <API>(api: API) =>
  filterApi<
    typeof api,
    FunctionReference<
      "query",
      "public",
      { sessionId: Id<"sessions"> | null },
      any
    >
  >(api);

// All the mutations that take sessionId as a parameter.
export const justSessionMutations = <API>(api: API) =>
  filterApi<
    typeof api,
    FunctionReference<
      "mutation",
      "public",
      { sessionId: Id<"sessions"> | null },
      any
    >
  >(api);

// All the actions that take sessionId as a parameter.
export const justSessionActions = <API>(api: API) =>
  filterApi<
    typeof api,
    FunctionReference<
      "action",
      "public",
      { sessionId: Id<"sessions"> | null },
      any
    >
  >(api);

type EmptyObject = Record<string, never>;

/**
 * Just remove sessionId
 */
type SessionFunctionArgs<Fn extends SessionFunction> =
  keyof Fn["_args"] extends "sessionId"
    ? EmptyObject
    : Expand<BetterOmit<Fn["_args"], "sessionId">>;

/**
 * Util
 */
type SessionFunctionRestArgs<Fn extends SessionFunction> =
  keyof Fn["_args"] extends "sessionId"
    ? []
    : [Expand<BetterOmit<Fn["_args"], "sessionId">>];

// Like useQuery, but for a Query that takes a session ID.
export function useSessionQueryOverload<
  Query extends FunctionReference<
    "query",
    "public",
    { sessionId: Id<"sessions"> | null },
    any
  >,
>(
  query: SessionFunctionArgs<Query> extends EmptyObject ? Query : never,
): Query["_returnType"] | undefined;
export function useSessionQueryOverload<
  Query extends FunctionReference<
    "query",
    "public",
    { sessionId: Id<"sessions"> | null },
    any
  >,
>(
  query: Query,
  args: SessionFunctionArgs<Query>,
): Query["_returnType"] | undefined;
export function useSessionQueryOverload<
  Query extends FunctionReference<
    "query",
    "public",
    { sessionId: Id<"sessions"> | null },
    any
  >,
>(
  query: Query,
  args?: SessionFunctionArgs<Query>,
): Query["_returnType"] | undefined {
  const sessionId = useContext(SessionContext);
  const newArgs = { ...args, sessionId } as Query["_args"];
  // This is a pain! Can we improve this?
  return useQuery(query, ...([newArgs] as OptionalRestArgs<Query>));
}

// Original
export const useSessionQuery = <
  Query extends FunctionReference<
    "query",
    "public",
    { sessionId: Id<"sessions"> | null },
    any
  >,
>(
  name: Query,
  args?: SessionFunctionArgs<Query>,
) => {
  const sessionId = useContext(SessionContext);
  const newArgs = {
    ...args,
    sessionId,
  } as Query["_args"];
  return useQuery(name, ...([newArgs] as OptionalRestArgs<Query>));
};

// Like useMutation, but for a Mutation that takes a session ID.
export const useSessionMutation = <
  Mutation extends FunctionReference<
    "mutation",
    "public",
    { sessionId: any },
    any
  >,
>(
  mutation: Mutation,
) => {
  const sessionId = useContext(SessionContext);
  const originalMutation = useMutation(mutation);

  const convex = useConvex();
  return (
    args: SessionFunctionArgs<Mutation>,
  ): Promise<Mutation["_returnType"]> => {
    const newArgs = { ...args, sessionId };

    if (Math.random() > 0.5) {
      // this is one way this works
      return convex.mutation(
        mutation,
        ...([newArgs] as OptionalRestArgs<Mutation>),
      );
    } else {
      // this is another way this works
      return originalMutation(...([newArgs] as OptionalRestArgs<Mutation>));
    }
  };
};

// Like useAction, but for a Action that takes a session ID.
export const useSessionAction = <
  Action extends FunctionReference<"action", "public", { sessionId: any }, any>,
>(
  action: Action,
) => {
  const sessionId = useContext(SessionContext);
  const originalAction = useAction(action);
  return (args: SessionFunctionArgs<Action>): Promise<Action["_args"]> => {
    const newArgs = { ...args, sessionId };
    // This is annoying
    return originalAction(...([newArgs] as OptionalRestArgs<Action>));
  };
};

/**
 * TESTS
 */

/**
 * Tests if two types are exactly the same.
 * Taken from https://github.com/Microsoft/TypeScript/issues/27024#issuecomment-421529650
 * (Apache Version 2.0, January 2004)
 */
export type Equals<X, Y> =
  (<T>() => T extends X ? 1 : 2) extends <T>() => T extends Y ? 1 : 2
    ? true
    : false;

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function assert<T extends true>() {
  // no need to do anything! we're just asserting at compile time that the type
  // parameter is true.
}

// SessionFunctionRestArgs produces args only for args other than sessionId
assert<
  Equals<
    SessionFunctionArgs<
      FunctionReference<
        any,
        any,
        { sessionId: Id<"sessions">; args: string },
        any
      >
    >,
    { args: string }
  >
>();
assert<
  Equals<
    SessionFunctionRestArgs<
      FunctionReference<any, any, { sessionId: Id<"sessions"> }, any>
    >,
    []
  >
>();

// the same, but with | null
assert<
  Equals<
    SessionFunctionRestArgs<
      FunctionReference<
        any,
        any,
        { sessionId: Id<"sessions"> | null; args: string },
        any
      >
    >,
    [{ args: string }]
  >
>();
assert<
  Equals<
    SessionFunctionRestArgs<
      FunctionReference<any, any, { sessionId: Id<"sessions"> | null }, any>
    >,
    []
  >
>();
