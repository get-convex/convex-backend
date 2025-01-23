import { mutation } from "./_generated/server";

export default mutation(({ db }) => db.insert("times", { time: Date.now() }));
