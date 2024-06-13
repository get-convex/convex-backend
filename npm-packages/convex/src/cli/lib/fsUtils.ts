import { Context, logOutput } from "../../bundler/context.js";
import path from "path";

export function recursivelyDelete(
  ctx: Context,
  deletePath: string,
  opts?: { force?: boolean; dryRun?: boolean },
) {
  const dryRun = !!opts?.dryRun;
  let st;
  try {
    st = ctx.fs.stat(deletePath);
  } catch (err: any) {
    if (err.code === "ENOENT" && opts?.force) {
      return;
    }
    // eslint-disable-next-line no-restricted-syntax
    throw err;
  }
  if (st.isDirectory()) {
    for (const entry of ctx.fs.listDir(deletePath)) {
      recursivelyDelete(ctx, path.join(deletePath, entry.name), opts);
    }
    if (dryRun) {
      logOutput(ctx, `Command would delete directory: ${deletePath}`);
      return;
    }
    try {
      ctx.fs.rmdir(deletePath);
    } catch (err: any) {
      if (err.code !== "ENOENT") {
        // eslint-disable-next-line no-restricted-syntax
        throw err;
      }
    }
  } else {
    if (dryRun) {
      logOutput(ctx, `Command would delete file: ${deletePath}`);
      return;
    }
    try {
      ctx.fs.unlink(deletePath);
    } catch (err: any) {
      if (err.code !== "ENOENT") {
        // eslint-disable-next-line no-restricted-syntax
        throw err;
      }
    }
  }
}
