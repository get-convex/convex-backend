const http = require("http");
const chalk = require("chalk");
const readline = require("readline");
const url = require("url");
const { spawnSync } = require("child_process");

require("dotenv").config({ path: ".env.development" });

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
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  rl.on("SIGINT", () => {
    rl.close();
    process.exit(1);
  });

  rl.question(
    chalk.yellow(
      "Looks like you don't have BigBrain running, start it now? [Y/n]: ",
    ),
    (answer) => {
      rl.close();
      if (
        answer.toLowerCase() === "yes" ||
        answer.toLowerCase() === "y" ||
        answer === ""
      ) {
        console.error(
          chalk.green(
            "Starting BigBrain now via `just run-big-brain`, " +
              "repeat your original command in a new terminal",
          ),
        );
        spawnSync("just run-big-brain", { shell: true, stdio: "inherit" });
        console.log(chalk.green("Quiting BigBrain, all is good!"));
        process.exit(1);
      } else {
        console.error(
          chalk.yellow(
            `Make sure to run ${chalk.white.bold(
              "just run-big-brain",
            )} in another terminal!`,
          ),
        );
      }
    },
  );
}
