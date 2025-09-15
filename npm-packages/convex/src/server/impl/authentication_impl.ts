import { Auth } from "../authentication.js";
import { performAsyncSyscall } from "./syscall.js";

export function setupAuth(requestId: string): Auth {
  return {
    getUserIdentity: async () => {
      return await performAsyncSyscall("1.0/getUserIdentity", {
        requestId,
      });
    },
    getUserIdentityDebug: async () => {
      return await performAsyncSyscall("1.0/getUserIdentityDebug", {
        requestId,
      });
    },
    getUserIdentityInsecure: async () => {
      return await performAsyncSyscall("1.0/getUserIdentityInsecure", {
        requestId,
      });
    },
  };
}
