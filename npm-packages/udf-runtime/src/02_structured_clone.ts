import { performOp } from "udf-syscall-ffi";
import { throwUncatchableDeveloperError } from "./helpers";

function structuredClone(value: any, options?: { transfer: any[] }) {
  if (options !== undefined) {
    return throwUncatchableDeveloperError(
      "structuredClone with transfer not supported",
    );
  }
  return performOp("structuredClone", value);
}

export const setupStructuredClone = (global: any) => {
  global.structuredClone = structuredClone;
};
