import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("ConvexHttpClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("consistent queries use old timestamps", async () => {
    const counter = await httpClient.mutation(api.counter.create);
    const count = await httpClient.query(api.counter.get, { counter });
    expect(count).toBe(0);

    await httpClient.mutation(api.counter.increment, { counter });
    const count2 = await httpClient.query(api.counter.get, { counter });
    expect(count2).toBe(1);
    const count3 = await httpClient.consistentQuery(api.counter.get, {
      counter,
    });
    expect(count3).toBe(1);

    await httpClient.mutation(api.counter.increment, { counter });
    const count4 = await httpClient.consistentQuery(api.counter.get, {
      counter,
    });
    expect(count4).toBe(1);
    const count5 = await httpClient.query(api.counter.get, { counter });
    expect(count5).toBe(2);
  });
});
