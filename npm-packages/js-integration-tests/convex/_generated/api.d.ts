/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as actions_auth from "../actions/auth.js";
import type * as actions_externalNodeDeps from "../actions/externalNodeDeps.js";
import type * as actions_simple from "../actions/simple.js";
import type * as addUser from "../addUser.js";
import type * as auth from "../auth.js";
import type * as basic from "../basic.js";
import type * as cachebust from "../cachebust.js";
import type * as cleanUp from "../cleanUp.js";
import type * as component from "../component.js";
import type * as componentFunctionsInNodeActions from "../componentFunctionsInNodeActions.js";
import type * as counter from "../counter.js";
import type * as customErrors from "../customErrors.js";
import type * as customErrorsNodeActions from "../customErrorsNodeActions.js";
import type * as error from "../error.js";
import type * as fileStorage from "../fileStorage.js";
import type * as fileStorageInComponent from "../fileStorageInComponent.js";
import type * as fileStorageNodeActions from "../fileStorageNodeActions.js";
import type * as fileStorageV8Actions from "../fileStorageV8Actions.js";
import type * as findObject from "../findObject.js";
import type * as foods from "../foods.js";
import type * as getObject from "../getObject.js";
import type * as getUsers from "../getUsers.js";
import type * as http from "../http.js";
import type * as internal_ from "../internal.js";
import type * as logging from "../logging.js";
import type * as maps from "../maps.js";
import type * as messages from "../messages.js";
import type * as mountedSearch from "../mountedSearch.js";
import type * as nodeError from "../nodeError.js";
import type * as references from "../references.js";
import type * as removeObject from "../removeObject.js";
import type * as scheduler from "../scheduler.js";
import type * as secretSystemTables from "../secretSystemTables.js";
import type * as sets from "../sets.js";
import type * as stacktraceNode from "../stacktraceNode.js";
import type * as stagedIndexes from "../stagedIndexes.js";
import type * as storeObject from "../storeObject.js";
import type * as systemTables from "../systemTables.js";
import type * as textSearch from "../textSearch.js";
import type * as updateObject from "../updateObject.js";
import type * as vectorActionNode from "../vectorActionNode.js";
import type * as vectorActionV8 from "../vectorActionV8.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

/**
 * A utility for referencing Convex functions in your app's API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
declare const fullApi: ApiFromModules<{
  "actions/auth": typeof actions_auth;
  "actions/externalNodeDeps": typeof actions_externalNodeDeps;
  "actions/simple": typeof actions_simple;
  addUser: typeof addUser;
  auth: typeof auth;
  basic: typeof basic;
  cachebust: typeof cachebust;
  cleanUp: typeof cleanUp;
  component: typeof component;
  componentFunctionsInNodeActions: typeof componentFunctionsInNodeActions;
  counter: typeof counter;
  customErrors: typeof customErrors;
  customErrorsNodeActions: typeof customErrorsNodeActions;
  error: typeof error;
  fileStorage: typeof fileStorage;
  fileStorageInComponent: typeof fileStorageInComponent;
  fileStorageNodeActions: typeof fileStorageNodeActions;
  fileStorageV8Actions: typeof fileStorageV8Actions;
  findObject: typeof findObject;
  foods: typeof foods;
  getObject: typeof getObject;
  getUsers: typeof getUsers;
  http: typeof http;
  internal: typeof internal_;
  logging: typeof logging;
  maps: typeof maps;
  messages: typeof messages;
  mountedSearch: typeof mountedSearch;
  nodeError: typeof nodeError;
  references: typeof references;
  removeObject: typeof removeObject;
  scheduler: typeof scheduler;
  secretSystemTables: typeof secretSystemTables;
  sets: typeof sets;
  stacktraceNode: typeof stacktraceNode;
  stagedIndexes: typeof stagedIndexes;
  storeObject: typeof storeObject;
  systemTables: typeof systemTables;
  textSearch: typeof textSearch;
  updateObject: typeof updateObject;
  vectorActionNode: typeof vectorActionNode;
  vectorActionV8: typeof vectorActionV8;
}>;
declare const fullApiWithMounts: typeof fullApi;

export declare const api: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "public">
>;
export declare const internal: FilterApi<
  typeof fullApiWithMounts,
  FunctionReference<any, "internal">
>;

export declare const components: {
  component: {
    cleanUp: {
      default: FunctionReference<"mutation", "internal", any, any>;
    };
    errors: {
      throwConvexError: FunctionReference<"query", "internal", any, any>;
      throwError: FunctionReference<"query", "internal", any, any>;
    };
    fileStorage: {
      deleteById: FunctionReference<
        "mutation",
        "internal",
        { storageId: string },
        any
      >;
      generateUploadUrl: FunctionReference<"mutation", "internal", any, any>;
      get: FunctionReference<"query", "internal", { id: string }, any>;
      getFile: FunctionReference<
        "action",
        "internal",
        { storageId: string },
        any
      >;
      getUrl: FunctionReference<
        "query",
        "internal",
        { storageId: string },
        any
      >;
      list: FunctionReference<"query", "internal", any, any>;
      storeFile: FunctionReference<"action", "internal", { data: string }, any>;
    };
    functionHandles: {
      fromAction: FunctionReference<"action", "internal", any, any>;
      fromQuery: FunctionReference<"query", "internal", any, any>;
      getInternalHandle: FunctionReference<
        "query",
        "internal",
        { functionType: "query" | "mutation" | "action" },
        string
      >;
    };
    scheduler: {
      listAllMessages: FunctionReference<"query", "internal", any, any>;
      scheduleWithinComponent: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        string
      >;
      sendMessage: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any
      >;
      status: FunctionReference<"query", "internal", { id: string }, any>;
    };
    transact: {
      allMessages: FunctionReference<"query", "internal", {}, Array<string>>;
      sendButFail: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any
      >;
      sendMessage: FunctionReference<
        "mutation",
        "internal",
        { message: string },
        any
      >;
    };
  };
  searchComponent: {
    cleanUp: {
      default: FunctionReference<"mutation", "internal", any, any>;
    };
    foods: {
      insertRow: FunctionReference<
        "mutation",
        "internal",
        { cuisine: string; description: string; embedding: Array<number> },
        any
      >;
      populate: FunctionReference<"action", "internal", {}, any>;
      queryDocs: FunctionReference<
        "query",
        "internal",
        { ids: Array<string> },
        any
      >;
    };
    textSearch: {
      fullTextSearchMutation: FunctionReference<
        "mutation",
        "internal",
        { cuisine?: string; query: string },
        any
      >;
      fullTextSearchMutationWithWrite: FunctionReference<
        "mutation",
        "internal",
        { cuisine?: string; query: string },
        any
      >;
      fullTextSearchQuery: FunctionReference<
        "query",
        "internal",
        { cuisine?: string; query: string },
        any
      >;
    };
    vectorActionV8: {
      vectorSearch: FunctionReference<
        "action",
        "internal",
        { cuisine: string; embedding: Array<number> },
        any
      >;
    };
  };
};
