import { RegisteredQuery } from "convex/server";

declare const addOneInt: RegisteredQuery<"public", { x: bigint }, bigint>;
