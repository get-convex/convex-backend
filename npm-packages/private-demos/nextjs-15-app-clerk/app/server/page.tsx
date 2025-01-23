import { auth } from "@clerk/nextjs/server";
import Home from "./inner";
import { preloadQuery, preloadedQueryResult } from "convex/nextjs";
import { api } from "@/convex/_generated/api";
import { ConvexClientProvider } from "../ConvexClientProvider";

export default async function ServerPage() {
  const { getToken } = await auth();
  const token = await getToken({ template: "convex" });
  console.log(token);
  const preloaded = await preloadQuery(api.tasks.user, {}, { token: token! });
  const data = preloadedQueryResult(preloaded);

  return (
    <div>
      <code>
        <pre>{JSON.stringify(data, null, 2)}</pre>
      </code>
      <ConvexClientProvider>
        <Home preloaded={preloaded} />
      </ConvexClientProvider>
    </div>
  );
}
