import { ConvexHttpClient } from "convex/browser";
import { NextResponse } from "next/server";

const convex = new ConvexHttpClient(process.env.NEXT_PUBLIC_CONVEX_URL);

export async function GET(request: Request) {
  const clicks = await convex.query(api.counter.get, { counterName: "clicks" });
  return NextResponse.json({ clicks });
}
