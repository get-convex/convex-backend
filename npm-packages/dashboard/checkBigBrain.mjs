import http from "http";
import chalk from "chalk";
import url from "url";
import dotenv from "dotenv";

dotenv.config({ path: ".env.development" });
dotenv.config({ path: ".env.local" });

if (!process.env.WORKOS_API_SECRET) {
  console.error(
    chalk.red(
      "WORKOS_API_SECRET environment variable is not set.  For local development, you can set it by running `npm run pullEnv`",
    ),
  );
  process.exit(1);
}

const parsedUrl = url.parse(process.env.NEXT_PUBLIC_BIG_BRAIN_URL);
http
  .request(
    {
      hostname: parsedUrl.hostname,
      port: parsedUrl.port,
      path: "/version",
      method: "GET",
    },
    (res) => {
      if (res.statusCode === 200) {
        process.exit(0);
      } else {
        onFailure();
      }
    },
  )
  .on("error", onFailure)
  .end();

function onFailure() {
  console.log(
    chalk.yellow(
      `Looks like you don't have BigBrain running. Make sure to run ${chalk.bold(chalk.yellowBright("just run-big-brain"))} in another terminal.`,
    ),
  );
}
