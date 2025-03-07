import { Command } from "commander";
import { invoke } from "./executor";
import { v4 as uuidv4 } from "uuid";
import { log, setDebugLogging } from "./log";
import os from "node:os";
import crypto from "crypto";
import fs from "node:fs";
import express, { Request, Response } from "express";

const DEFAULT_PORT = 3002;

async function setupTempDir() {
  // Monkey-patch os.tmpdir to avoid filesystem write races
  const prevTempdir = os.tmpdir();
  const seed = crypto.randomBytes(20).toString("hex");
  const tempdir = `${prevTempdir}/${seed}`;
  fs.mkdirSync(tempdir);
  os.tmpdir = () => tempdir;
  return tempdir;
}

async function startServer(port: number, debug: boolean) {
  setDebugLogging(debug);
  const app = express();
  app.use(express.json());

  const tempdir = await setupTempDir();

  // Add health check endpoint
  app.get("/health", (_req: Request, res: Response) => {
    res.json({ status: "ok" });
  });

  app.post("/invoke", async (req: Request, res: Response) => {
    try {
      const request = req.body;
      request.requestId = uuidv4();

      // Set up streaming response
      res.setHeader("Content-Type", "application/x-ndjson");
      res.setHeader("Transfer-Encoding", "chunked");

      await invoke(request, res);
    } catch (err: any) {
      // If we haven't written anything yet, send an error response
      if (!res.headersSent) {
        res.status(500).json({
          type: "error",
          message: err.message || "Internal server error",
        });
      } else {
        // If we've already started streaming, try to write an error line
        res.write(
          JSON.stringify({
            type: "error",
            message: err.message || "Internal server error",
          }) + "\n",
        );
      }
    } finally {
      res.end();
    }
  });

  const server = app.listen(port, () => {
    log(`Node executor server listening on port ${port}`);
  });

  // Handle cleanup on process exit
  const cleanup = () => {
    server.close();
    fs.rmSync(tempdir, { recursive: true });
    process.exit(0);
  };

  process.on("SIGINT", cleanup);
  process.on("SIGTERM", cleanup);
}

const program = new Command();
program
  .name("node-executor")
  .description(
    "node-executor runs an HTTP server for executing actions locally",
  )
  .usage("command [options]")
  .option("--debug", "print debug output", false)
  .option("--port <number>", "port to listen on", DEFAULT_PORT.toString())
  .action(async (options) => {
    const port = parseInt(options.port, 10);
    await startServer(port, options.debug);
  });

program.parseAsync(process.argv);
