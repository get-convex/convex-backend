"use node";
// waiting for a better quickfix to enforce this
/* eslint-disable @convex-dev/no-old-registered-function-syntax */
import { action } from "./_generated/server";
import snowflake from "snowflake-sdk";

export const doSqlQuery = action(
  async (_, { stmt }: { stmt: string }): Promise<string> => {
    const connection = snowflake.createConnection({
      account: process.env.SNOWFLAKE_ACCOUNT!,
      username: process.env.SNOWFLAKE_USERNAME,
      password: process.env.SNOWFLAKE_PASSWORD,
      authenticator: "SNOWFLAKE",
    });

    const connPromise = new Promise((resolve, reject) => {
      connection.connect(async function (err, conn) {
        if (err) {
          throw err;
        } else {
          console.log("Successfully connected as id: " + conn.getId());
          if (!(await conn.isValidAsync())) {
            reject("connection invalid");
          }

          conn.execute({
            sqlText: stmt,
            complete: function (err, _, rows) {
              if (err) {
                throw err;
              } else {
                resolve(rows || []);
              }
            },
          });
        }
      });
    });

    return JSON.stringify(await connPromise);
  },
);
