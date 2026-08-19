(() => {
  const error = globalThis.__convex_last_error;
  try {
    return JSON.stringify({
      message:
        error && typeof error.message === "string"
          ? error.message
          : String(error),
      stack: error && typeof error.stack === "string" ? error.stack : null,
    });
  } finally {
    delete globalThis.__convex_last_error;
  }
})();
