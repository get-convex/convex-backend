import Link from "next/link";
import { useRouter } from "next/router";
import { ReactNode } from "react";

const ITEMS = [
  { key: "general", label: "General", href: (t: string) => `/t/${t}/settings` },
  {
    key: "members",
    label: "Members",
    href: (t: string) => `/t/${t}/settings/members`,
  },
  {
    key: "usage",
    label: "Usage",
    href: (t: string) => `/t/${t}/settings/usage`,
  },
  {
    key: "access-tokens",
    label: "Access Tokens",
    href: (t: string) => `/t/${t}/settings/access-tokens`,
  },
  {
    key: "audit-log",
    label: "Audit Log",
    href: (t: string) => `/t/${t}/settings/audit-log`,
  },
];

export function TeamSettingsLayout({
  page,
  title,
  children,
}: {
  page: "general" | "members" | "usage" | "access-tokens" | "audit-log";
  title: string;
  children: ReactNode;
}) {
  const router = useRouter();
  const teamSlug = router.query.team as string;
  return (
    <div className="flex h-full min-h-0 flex-1 overflow-hidden">
      <nav className="flex w-56 shrink-0 flex-col gap-1 border-r p-3">
        {ITEMS.map((item) => (
          <Link
            key={item.key}
            href={item.href(teamSlug)}
            className={`rounded-sm px-2 py-1.5 text-sm ${
              item.key === page
                ? "bg-background-tertiary font-medium text-content-primary"
                : "text-content-secondary hover:bg-background-tertiary"
            }`}
          >
            {item.label}
          </Link>
        ))}
      </nav>
      <main className="scrollbar flex-1 overflow-y-auto p-6">
        {/* eslint-disable-next-line no-restricted-syntax -- text-xl IS the heading style on an h1 */}
        <h1 className="mb-6 text-xl font-semibold text-content-primary">
          {title}
        </h1>
        <div className="max-w-240 space-y-4">{children}</div>
      </main>
    </div>
  );
}
