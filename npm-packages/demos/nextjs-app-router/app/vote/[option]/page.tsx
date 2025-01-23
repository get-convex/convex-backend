import { FrameworkList, getOptionName } from "@/app/vote/FrameworkList";
import { Counter } from "@/app/vote/[option]/Counter";
import { api } from "@/convex/_generated/api";
import { preloadQuery, preloadedQueryResult } from "convex/nextjs";

export default async function VoteOptionPage({
  params: { option },
}: {
  params: { option: string };
}) {
  const counterName = option;
  const preloadedCounter = await preloadQuery(api.counter.get, {
    counterName,
  });
  const value = preloadedQueryResult(preloadedCounter);

  return (
    <>
      <h1 className="text-4xl font-extrabold my-8 text-center">
        Vote for {getOptionName(option)}
      </h1>
      <p>This is a Server Component content.</p>
      <p>
        The value of the counter when the page was loaded was:{" "}
        <span className="font-bold">{value}</span>
      </p>
      <Counter counterName={counterName} preloadedCounter={preloadedCounter} />
      <p>
        Open this page in another tab to see the Client Component update in real
        time.
      </p>
      <div>
        <p>Go to another page:</p>
        <FrameworkList current={option} />
      </div>
    </>
  );
}
