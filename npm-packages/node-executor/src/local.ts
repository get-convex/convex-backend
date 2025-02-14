import { Command } from "commander";
import { invoke } from "./executor";
import { v4 as uuidv4 } from "uuid";
import { log, setDebugLogging } from "./log";
import os from "node:os";
import crypto from "crypto";
import fs from "node:fs";
import { Writable } from "node:stream";

async function main(request_str: string, debug: boolean) {
  let request;
  setDebugLogging(debug);
  try {
    request = JSON.parse(request_str);
  } catch (err: any) {
    throw new Error(
      `Failed to parse request json. Error: ${err.message.toString()}`,
    );
  }
  request.requestId = uuidv4();

  // Monkey-patch os.tmpdir to avoid filesystem write races
  const prevTempdir = os.tmpdir();
  const seed = crypto.randomBytes(20).toString("hex");
  const tempdir = `${prevTempdir}/${seed}`;
  fs.mkdirSync(tempdir);
  os.tmpdir = () => tempdir;

  const responseStream = new Writable({
    write: (chunk, _encoding, callback) => {
      log(chunk.toString());
      callback();
    },
  });
  await invoke(request, responseStream);
  responseStream.end();

  fs.rmSync(tempdir, { recursive: true });
  // Don't wait for dangling promises. This matches AWS Lambda behavior.
  process.exit(0);
}

const program = new Command();
program
  .name("node-executor")
  .description("node-executor executes a actions locally")
  .usage("command url [options]")
  .option("--debug", "print debug output", false)
  .requiredOption("--request <json>", "json request serialized as string")
  .action(async (options) => {
    await main(options.request, options.debug);
  });
program.parseAsync(process.argv);
