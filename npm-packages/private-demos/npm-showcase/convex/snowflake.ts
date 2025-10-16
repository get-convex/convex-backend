"use node";
import { v } from "convex/values";
import { action } from "./_generated/server";
import snowflake from "snowflake-sdk";

export const doSqlQuery = action({
  args: {
    stmt: v.string(),
  },
  handler: async (_, { stmt }) => {
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
});
