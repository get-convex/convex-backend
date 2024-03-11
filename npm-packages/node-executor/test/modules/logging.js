export const logSome = {
  isAction: true,
  invokeAction: async (_requestId, _args) => {
    for (let i = 0; i < 40; i += 1) {
      console.log("Hello");
    }
    return "";
  },
};

export const logTooManyLines = {
  isAction: true,
  invokeAction: async (_requestId, _args) => {
    for (let i = 0; i < 260; i += 1) {
      console.log("Hello");
    }
    return "";
  },
};

export const logOverTotalLength = {
  isAction: true,
  invokeAction: async (_requestId, _args) => {
    for (let i = 0; i < 40; i += 1) {
      console.log(`Hello `.repeat(10_000));
    }
    return "";
  },
};
