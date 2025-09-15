import classNames from "classnames";
import Link from "next/link";
import { useEffect, useRef } from "react";

type NavItem = {
  label: string;
  href: string;
};

type NavBarProps = {
  items: NavItem[];
  activeLabel: string;
};

export function NavBar({ items, activeLabel }: NavBarProps) {
  const parentRef = useRef<HTMLDivElement | null>(null);
  const activeLinkRef = useRef<HTMLAnchorElement | null>(null);
  const activeIndicatorRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    // Reposition indicator
    if (
      !activeLinkRef.current ||
      !activeIndicatorRef.current ||
      !parentRef.current
    ) {
      return;
    }

    const rect = activeLinkRef.current.getBoundingClientRect();
    const parentRect = parentRef.current.getBoundingClientRect();

    activeIndicatorRef.current.style.width = `100%`;
    activeIndicatorRef.current.style.top = `${rect.y + rect.height + 4}px`;
    activeIndicatorRef.current.style.transform = `translateX(${rect.x - parentRect.x}px) scaleX(${parentRect.width === 0 ? 0 : rect.width / parentRect.width})`;
  }, [activeLabel]);

  return (
    <div className="relative">
      <div className="flex gap-1 truncate select-none" ref={parentRef}>
        {items.map(({ label, href }) => (
          <div className="flex flex-col" key={label}>
            <Link
              href={href}
              passHref
              ref={activeLabel === label ? activeLinkRef : undefined}
              className={classNames(
                "p-2 my-2 mx-1 text-sm",
                "text-content-primary",
                "hover:bg-background-tertiary rounded-full",
                {
                  "decoration-4 font-medium": activeLabel === label,
                },
              )}
            >
              {label}
            </Link>
          </div>
        ))}

        <div
          className="absolute mt-auto h-1 w-0 origin-top-left bg-content-secondary transition-transform will-change-transform motion-reduce:transition-none"
          ref={activeIndicatorRef}
        />
      </div>
    </div>
  );
}
