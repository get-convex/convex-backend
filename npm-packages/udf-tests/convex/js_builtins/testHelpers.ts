import { use } from "chai";
import chaiAsPromised from "chai-as-promised";
import chai from "chai";

export const wrapInTests = async (
  tests: Record<string, () => Promise<void> | void>,
): Promise<string> => {
  use(chaiAsPromised);
  // Disable truncation of error messages.
  chai.config.truncateThreshold = 0;
  for (const [_name, func] of Object.entries(tests)) {
    await func();
  }
  return "success";
};
