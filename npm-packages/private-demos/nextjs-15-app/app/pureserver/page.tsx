import { api } from "@/convex/_generated/api";
import { fetchMutation, fetchQuery } from "convex/nextjs";
import { revalidatePath } from "next/cache";

export default async function PureServerPage() {
  const counterName = "server-actions";
  const value = await fetchQuery(api.counter.get, {
    counterName,
  });
  async function incrementCounter() {
    "use server";

    await fetchMutation(api.counter.increment, { counterName });
    revalidatePath("/pure-server");
  }

  return (
    <>
      <h1 className="text-4xl font-extrabold my-8 text-center">
        Vote for Server Actions
      </h1>
      <p>This page uses only Server Components and Server Actions.</p>
      <p>The value of the counter will only update when you vote.</p>
      <p>
        The value of the counter is : <span className="font-bold">{value}</span>
      </p>
      <form action={incrementCounter}>
        <button type="submit">Vote for Server Actions</button>
      </form>
      <p>
        Open this page in another tab to see the Client Component update in real
        time:
      </p>
    </>
  );
}
