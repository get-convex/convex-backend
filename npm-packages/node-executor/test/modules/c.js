/*global Convex*/

require("fs");
require("node:fs");

import * as fs from "fs";
console.assert(fs.constants.O_CREAT);
import * as fs2 from "node:fs";
console.assert(fs2.constants.O_CREAT);

export default {
  isAction: true,

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  invokeAction: async (requestId, args) => {
    const mutationArgs = {
      name: "incrementCounter.js",
      args: [1],
      version: "0.2.1",
      requestId,
    };
    JSON.parse(
      await Convex.asyncSyscall(
        "1.0/actions/mutation",
        JSON.stringify(mutationArgs),
      ),
    );

    const queryArgs = {
      name: "getCounter.js",
      args: [],
      version: "0.2.1",
      requestId,
    };
    return Convex.asyncSyscall("1.0/actions/query", JSON.stringify(queryArgs));
  },
};
