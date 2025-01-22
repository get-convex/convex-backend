import { query } from "./_generated/server";

export default query(({ db }) => db.query("times").collect());
