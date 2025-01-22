import { v } from "convex/values";

export const RAND_MAX = 1000000000;
export const rand = () => Math.floor(Math.random() * RAND_MAX);
export const CACHE_BREAKER_ARGS = { cacheBreaker: v.number() };

export type MessagesTable = "messages" | "messages_with_search";
