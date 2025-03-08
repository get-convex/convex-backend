// Mock for convex/values
module.exports = {
  JSONValue: {},
  ValidatorJSON: {},
  Value: {},
  jsonToConvex: (value) => {
    // Simple validation: reject objects with keys starting with $
    if (typeof value === "object" && value !== null) {
      for (const key in value) {
        if (key.startsWith("$")) {
          throw new Error(`Invalid value: ${JSON.stringify(value)}.`);
        }
      }
    }
    return value;
  },
  validateValue: (value) => {
    // Simple validation: reject objects with keys starting with $
    if (typeof value === "object" && value !== null) {
      for (const key in value) {
        if (key.startsWith("$")) {
          throw new Error(`Invalid value: ${JSON.stringify(value)}.`);
        }
      }
    }
    return value;
  },
};
