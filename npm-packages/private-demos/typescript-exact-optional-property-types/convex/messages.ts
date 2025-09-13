import { query } from "./_generated/server.js";

export const list = query(async (ctx) => {
  const stuff = await ctx.db.query("messages").collect();

  // (noUncheckedIndexedAccess)
  const doc = stuff[0]!;

  // exactOptionalPropertyTypes isn't any different when you access this
  const optionalField: undefined | string = doc.optionalString;
  console.log(optionalField);

  const {
    _id,
    _creationTime,
    body: _body,
    author: _author,
    objectWithOptionalString,
    ...justOptional
  } = doc;

  if ("optionalString" in justOptional) {
    const exists: string = justOptional.optionalString;
    console.log(exists);
  } else {
    const dne: undefined = justOptional.optionalString;
    // @ts-expect-error undefined is not assignable to string
    const exists: string = justOptional.optionalString;
    console.log(dne, exists);
  }

  if ("optionalString" in objectWithOptionalString) {
    // @ts-expect-error building convex with exact-optional-property-types fixes this
    const exists: string = justOptional.optionalString;
    console.log(exists);
  } else {
    // @ts-expect-error building convex with exact-optional-property-types fixes this
    const dne: undefined = justOptional.optionalString;
    // @ts-expect-error undefined is not assignable to string
    const exists: string = justOptional.optionalString;
    console.log(dne, exists);
  }
});
