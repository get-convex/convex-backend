"use node";

import { action } from "./_generated/server";

export default action(async () => {
  console.log("starting something that takes time...");
  if (Math.random() > 0.5) {
    fail();
  }
  await new Promise((r) => setTimeout(r, 15000));
  console.log("done");
});

function fail() {
  fail2();
}

function fail2() {
  throw new Error("Oh no!");
}
