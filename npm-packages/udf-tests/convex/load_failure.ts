if (process.env.FAIL_MODULE_LOAD) {
  throw new Error("boom");
}

export default null;
