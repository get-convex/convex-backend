import { Id } from "./_generated/dataModel";
import { mutation, query, action } from "./_generated/server";

export const addOneInt = query(async (_, { x }: { x: bigint }) => {
  return x + 1n;
});

export const readTime = query(async () => {
  return new Date().toString();
});

export const readTimeMs = query(async () => {
  return Date.now();
});

export const createTimeMs = query(async (_, { args }: { args: [any] }) => {
  // Date expects 0-7 args, not n args, so pretend it's 1 arg.
  return +new Date(...args);
});

export const getObject = query(async ({ db }, { id }: { id: Id<any> }) => {
  return db.get(id);
});

export const insertObject = mutation(async ({ db }, obj) => {
  const id = await db.insert("objects", obj);
  return await db.get(id);
});

// Regression test, ensuring that `db.patch` updates the table summary.
// If it doesn't, the db.delete will try to delete an object larger than
// the one that was inserted, and the table summary's size will go negative.
export const insertModifyDeleteObject = mutation(async ({ db }) => {
  const obj: any = { field: "a" };
  const id = await db.insert("objects", obj);
  obj.field = "ab";
  await db.patch(id, obj);
  await db.delete(id);
});

export const insertTwoObjects = mutation(
  async ({ db }, { obj1, obj2 }: { obj1: any; obj2: any }) => {
    const id1 = await db.insert("objects", obj1);
    const id2 = await db.insert("objects", obj2);
    return [await db.get(id1), await db.get(id2)];
  },
);

export const patchObject = mutation(
  async ({ db }, { id, obj }: { id: Id<any>; obj: any }) => {
    await db.patch(id, obj);
    return await db.get(id);
  },
);

export const deleteObjectField = mutation(
  async ({ db }, { id, fieldName }: { id: Id<any>; fieldName: string }) => {
    const patchValue: any = {};
    patchValue[fieldName] = undefined;
    await db.patch(id, patchValue);
    return await db.get(id);
  },
);

export const replaceObject = mutation(
  async ({ db }, { id, obj }: { id: Id<any>; obj: any }) => {
    await db.replace(id, obj);
    return await db.get(id);
  },
);

// Add and deletes the same object in the single mutation.
export const insertAndDeleteObject = mutation(async ({ db }, obj: any) => {
  const id = await db.insert("objects", obj);
  obj = await db.get(id);
  await db.delete(id);
  return obj;
});

export const listAllObjects = query(async ({ db }) => {
  return await db.query("objects").collect();
});

export const explicitDbTableApi = mutation(async ({ db }) => {
  const id = await db.insert("objects", {
    name: "test",
  });

  const obj = await db.get("objects", id);
  if (!obj || obj.name !== "test") {
    throw new Error();
  }

  await db.patch("objects", id, {
    name: "test2",
  });

  const obj2 = await db.get("objects", id);
  if (!obj2 || obj2.name !== "test2") {
    throw new Error();
  }

  await db.replace("objects", id, {
    name: "test3",
  });

  const obj3 = await db.get("objects", id);
  if (!obj3 || obj3.name !== "test3") {
    throw new Error();
  }

  await db.delete("objects", id);

  const obj4 = await db.get("objects", id);
  if (obj4 !== null) {
    throw new Error();
  }
});

export const doNothing = query(async () => "hi");

export const count = query(async ({ db }) => {
  return await db.query("objects").count();
});

export const insertAndCount = mutation(async ({ db }, obj) => {
  await db.insert("objects", obj);
  return await db.query("objects").count();
});

export const deleteAndCount = mutation(
  async ({ db }, { id }: { id: Id<any> }) => {
    await db.delete(id);
    return await db.query("objects").count();
  },
);

export const simpleMutation = mutation(async () => {
  return 2;
});

export const simpleAction = action(async () => {
  return 2;
});
