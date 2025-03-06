import { action } from "./_generated/server";

export default action({
  handler: () => {
    let s = "";
    for (let i = 0; i < 512; i += 1) {
      s += "a";
    }

    for (let i = 0; i < 256; i += 1) {
      console.log(s);
    }
  },
});
