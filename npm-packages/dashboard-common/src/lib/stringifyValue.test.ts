import { stringifyValue } from "@common/lib/stringifyValue";

describe("stringifyValue", () => {
  test.each([
    [null, "null"],
    [BigInt(123), "123n"],
    [123, "123"],
    [true, "true"],
    ["single line string", '"single line string"'],
    ["multi\nline\nstring", "`multi\nline\nstring`"],
    [new ArrayBuffer(8), 'Bytes("AAAAAAAAAAA=")'],
    [[1, 2, 3], "[1, 2, 3]"],
    [{ key: "value" }, '{ key: "value" }'],
    [{ "key with spaces": "value" }, '{ "key with spaces": "value" }'],
    [{ a: `\`\`` }, '{ a: "``" }'],
    [{ a: `\`\${}\`` }, '{ a: "`${}`" }'],
    [{ '"': "value" }, '{ \'"\': "value" }'],
    [{ '\\"': "value" }, '{ \'\\\\"\': "value" }'],
    [{ "\\": "value" }, '{ "\\\\": "value" }'],
    [
      "super-long-string-that-is-longer-than-the-print-width-and-should-not-be-indented",
      '"super-long-string-that-is-longer-than-the-print-width-and-should-not-be-indented"',
    ],
  ])("stringifyValue(%p)", (value, expected) => {
    expect(stringifyValue(value, true)).toBe(expected);
  });
});
