"use node";
import { action } from "./_generated/server";
import fs from "node:fs";

export default action(async (_ctx) => {
  return await new Promise((_resolve, _reject) => {
    fs.exists("asdf", function (_exists) {
      throw new Error("Yikes");
    });
  });
});
