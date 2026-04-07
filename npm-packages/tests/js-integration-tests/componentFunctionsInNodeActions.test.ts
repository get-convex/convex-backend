import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("Calling component functions from root node action", () => {
  let httpClient: ConvexHttpClient;
  beforeEach(async () => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });
  test("Run a node action that calls a component query", async () => {
    await httpClient.action(
      api.componentFunctionsInNodeActions.nodeActionCallingComponentQuery,
    );
  });
  test("Run a node action that calls a component mutation", async () => {
    await httpClient.action(
      api.componentFunctionsInNodeActions.nodeActionCallingComponentMutation,
    );
  });
  test("Run a node action that calls a component action", async () => {
    await httpClient.action(
      api.componentFunctionsInNodeActions.nodeActionCallingComponentAction,
    );
  });
  test("Run a node action that schedules a component function", async () => {
    await httpClient.action(
      api.componentFunctionsInNodeActions.nodeActionSchedulingInComponent,
    );
  });
  test("Run a node action that creates a function handle and calls it", async () => {
    await httpClient.action(
      api.componentFunctionsInNodeActions.nodeActionCreateFunctionHandle,
    );
  });
});
