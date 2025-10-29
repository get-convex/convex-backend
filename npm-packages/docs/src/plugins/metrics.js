const path = require("path");

module.exports = function () {
  return {
    name: "metrics",
    getClientModules() {
      return [path.resolve(__dirname, "./pagelogger")];
    },
  };
};
