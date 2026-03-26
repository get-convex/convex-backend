// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import { chalkStderr } from "chalk";
import { logMessage } from "../../../bundler/log.js";
import { downloadGuidelines } from "../versionApi.js";
import { hashSha256 } from "../utils/hash.js";
import { guidelinesPathForConvexDir } from "./paths.js";
import { readFileSafe } from "./utils.js";
import { type AiFilesConfig } from "./config.js";

export async function hasGuidelinesInstalled(
  convexDir: string,
): Promise<boolean> {
  return (await readFileSafe(guidelinesPathForConvexDir(convexDir))) !== null;
}

/**
 * Download and write the guidelines file.
 * Guidelines live in `_generated/` so local edits are not expected and are
 * not preserved.
 */
export async function installGuidelinesFile({
  convexDir,
  config,
}: {
  convexDir: string;
  config: AiFilesConfig;
}): Promise<void> {
  const guidelines = await downloadGuidelines();
  if (guidelines === null) {
    logMessage(
      chalkStderr.yellow(
        "Could not download Convex AI guidelines right now. You can retry with: npx convex ai-files install",
      ),
    );
    return;
  }

  await fs.writeFile(guidelinesPathForConvexDir(convexDir), guidelines, "utf8");
  config.guidelinesHash = hashSha256(guidelines);
}
