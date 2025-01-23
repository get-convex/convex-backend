import { mutation } from "./_generated/server";
export default mutation(async (_ctx, _args: { a?: number } = { a: 1 }) => {
  console.log("Clearing presence...\nany minute now...\nany minute...");
  console.log("...cleared presence!");
  // return a large object to see how it renders in logs
  return {
    a: 123,
    b: {
      c: 123123,
      d: "asdf asdf asd fasdf adsf asdf ads fads f adsf dsaf ads fads f dasf asdf asd fasd f asdf sadf asd fdas",
    },
  };
});
