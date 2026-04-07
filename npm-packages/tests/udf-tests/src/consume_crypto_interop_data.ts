import { consumeData } from "./crypto_interop";
import { text } from "node:stream/consumers";

text(process.stdin)
  .then(consumeData)
  .then(() => console.log("ok"));
