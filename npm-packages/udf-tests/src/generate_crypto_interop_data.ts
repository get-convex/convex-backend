import { generateData } from "./crypto_interop";
generateData().then((x) => process.stdout.write(x));
