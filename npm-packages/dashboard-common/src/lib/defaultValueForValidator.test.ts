import { defaultValueForValidator } from "./defaultValueForValidator";

describe("defaultValueForValidator", () => {
  it("should use `null` as a default value for `v.union()` (empty union / bottom type / never)", () => {
    const defaultValue = defaultValueForValidator({
      type: "object",
      value: {
        neverField: {
          fieldType: {
            type: "union",
            value: [],
          },
          optional: false,
        },
      },
    });

    expect(defaultValue).toEqual({
      neverField: null,
    });
  });
});
