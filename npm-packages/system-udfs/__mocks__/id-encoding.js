// Mock for id-encoding
module.exports = {
  isId: (value) => typeof value === "string" && value.length === 32,
};
