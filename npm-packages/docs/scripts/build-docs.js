const { execSync } = require("child_process");

async function buildDocs() {
  console.log("📁 Cleaning docs/api directory...");
  execSync("npx rimraf docs/api", { stdio: "inherit" });

  console.log("📚 Generating platform API documentation...");
  execSync("npm run generate-platform-api", { stdio: "inherit" });

  console.log("🔨 Building Docusaurus site...");
  execSync("docusaurus build", { stdio: "inherit" });

  console.log("📋 Copying redirects file...");
  execSync("npx copyfiles _redirects build/", { stdio: "inherit" });
}

buildDocs();
