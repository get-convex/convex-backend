import { FunctionReturnType } from "convex/server";
import { UsePaginatedQueryReturnType } from "convex/react";
import { api } from "../convex/_generated/api";

export function MyHelperComponent(props: {
  data: FunctionReturnType<typeof api.myFunctions.getSomething>;
}) {
  // ...
}

export function MyPaginationHelperComponent(props: {
  paginatedData: UsePaginatedQueryReturnType<
    typeof api.myFunctions.getSomethingPaginated
  >;
}) {
  // ...
}
