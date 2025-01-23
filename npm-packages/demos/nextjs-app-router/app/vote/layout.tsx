import { Link } from "@/components/typography/link";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <main className="container max-w-3xl flex flex-col gap-8 py-4">
      <Link href="/">Back to home</Link>
      {children}
    </main>
  );
}
