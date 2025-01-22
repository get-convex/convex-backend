import { ClientComponentExample } from "@/app/ClientComponentExample";
import { FrameworkList } from "@/app/vote/FrameworkList";
import { Link } from "@/components/typography/link";

export default function Home() {
  return (
    <main className="container max-w-3xl flex flex-col gap-8">
      <h1 className="text-4xl font-extrabold my-8 text-center">
        Convex + Next.js SSR
      </h1>
      <p>
        This demo uses the{" "}
        <Link
          href="https://nextjs.org/docs#app-router-vs-pages-router"
          target="_blank"
        >
          Next.js App Router
        </Link>
        . Everything with an orange background was rendered by Server
        Components, and is not dynamic.
      </p>

      <ClientComponentExample />
      <div>
        <p>
          In this demo you can vote on your favorite part of this stack. Click
          on each to go to their respective voting page:
        </p>
        <FrameworkList />
      </div>
      <p>
        There{"'"}s also a non-reactive example using only Server Components and
        Server Actions: <Link href="/pureserver">Server Actions</Link>
      </p>
    </main>
  );
}
