import { performOp } from "../../syscall";
import { queryPrivateSystem } from "../secretSystemTables";

/**
 * Returns an object mapping the table numbers to table names
 * (e.g. {"1": "users"})
 */
export default queryPrivateSystem({
  args: {},
  handler: async () => {
    return performOp("getTableMapping");
  },
});
