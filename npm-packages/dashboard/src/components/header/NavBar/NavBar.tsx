import classNames from "classnames";
import Link from "next/link";
import { useState } from "react";

type NavItem = {
  label: string;
  href: string;
};

type NavBarProps = {
  items: NavItem[];
  activeLabel: string;
};

export function NavBar({ items, activeLabel }: NavBarProps) {
  const [ref, setRef] = useState<HTMLAnchorElement | null>(null);
  const rect = ref?.getBoundingClientRect();

  return (
    <div>
      <div className="flex gap-1 truncate select-none">
        {items.map(({ label, href }) => (
          <div className="flex flex-col" key={label}>
            <Link
              href={href}
              passHref
              ref={(r) => (activeLabel === label ? setRef(r) : undefined)}
              className={classNames(
                "p-2 my-2 mx-1 text-sm",
                "text-content-primary",
                "hover:bg-background-tertiary rounded-full",
                {
                  "underline-offset-[1rem] decoration-4 font-medium underline sm:no-underline":
                    activeLabel === label,
                },
              )}
            >
              {label}
            </Link>
          </div>
        ))}
        {rect && (
          <div
            className="absolute mt-auto hidden h-1 w-full bg-content-secondary sm:block"
            style={{
              width: rect.width,
              top: rect.y + rect.height + 4,
              left: rect.x,
              transition: "left 150ms",
            }}
          />
        )}
      </div>
    </div>
  );
}
