import { api } from "@/convex/_generated/api";
import { fetchMutation } from "convex/nextjs";

export async function POST(request) {
  const args = await request.json();
  await fetchMutation(api.tasks.create, { text: args.text });
  return Response.json({ success: true });
}
