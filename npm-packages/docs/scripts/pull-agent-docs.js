const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

/**
 * Cross-platform script to pull agent docs and process them
 * Replaces Unix-specific find and sed commands
 */

async function processFiles(dirPath) {
  const files = fs.readdirSync(dirPath);

  for (const file of files) {
    const filePath = path.join(dirPath, file);
    const stat = fs.statSync(filePath);

    if (stat.isDirectory()) {
      await processFiles(filePath); // Recursive
    } else if (file.endsWith(".mdx")) {
      // Process .mdx files - replace relative links with GitHub links
      let content = fs.readFileSync(filePath, "utf8");

      // Replace patterns like ](../ with ](https://github.com/get-convex/agent/blob/main/
      content = content.replace(
        /\]\(\.\.\//g,
        "](https://github.com/get-convex/agent/blob/main/",
      );

      fs.writeFileSync(filePath, content);
      console.log(`Processed: ${filePath}`);
    }
  }
}

async function main() {
  console.log("Pulling agent docs from GitHub...");
  execSync("npx degit github:get-convex/agent/docs ./docs/agents --force", {
    stdio: "inherit",
  });

  console.log("Processing .mdx files...");
  await processFiles("./docs/agents");

  console.log("Running prettier...");
  execSync("npx prettier -w ./docs/agents", { stdio: "inherit" });

  console.log("✅ Agent docs pulled and processed successfully!");
}

main();
