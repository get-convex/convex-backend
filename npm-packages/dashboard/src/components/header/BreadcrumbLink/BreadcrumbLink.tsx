import classNames from "classnames";
import Link from "next/link";
import { ReactNode } from "react";

type BreadcrumbLinkProps = {
  children: ReactNode;
  href: string;
  className?: string;
};

export function BreadcrumbLink({
  children,
  href,
  className,
}: BreadcrumbLinkProps) {
  return (
    <Link
      href={href}
      passHref
      className={classNames("py-2 text-sm text-content-primary", className)}
    >
      {children}
    </Link>
  );
}
