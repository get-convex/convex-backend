// The async context is stored as an array: [ALS1, value1, ALS2, value2, ...]
// We use undefined to represent an empty context
type AsyncContextData = ReadonlyArray<unknown> | undefined;

type ConvexAsyncContextBridge = {
  getContinuationPreservedEmbedderData?: () => unknown;
  setContinuationPreservedEmbedderData?: (value: unknown) => void;
};

// Fallback used only when running outside the Convex backend runtime.
let fallbackAsyncContext: AsyncContextData = undefined;

function getConvexAsyncContextBridge(): ConvexAsyncContextBridge | undefined {
  return (globalThis as { Convex?: ConvexAsyncContextBridge }).Convex;
}

function normalizeAsyncContext(value: unknown): AsyncContextData {
  if (value === undefined) {
    return undefined;
  }
  if (Array.isArray(value)) {
    return value;
  }
  return undefined;
}

function getAsyncContext(): AsyncContextData {
  const convex = getConvexAsyncContextBridge();
  if (convex?.getContinuationPreservedEmbedderData) {
    return normalizeAsyncContext(convex.getContinuationPreservedEmbedderData());
  }
  return fallbackAsyncContext;
}

function setAsyncContext(context: AsyncContextData): void {
  const convex = getConvexAsyncContextBridge();
  if (convex?.setContinuationPreservedEmbedderData) {
    convex.setContinuationPreservedEmbedderData(context);
    return;
  }
  fallbackAsyncContext = context;
}

export class AsyncLocalStorage<T = unknown> {
  #disabled = false;

  static bind<F extends (...args: unknown[]) => unknown>(
    fn: F,
    ...args: unknown[]
  ): (...callArgs: unknown[]) => ReturnType<F> {
    if (typeof fn !== "function") {
      throw new TypeError("fn must be a function");
    }
    const boundSnapshot = AsyncLocalStorage.snapshot();
    return (...callArgs: unknown[]) =>
      boundSnapshot(fn, ...args, ...callArgs) as ReturnType<F>;
  }

  static snapshot(): <R, TArgs extends unknown[]>(
    fn: (...args: TArgs) => R,
    ...args: TArgs
  ) => R {
    const context = getAsyncContext();
    return <R, TArgs extends unknown[]>(
      fn: (...args: TArgs) => R,
      ...args: TArgs
    ): R => {
      const prev = getAsyncContext();
      setAsyncContext(context);
      try {
        return fn(...args);
      } finally {
        setAsyncContext(prev);
      }
    };
  }

  enterWith(store: T): void {
    this.#disabled = false;

    const context = getAsyncContext();
    if (!context) {
      setAsyncContext([this, store]);
      return;
    }

    const { length } = context;
    for (let i = 0; i < length; i += 2) {
      if (context[i] === this) {
        const clone = context.slice();
        clone[i + 1] = store;
        setAsyncContext(clone);
        return;
      }
    }
    setAsyncContext(context.concat(this, store));
  }

  exit<R, TArgs extends unknown[]>(
    callback: (...args: TArgs) => R,
    ...args: TArgs
  ): R {
    return this.run(undefined as T, callback, ...args);
  }

  run<R, TArgs extends unknown[]>(
    store: T,
    callback: (...args: TArgs) => R,
    ...args: TArgs
  ): R {
    const wasDisabled = this.#disabled;
    this.#disabled = false;

    let context = getAsyncContext() as unknown[] | undefined;
    let hasPrevious = false;
    let previousValue: unknown;
    let index = -1;
    const contextWasEmpty = !context;

    if (contextWasEmpty) {
      setAsyncContext((context = [this, store]));
      index = 0;
    } else {
      context = context!.slice();
      index = context.indexOf(this);

      if (index > -1) {
        hasPrevious = true;
        previousValue = context[index + 1];
        context[index + 1] = store;
      } else {
        index = context.length;
        context.push(this, store);
      }
      setAsyncContext(context);
    }

    try {
      return callback(...args);
    } finally {
      if (!wasDisabled) {
        let context2 = getAsyncContext() as unknown[] | undefined;

        if (context2 === context && contextWasEmpty) {
          setAsyncContext(undefined);
        } else if (context2) {
          context2 = context2.slice();

          if (hasPrevious) {
            context2[index + 1] = previousValue;
            setAsyncContext(context2);
          } else {
            context2.splice(index, 2);
            setAsyncContext(context2.length ? context2 : undefined);
          }
        }
      }
    }
  }

  disable(): void {
    if (this.#disabled) return;
    this.#disabled = true;

    const context = getAsyncContext() as unknown[] | undefined;
    if (context) {
      const { length } = context;
      for (let i = 0; i < length; i += 2) {
        if (context[i] === this) {
          const newContext = context.slice();
          newContext.splice(i, 2);
          setAsyncContext(newContext.length ? newContext : undefined);
          break;
        }
      }
    }
  }

  getStore(): T | undefined {
    if (this.#disabled) return undefined;

    const context = getAsyncContext();
    if (!context) return undefined;

    const { length } = context;
    for (let i = 0; i < length; i += 2) {
      if (context[i] === this) {
        return context[i + 1] as T;
      }
    }

    return undefined;
  }
}

export class AsyncResource {
  readonly type: string;
  #snapshot: AsyncContextData;

  constructor(type: string, _options?: { triggerAsyncId?: number } | number) {
    if (typeof type !== "string") {
      throw new TypeError("type must be a string");
    }

    this.type = type;
    this.#snapshot = getAsyncContext();
  }

  runInAsyncScope<R, This, TArgs extends unknown[]>(
    fn: (this: This, ...args: TArgs) => R,
    thisArg?: This,
    ...args: TArgs
  ): R {
    const prev = getAsyncContext();
    setAsyncContext(this.#snapshot);
    try {
      return fn.apply(thisArg as This, args);
    } finally {
      setAsyncContext(prev);
    }
  }

  bind<F extends (...args: unknown[]) => unknown>(fn: F, thisArg?: unknown): F {
    if (typeof fn !== "function") {
      throw new TypeError("fn must be a function");
    }
    const snapshot = this.#snapshot;
    const bound = function (this: unknown, ...args: unknown[]) {
      const prev = getAsyncContext();
      setAsyncContext(snapshot);
      try {
        return fn.apply(thisArg ?? this, args);
      } finally {
        setAsyncContext(prev);
      }
    };
    return bound as F;
  }

  static bind<F extends (...args: unknown[]) => unknown>(
    fn: F,
    type?: string,
    thisArg?: unknown,
  ): F {
    type = type || fn.name || "bound-anonymous-fn";
    return new AsyncResource(type).bind(fn, thisArg);
  }

  emitBefore(): boolean {
    return true;
  }

  emitAfter(): boolean {
    return true;
  }

  asyncId(): number {
    return 0;
  }

  triggerAsyncId(): number {
    return 0;
  }

  emitDestroy(): void {
    return;
  }
}

export function executionAsyncId(): number {
  return 0;
}

export function triggerAsyncId(): number {
  return 0;
}

export function executionAsyncResource(): object {
  return {};
}

export function createHook(_hooks: {
  init?: (
    asyncId: number,
    type: string,
    triggerAsyncId: number,
    resource: object,
  ) => void;
  before?: (asyncId: number) => void;
  after?: (asyncId: number) => void;
  destroy?: (asyncId: number) => void;
  promiseResolve?: (asyncId: number) => void;
}): { enable: () => object; disable: () => object } {
  return {
    enable() {
      return this;
    },
    disable() {
      return this;
    },
  };
}

export const asyncWrapProviders = {
  NONE: 0,
  DIRHANDLE: 1,
  DNSCHANNEL: 2,
  ELDHISTOGRAM: 3,
  FILEHANDLE: 4,
  FILEHANDLECLOSEREQ: 5,
  FIXEDSIZEBLOBCOPY: 6,
  FSEVENTWRAP: 7,
  FSREQCALLBACK: 8,
  FSREQPROMISE: 9,
  GETADDRINFOREQWRAP: 10,
  GETNAMEINFOREQWRAP: 11,
  HEAPSNAPSHOT: 12,
  HTTP2SESSION: 13,
  HTTP2STREAM: 14,
  HTTP2PING: 15,
  HTTP2SETTINGS: 16,
  HTTPINCOMINGMESSAGE: 17,
  HTTPCLIENTREQUEST: 18,
  JSSTREAM: 19,
  JSUDPWRAP: 20,
  MESSAGEPORT: 21,
  PIPECONNECTWRAP: 22,
  PIPESERVERWRAP: 23,
  PIPEWRAP: 24,
  PROCESSWRAP: 25,
  PROMISE: 26,
  QUERYWRAP: 27,
  SHUTDOWNWRAP: 28,
  SIGNALWRAP: 29,
  STATWATCHER: 30,
  STREAMPIPE: 31,
  TCPCONNECTWRAP: 32,
  TCPSERVERWRAP: 33,
  TCPWRAP: 34,
  TTYWRAP: 35,
  UDPSENDWRAP: 36,
  UDPWRAP: 37,
  SIGINTWATCHDOG: 38,
  WORKER: 39,
  WORKERHEAPSNAPSHOT: 40,
  WRITEWRAP: 41,
  ZLIB: 42,
  CHECKPRIMEREQUEST: 43,
  PBKDF2REQUEST: 44,
  KEYPAIRGENREQUEST: 45,
  KEYGENREQUEST: 46,
  KEYEXPORTREQUEST: 47,
  CIPHERREQUEST: 48,
  DERIVEBITSREQUEST: 49,
  HASHREQUEST: 50,
  RANDOMBYTESREQUEST: 51,
  RANDOMPRIMEREQUEST: 52,
  SCRYPTREQUEST: 53,
  SIGNREQUEST: 54,
  TLSWRAP: 55,
  VERIFYREQUEST: 56,
  INSPECTORJSBINDING: 57,
};

export function setupAsyncHooks(global: typeof globalThis): void {
  const asyncHooksModule = {
    AsyncLocalStorage,
    AsyncResource,
    createHook,
    executionAsyncId,
    triggerAsyncId,
    executionAsyncResource,
    asyncWrapProviders,
    getAsyncContext,
    setAsyncContext,
  };

  (global as Record<string, unknown>).AsyncLocalStorage = AsyncLocalStorage;
  (global as Record<string, unknown>).AsyncResource = AsyncResource;

  (global as Record<string, unknown>).__async_hooks__ = asyncHooksModule;
}
