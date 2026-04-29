import { audit } from "./audit_logging.js";
import { vars } from "./logVars.js";

// Type annotations are needed for the `unique symbol` types in `vars` to typecheck correctly
interface Log {
  audit: typeof audit;
  vars: typeof vars;
}

/**
 * @internal
 */
export const log: Log = {
  audit,
  vars,
};
