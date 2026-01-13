export const throwString = {
  isAction: true,
  invokeAction: async () => {
    throw "hello world";
  },
};
