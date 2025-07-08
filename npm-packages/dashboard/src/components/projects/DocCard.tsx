import Link from "next/link";
import { logEvent } from "convex-analytics";
import classNames from "classnames";
import React from "react";

export function DocCard({
  href,
  icon,
  title,
  description,
  bgColor,
}: {
  href: string;
  icon: React.FC<any>;
  title: string;
  description: string;
  bgColor?: string;
}) {
  const Icon = icon;
  return (
    <li className="grow">
      <div className="relative -m-2 flex items-center space-x-4 rounded-xl p-2 focus-within:ring-2 hover:bg-background-tertiary dark:ring-neutral-6">
        <div
          className={classNames(
            bgColor,
            "flex h-16 w-16 flex-shrink-0 items-center justify-center rounded-lg",
          )}
        >
          <Icon className="h-6 w-6 text-white" aria-hidden="true" />
        </div>
        <div>
          <p className="text-sm font-medium text-content-primary">
            <Link
              passHref
              href={href}
              target="_blank"
              onClick={() => logEvent("viewed doc", { title })}
              className="focus:outline-hidden"
            >
              <span className="absolute inset-0" aria-hidden="true" />
              <span>{title}</span>
              <span aria-hidden="true"> &rarr;</span>
            </Link>
          </p>
          <p className="mt-1 text-sm text-content-secondary">{description}</p>
        </div>
      </div>
    </li>
  );
}
