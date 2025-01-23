import { FunctionHandle } from "convex/server";
import { v, VString } from "convex/values";

export function functionValidator<T extends FunctionHandle<any>>(): VString<T> {
  return v.string() as any;
}
