import { BaseConvexClient, FunctionResult, QueryToken } from "convex/browser";
import {
  ConvexSubscriptionId,
  MutationId,
  MutationInfo,
  PageArguments,
  SyncFunction,
} from "../shared/types";
import { getFunctionName } from "convex/server";

type Unsubscribe = () => void;

export interface Network {
  sendQueryToNetwork(syncFunction: SyncFunction, args: PageArguments): void;
  removeQueryFromNetwork(queriesToRemove: ConvexSubscriptionId[]): void;
  sendMutationToNetwork(mutationInfo: MutationInfo): void;
  getMutationId(requestId: number): MutationId | null;
  convexClient: BaseConvexClient;
  addOnTransitionHandler(handler: (transition: Transition) => void): void;
}

export class NetworkImpl implements Network {
  private subscriptions: Map<ConvexSubscriptionId, Unsubscribe> = new Map();
  private mutationMapping: Map<
    number,
    {
      mutationId: MutationId;
      result: Promise<any>;
    }
  > = new Map();

  convexClient: BaseConvexClient;
  constructor(opts: { convexClient: BaseConvexClient }) {
    this.convexClient = opts.convexClient;
  }

  sendQueryToNetwork(syncFunction: SyncFunction, args: PageArguments): void {
    const { queryToken, unsubscribe } = this.convexClient.subscribe(
      getFunctionName(syncFunction),
      args as any,
    );
    this.subscriptions.set(queryToken as ConvexSubscriptionId, unsubscribe);
  }

  removeQueryFromNetwork(queriesToRemove: ConvexSubscriptionId[]): void {
    for (const query of queriesToRemove) {
      const unsubscribe = this.subscriptions.get(query);
      if (unsubscribe) {
        unsubscribe();
      }
      this.subscriptions.delete(query);
    }
  }

  sendMutationToNetwork(mutationInfo: MutationInfo): void {
    // TODO: internal types aren't working.
    const { requestId, mutationPromise } = (
      this.convexClient as any
    ).enqueueMutation(
      getFunctionName(mutationInfo.mutationPath),
      mutationInfo.serverArgs as any,
    );
    this.mutationMapping.set(requestId, {
      mutationId: mutationInfo.mutationId,
      result: mutationPromise,
    });
  }

  getMutationId(requestId: number): MutationId | null {
    const mutation = this.mutationMapping.get(requestId);
    if (!mutation) {
      return null;
    }
    return mutation.mutationId;
  }

  addOnTransitionHandler(handler: (transition: Transition) => void) {
    this.convexClient.addOnTransitionHandler(handler);
  }
}

// TODO: Import these from convex
type QueryModification =
  // `undefined` generally comes from an optimistic update setting the query to be loading
  { kind: "Updated"; result: FunctionResult | undefined } | { kind: "Removed" };

type Transition = {
  queries: Array<{ token: QueryToken; modification: QueryModification }>;
  reflectedMutations: Array<{ requestId: any; result: FunctionResult }>;
  timestamp: any;
};
