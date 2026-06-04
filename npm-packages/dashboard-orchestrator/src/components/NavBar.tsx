import Link from "next/link";
import { useRouter } from "next/router";

export type NavBarItem = {
  label: string;
  href: string;
};

export function NavBar({ items }: { items: NavBarItem[] }) {
  const router = useRouter();
  return (
    <nav className="flex h-full items-center" aria-label="Section navigation">
      {items.map((item) => {
        const active =
          router.asPath === item.href ||
          router.asPath.startsWith(`${item.href}/`) ||
          (item.href.endsWith("/settings") &&
            router.asPath.includes("/settings"));
        return (
          <Link
            key={item.href}
            href={item.href}
            className={`relative flex h-full items-center px-3 text-sm transition-colors ${
              active
                ? "font-medium text-content-primary"
                : "text-content-secondary hover:text-content-primary"
            }`}
          >
            {item.label}
            {active && (
              <span
                aria-hidden
                className="absolute inset-x-1.5 bottom-0 h-0.5 bg-content-primary"
              />
            )}
          </Link>
        );
      })}
    </nav>
  );
}
