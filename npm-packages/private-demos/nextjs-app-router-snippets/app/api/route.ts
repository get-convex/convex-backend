import { NextResponse } from "next/server";
// Hack for TypeScript before 5.2
const Response = NextResponse;

// @snippet start example
import { api } from "@/convex/_generated/api";
import { fetchMutation } from "convex/nextjs";

export async function POST(request: Request) {
  const args = await request.json();
  await fetchMutation(api.tasks.create, { text: args.text });
  return Response.json({ success: true });
}
// @snippet end example
