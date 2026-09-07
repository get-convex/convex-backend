import { describe, expect, test } from "vitest";
import { v } from "../../values/validator.js";
import { convexToJson } from "../../values/value.js";
import {
  apiSpecFunctionsToJson,
  validatorSpecToJson,
} from "./functionSpecJson.js";

function bigintLiteralValidatorJson() {
  return JSON.stringify(
    v.object({
      i: v.literal(BigInt(1)),
    }).json,
  );
}

describe("function-spec bigint literals", () => {
  test("convexToJson rejects parsed validator JSON that contains $integer", () => {
    const parsed = JSON.parse(bigintLiteralValidatorJson());
    expect(() => convexToJson(parsed)).toThrow(
      /Field name \$integer starts with a '\$', which is reserved/,
    );
  });

  test("raw validator strings with bigint literals serialize without crashing", () => {
    const validatorJson = bigintLiteralValidatorJson();
    const json = apiSpecFunctionsToJson([
      {
        identifier: "messages:query12",
        functionType: "Query",
        visibility: { kind: "public" },
        args: validatorJson,
        returns: validatorJson,
      },
    ]);
    expect(Array.isArray(json)).toBe(true);
    const spec = (json as any[])[0];
    expect(spec.args.value.i.fieldType.type).toBe("literal");
    expect(spec.args.value.i.fieldType.value).toHaveProperty("$integer");
    expect(spec.returns.value.i.fieldType.value).toHaveProperty("$integer");
  });

  test("legacy object validators still go through convexToJson", () => {
    expect(validatorSpecToJson({ type: "any" })).toEqual({ type: "any" });
  });
});
