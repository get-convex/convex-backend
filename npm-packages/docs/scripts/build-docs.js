const { execSync } = require("child_process");
const fs = require("fs");

async function buildDocs() {
  console.log("📁 Cleaning docs/api directory...");
  execSync("npx rimraf docs/api", { stdio: "inherit" });

  console.log("📚 Generating platform API documentation...");
  execSync("npm run generate-platform-api", { stdio: "inherit" });

  console.log("🔨 Building Docusaurus site...");
  execSync("docusaurus build", { stdio: "inherit" });

  console.log("📋 Copying redirects file...");
  execSync("npx copyfiles _redirects build/", { stdio: "inherit" });

  console.log("🔖 Writing git commit SHA...");
  const sha = execSync("git rev-parse HEAD").toString().trim();
  fs.mkdirSync("build/api", { recursive: true });
  fs.writeFileSync("build/api/version", JSON.stringify({ sha }));
}

buildDocs();
