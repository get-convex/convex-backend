import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { test, describe } from "vitest";
import { convexAction, convexQuery } from "./index.js";
import { api } from "../convex/_generated/api.js";

describe("query options factory types", () => {
  test("with useQuery", () => {
    if (1 + 2 === 3) return; // test types only

    type ActionFunc = typeof api.weather.getSFWeather;
    {
      const action = convexAction(api.weather.getSFWeather, {});
      const result = useQuery(action);
      const data: ActionFunc["_returnType"] | undefined = result.data;
      console.log(data);
    }

    {
      const action = convexAction(api.weather.getSFWeather, "skip");
      const result = useQuery(action);
      // Skip doesn't need to cause data in types since there's no point
      // to always passing "skip".
      const data: ActionFunc["_returnType"] | undefined = result.data;
      console.log(data);

      // @ts-expect-error Actions with "skip" can't be used with useSuspenseQuery
      useSuspenseQuery(action);
    }

    type QueryFunc = typeof api.messages.list;
    {
      const query = convexQuery(api.messages.list, {});
      const result = useQuery(query);
      const data: QueryFunc["_returnType"] | undefined = result.data;
      console.log(data);
    }

    {
      const query = convexQuery(api.messages.list, "skip");
      const result = useQuery(query);
      // Skip doesn't need to cause data in types since there's no point
      // to always passing "skip".
      const data: QueryFunc["_returnType"] | undefined = result.data;
      console.log(data);

      // @ts-expect-error Queries with "skip" can't be used with useSuspenseQuery
      useSuspenseQuery(query);
    }
  });
});
