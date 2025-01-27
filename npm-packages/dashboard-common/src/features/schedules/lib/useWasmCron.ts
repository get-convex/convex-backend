import { useState } from "react";
import type * as WasmCronModule from "saffron";

let globalWasmCronP: Promise<typeof WasmCronModule>;
let globalWasmCron: typeof WasmCronModule.WasmCron;

/**
 * Hax to load the Saffron WebAssembly module only in the browser because
 * I haven't figured out how to bundle Saffron for Node.js in Next.js.
 */
export function useWasmCron(): typeof WasmCronModule.WasmCron | undefined {
  const [wasmCron, setWasmCron] = useState<
    typeof WasmCronModule.WasmCron | undefined
  >(() => globalWasmCron);
  if (wasmCron) return wasmCron;

  if (!globalWasmCronP && typeof window !== "undefined") {
    globalWasmCronP = import("saffron");
    void globalWasmCronP.then((module) => {
      globalWasmCron = module.WasmCron;
    });
  }

  void globalWasmCronP.then(() => {
    setWasmCron(() => globalWasmCron);
  });

  return wasmCron;
}
