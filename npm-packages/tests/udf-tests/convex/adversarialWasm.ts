// This file is also doing all kinds of wonky things so just turn off eslint
/* eslint-disable */

import { query } from "./_generated/server";
import { wasmSource } from "./wasmTests";

export const simpleLoop = query(async function () {
  const go = new Go();
  const { instance } = await WebAssembly.instantiate(
    wasmBuffer,
    go.importObject,
  );
  await go.run(instance);
  const exports = instance.exports as any;

  return exports.simpleLoop();
});

export const allocatingLoop = query(async function () {
  const go = new Go();
  const { instance } = await WebAssembly.instantiate(
    wasmBuffer,
    go.importObject,
  );
  await go.run(instance);
  const exports = instance.exports as any;
  return exports.allocatingLoop();
});

const wasmBinary = atob(wasmSource);
const wasmBuffer = new Uint8Array(wasmBinary.length);
for (let i = 0; i < wasmBinary.length; i++) {
  wasmBuffer[i] = wasmBinary.charCodeAt(i);
}

// Copyright 2018 The Go Authors. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.
//
// This file has been modified for use by the TinyGo compiler.

const enosys = () => {
  const err = new Error("not implemented");
  (err as any).code = "ENOSYS";
  return err;
};

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8");
const reinterpretBuf = new DataView(new ArrayBuffer(8));
let logLine: any[] = [];

export class Go {
  importObject: any;
  _inst: any;
  _values: any;
  _goRefCounts: any;
  _ids: any;
  _idPool: any;
  exited: any;
  _resolveExitPromise: any;
  _resolveCallbackPromise: any;
  _pendingEvent: any;
  _callbackTimeouts: any;
  _nextCallbackTimeoutID: any;

  constructor() {
    this._callbackTimeouts = new Map();
    this._nextCallbackTimeoutID = 1;

    const mem = () => {
      // The buffer may change when requesting more memory.
      return new DataView(this._inst.exports.memory.buffer);
    };

    const unboxValue = (v_ref: any) => {
      reinterpretBuf.setBigInt64(0, v_ref, true);
      const f = reinterpretBuf.getFloat64(0, true);
      if (f === 0) {
        return undefined;
      }
      if (!isNaN(f)) {
        return f;
      }

      const id = v_ref & 0xffffffffn;
      return this._values[id as any];
    };

    const loadValue = (addr: any) => {
      const v_ref = mem().getBigUint64(addr, true);
      return unboxValue(v_ref);
    };

    const boxValue = (v: any) => {
      const nanHead = 0x7ff80000n;

      if (typeof v === "number") {
        if (isNaN(v)) {
          return nanHead << 32n;
        }
        if (v === 0) {
          return (nanHead << 32n) | 1n;
        }
        reinterpretBuf.setFloat64(0, v, true);
        return reinterpretBuf.getBigInt64(0, true);
      }

      switch (v) {
        case undefined:
          return 0n;
        case null:
          return (nanHead << 32n) | 2n;
        case true:
          return (nanHead << 32n) | 3n;
        case false:
          return (nanHead << 32n) | 4n;
      }

      let id = this._ids.get(v);
      if (id === undefined) {
        id = this._idPool.pop();
        if (id === undefined) {
          id = BigInt(this._values.length);
        }
        this._values[id] = v;
        this._goRefCounts[id] = 0;
        this._ids.set(v, id);
      }
      this._goRefCounts[id]++;
      let typeFlag = 1n;
      switch (typeof v) {
        case "string":
          typeFlag = 2n;
          break;
        case "symbol":
          typeFlag = 3n;
          break;
        case "function":
          typeFlag = 4n;
          break;
      }
      return id | ((nanHead | typeFlag) << 32n);
    };

    const storeValue = (addr: any, v: any) => {
      const v_ref = boxValue(v);
      mem().setBigUint64(addr, v_ref, true);
    };

    const loadSlice = (array: any, len: any, cap?: any) => {
      return new Uint8Array(this._inst.exports.memory.buffer, array, len);
    };

    const loadSliceOfValues = (array: any, len: any, cap: any) => {
      const a = new Array(len);
      for (let i = 0; i < len; i++) {
        a[i] = loadValue(array + i * 8);
      }
      return a;
    };

    const loadString = (ptr: any, len: any) => {
      return decoder.decode(
        new DataView(this._inst.exports.memory.buffer, ptr, len),
      );
    };

    this.importObject = {
      wasi_snapshot_preview1: {
        // https://github.com/WebAssembly/WASI/blob/main/phases/snapshot/docs.md#fd_write
        fd_write: function (
          fd: any,
          iovs_ptr: any,
          iovs_len: any,
          nwritten_ptr: any,
        ) {
          let nwritten = 0;
          if (fd == 1) {
            for (let iovs_i = 0; iovs_i < iovs_len; iovs_i++) {
              const iov_ptr = iovs_ptr + iovs_i * 8; // assuming wasm32
              const ptr = mem().getUint32(iov_ptr + 0, true);
              const len = mem().getUint32(iov_ptr + 4, true);
              nwritten += len;
              for (let i = 0; i < len; i++) {
                const c = mem().getUint8(ptr + i);
                if (c == 13) {
                  // CR
                  // ignore
                } else if (c == 10) {
                  // LF
                  // write line
                  const line = decoder.decode(new Uint8Array(logLine));
                  logLine = [];
                  console.log(line);
                } else {
                  logLine.push(c);
                }
              }
            }
          } else {
            console.error("invalid file descriptor:", fd);
          }
          mem().setUint32(nwritten_ptr, nwritten, true);
          return 0;
        },
        fd_close: () => 0, // dummy
        fd_fdstat_get: () => 0, // dummy
        fd_seek: () => 0, // dummy
        proc_exit: (code: any) => {
          // Can't exit in a browser.
          throw "trying to exit with code " + code;
        },
        random_get: (bufPtr: any, bufLen: any) => {
          crypto.getRandomValues(loadSlice(bufPtr, bufLen));
          return 0;
        },
      },
      gojs: {
        // func ticks() float64
        "runtime.ticks": () => {
          return Date.now();
        },

        // func sleepTicks(timeout float64)
        "runtime.sleepTicks": (timeout: any) => {
          // Do not sleep, only reactivate scheduler after the given timeout.
          setTimeout(this._inst.exports.go_scheduler, timeout);
        },

        // func finalizeRef(v ref)
        "syscall/js.finalizeRef": (v_ref: any) => {
          // Note: TinyGo does not support finalizers so this should never be
          // called.
          console.error("syscall/js.finalizeRef not implemented");
        },

        // func stringVal(value string) ref
        "syscall/js.stringVal": (value_ptr: any, value_len: any) => {
          const s = loadString(value_ptr, value_len);
          return boxValue(s);
        },

        // func valueGet(v ref, p string) ref
        "syscall/js.valueGet": (v_ref: any, p_ptr: any, p_len: any) => {
          const prop = loadString(p_ptr, p_len);
          const v = unboxValue(v_ref);
          const result = Reflect.get(v, prop);
          return boxValue(result);
        },

        // func valueSet(v ref, p string, x ref)
        "syscall/js.valueSet": (
          v_ref: any,
          p_ptr: any,
          p_len: any,
          x_ref: any,
        ) => {
          const v = unboxValue(v_ref);
          const p = loadString(p_ptr, p_len);
          const x = unboxValue(x_ref);
          Reflect.set(v, p, x);
        },

        // func valueDelete(v ref, p string)
        "syscall/js.valueDelete": (v_ref: any, p_ptr: any, p_len: any) => {
          const v = unboxValue(v_ref);
          const p = loadString(p_ptr, p_len);
          Reflect.deleteProperty(v, p);
        },

        // func valueIndex(v ref, i int) ref
        "syscall/js.valueIndex": (v_ref: any, i: any) => {
          return boxValue(Reflect.get(unboxValue(v_ref), i));
        },

        // valueSetIndex(v ref, i int, x ref)
        "syscall/js.valueSetIndex": (v_ref: any, i: any, x_ref: any) => {
          Reflect.set(unboxValue(v_ref), i, unboxValue(x_ref));
        },

        // func valueCall(v ref, m string, args []ref) (ref, bool)
        "syscall/js.valueCall": (
          ret_addr: any,
          v_ref: any,
          m_ptr: any,
          m_len: any,
          args_ptr: any,
          args_len: any,
          args_cap: any,
        ) => {
          const v = unboxValue(v_ref);
          const name = loadString(m_ptr, m_len);
          const args = loadSliceOfValues(args_ptr, args_len, args_cap);
          try {
            const m = Reflect.get(v, name);
            storeValue(ret_addr, Reflect.apply(m, v, args));
            mem().setUint8(ret_addr + 8, 1);
          } catch (err) {
            storeValue(ret_addr, err);
            mem().setUint8(ret_addr + 8, 0);
          }
        },

        // func valueInvoke(v ref, args []ref) (ref, bool)
        "syscall/js.valueInvoke": (
          ret_addr: any,
          v_ref: any,
          args_ptr: any,
          args_len: any,
          args_cap: any,
        ) => {
          try {
            const v = unboxValue(v_ref);
            const args = loadSliceOfValues(args_ptr, args_len, args_cap);
            storeValue(ret_addr, Reflect.apply(v, undefined, args));
            mem().setUint8(ret_addr + 8, 1);
          } catch (err) {
            storeValue(ret_addr, err);
            mem().setUint8(ret_addr + 8, 0);
          }
        },

        // func valueNew(v ref, args []ref) (ref, bool)
        "syscall/js.valueNew": (
          ret_addr: any,
          v_ref: any,
          args_ptr: any,
          args_len: any,
          args_cap: any,
        ) => {
          const v = unboxValue(v_ref);
          const args = loadSliceOfValues(args_ptr, args_len, args_cap);
          try {
            storeValue(ret_addr, Reflect.construct(v, args));
            mem().setUint8(ret_addr + 8, 1);
          } catch (err) {
            storeValue(ret_addr, err);
            mem().setUint8(ret_addr + 8, 0);
          }
        },

        // func valueLength(v ref) int
        "syscall/js.valueLength": (v_ref: any) => {
          return unboxValue(v_ref).length;
        },

        // valuePrepareString(v ref) (ref, int)
        "syscall/js.valuePrepareString": (ret_addr: any, v_ref: any) => {
          const s = String(unboxValue(v_ref));
          const str = encoder.encode(s);
          storeValue(ret_addr, str);
          mem().setInt32(ret_addr + 8, str.length, true);
        },

        // valueLoadString(v ref, b []byte)
        "syscall/js.valueLoadString": (
          v_ref: any,
          slice_ptr: any,
          slice_len: any,
          slice_cap: any,
        ) => {
          const str = unboxValue(v_ref);
          loadSlice(slice_ptr, slice_len, slice_cap).set(str);
        },

        // func valueInstanceOf(v ref, t ref) bool
        "syscall/js.valueInstanceOf": (v_ref: any, t_ref: any) => {
          return unboxValue(v_ref) instanceof unboxValue(t_ref);
        },

        // func copyBytesToGo(dst []byte, src ref) (int, bool)
        "syscall/js.copyBytesToGo": (
          ret_addr: any,
          dest_addr: any,
          dest_len: any,
          dest_cap: any,
          src_ref: any,
        ) => {
          const num_bytes_copied_addr = ret_addr;
          const returned_status_addr = ret_addr + 4; // Address of returned boolean status variable

          const dst = loadSlice(dest_addr, dest_len);
          const src = unboxValue(src_ref);
          if (
            !(src instanceof Uint8Array || src instanceof Uint8ClampedArray)
          ) {
            mem().setUint8(returned_status_addr, 0); // Return "not ok" status
            return;
          }
          const toCopy = src.subarray(0, dst.length);
          dst.set(toCopy);
          mem().setUint32(num_bytes_copied_addr, toCopy.length, true);
          mem().setUint8(returned_status_addr, 1); // Return "ok" status
        },

        // copyBytesToJS(dst ref, src []byte) (int, bool)
        // Originally copied from upstream Go project, then modified:
        //   https://github.com/golang/go/blob/3f995c3f3b43033013013e6c7ccc93a9b1411ca9/misc/wasm/wasm_exec.js#L404-L416
        "syscall/js.copyBytesToJS": (
          ret_addr: any,
          dst_ref: any,
          src_addr: any,
          src_len: any,
          src_cap: any,
        ) => {
          const num_bytes_copied_addr = ret_addr;
          const returned_status_addr = ret_addr + 4; // Address of returned boolean status variable

          const dst = unboxValue(dst_ref);
          const src = loadSlice(src_addr, src_len);
          if (
            !(dst instanceof Uint8Array || dst instanceof Uint8ClampedArray)
          ) {
            mem().setUint8(returned_status_addr, 0); // Return "not ok" status
            return;
          }
          const toCopy = src.subarray(0, dst.length);
          dst.set(toCopy);
          mem().setUint32(num_bytes_copied_addr, toCopy.length, true);
          mem().setUint8(returned_status_addr, 1); // Return "ok" status
        },
      },
    };

    // Go 1.20 uses 'env'. Go 1.21 uses 'gojs'.
    // For compatibility, we use both as long as Go 1.20 is supported.
    this.importObject.env = this.importObject.gojs;
  }

  async run(instance: any) {
    this._inst = instance;
    this._values = [
      // JS values that Go currently has references to, indexed by reference id
      NaN,
      0,
      null,
      true,
      false,
      globalThis,
      this,
    ];
    this._goRefCounts = []; // number of references that Go has to a JS value, indexed by reference id
    this._ids = new Map(); // mapping from JS values to reference ids
    this._idPool = []; // unused ids that have been garbage collected
    this.exited = false; // whether the Go program has exited

    // while (true) {
    //     const callbackPromise = new Promise((resolve) => {
    //         this._resolveCallbackPromise = () => {
    //             if (this.exited) {
    //                 throw new Error("bad callback: Go program has already exited");
    //             }
    //             setTimeout(resolve, 0); // make sure it is asynchronous
    //         };
    //     });
    //     console.log(this._inst)
    //     this._inst.exports._start();
    //     if (this.exited) {
    //         break;
    //     }
    //     await callbackPromise;
    // }
    this._inst.exports._start();
  }

  _resume() {
    if (this.exited) {
      throw new Error("Go program has already exited");
    }
    this._inst.exports.resume();
    if (this.exited) {
      this._resolveExitPromise();
    }
  }

  _makeFuncWrapper(id: any) {
    const go = this;
    return function (this: any) {
      const event: any = { id: id, this: this, args: arguments };
      go._pendingEvent = event;
      go._resume();
      return event.result;
    };
  }
}
