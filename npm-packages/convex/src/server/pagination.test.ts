import { assert } from "../test/type_testing.js";
import { test } from "vitest";
import { PaginationOptions, paginationOptsValidator } from "./pagination.js";
import { Infer } from "../values/validator.js";

test("paginationOptsValidator matches the paginationOpts type", () => {
  type validatorType = Infer<typeof paginationOptsValidator>;
  assert<validatorType extends PaginationOptions ? true : false>();
  // All optional fields exist and have the correct type.
  assert<
    Required<validatorType> extends Required<PaginationOptions> ? true : false
  >();
});
