"use node";
import { action } from "../_generated/server";

export default action(async () => {
  const p = new Promise((r) => setTimeout(r, 10 * 1000));
  await p;
  return "success";
});
