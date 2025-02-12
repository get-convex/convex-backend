import fs from "fs";
export const deploymentUrl =
  process.env.DEPLOYMENT_URL || "http://127.0.0.1:8000";
export const siteUrl = process.env.SITE_URL || "http://127.0.0.1:8001";

export const adminKey =
  process.env.ADMIN_KEY ||
  fs.readFileSync("../../crates/keybroker/dev/admin_key.txt", "utf8");
