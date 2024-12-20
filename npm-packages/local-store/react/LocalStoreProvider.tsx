import { createContext, ReactNode, useContext } from "react";
import { LocalStoreClient } from "../browser/ui";
import { SchemaDefinition } from "convex/server";
import { BaseConvexClient } from "convex/browser";
import { Driver } from "../browser/driver";
import { Logger } from "../browser/logger";
import { CoreSyncEngine } from "../browser/core/core";
import { NetworkImpl } from "../browser/network";
import {
  LocalPersistence,
  NoopLocalPersistence,
} from "../browser/localPersistence";
import { MutationRegistry } from "./mutationRegistry";

export const LocalStoreContext = createContext<LocalStoreClient | null>(null);

type LocalStoreProviderProps<SyncSchema extends SchemaDefinition<any, any>> =
  | {
      children: ReactNode;

      client: BaseConvexClient;
      syncSchema: SyncSchema;
      persistence?: LocalPersistence;
      mutations: MutationRegistry<SyncSchema>;
    }
  | {
      children: ReactNode;

      localStoreClient: LocalStoreClient;
    };

export function LocalStoreProvider<
  SyncSchema extends SchemaDefinition<any, any>,
>(props: LocalStoreProviderProps<SyncSchema>) {
  let localStoreClient: LocalStoreClient;
  if ("localStoreClient" in props) {
    localStoreClient = props.localStoreClient;
  } else {
    const { client, syncSchema, mutations, persistence } = props;
    const logger = new Logger();
    const mutationMap = mutations.exportToMutationMap();
    const coreLocalStore = new CoreSyncEngine(syncSchema, mutationMap, logger);
    const driver = new Driver({
      coreLocalStore,
      network: new NetworkImpl({ convexClient: client }),
      localPersistence: persistence ?? new NoopLocalPersistence(),
      logger,
    });
    localStoreClient = new LocalStoreClient({
      driver,
      syncSchema,
      mutations: mutationMap,
    });
  }
  (globalThis as any).localDb = localStoreClient;
  return (
    <LocalStoreContext.Provider value={localStoreClient}>
      {props.children}
    </LocalStoreContext.Provider>
  );
}

export function useLocalStoreClient(): LocalStoreClient {
  const localStoreClient = useContext(LocalStoreContext);
  if (localStoreClient === null) {
    throw new Error(
      "useLocalStoreClient must be used within a LocalStoreProvider",
    );
  }
  return localStoreClient;
}
