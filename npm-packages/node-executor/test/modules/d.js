const global_scope_var = process.env.GLOBAL_SCOPE_VAR;

export default {
  isAction: true,

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  invokeAction: async (requestId, args) => {
    return global_scope_var;
  },
};
