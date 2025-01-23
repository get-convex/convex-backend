import React from "react";
import Link from "@docusaurus/Link";
// eslint-disable-next-line
// @ts-expect-error ts(2307) Internal module doesn't have types
import { useDocById } from "@docusaurus/theme-common/internal";
import { ReactNode } from "@mdx-js/react/lib";
import Heading from "@theme/Heading";

type Item = {
  docId: string;
  href: string;
  label: string;
  icon?: ReactNode;
  invertIcon?: true;
};

export function DocCardList(props: { items: Item[] }) {
  const { items } = props;
  return (
    <div className="cards">
      {items.map((item, index) => (
        <CardLink key={index} item={item} />
      ))}
    </div>
  );
}

export function CardLink({
  className,
  item,
}: {
  className?: string;
  item: Item;
}) {
  const doc = useDocById(item.docId ?? undefined);
  const icon = item.icon;
  return (
    <Link
      href={item.href}
      className={
        "card" +
        (item.invertIcon ? " convex-invert-icon" : "") +
        " " +
        (className ?? "")
      }
    >
      {icon}
      <div>
        <Heading as="h2" className="text--truncate" title={item.label}>
          {item.label}
        </Heading>
        <p className="text--truncate" title={doc?.description}>
          {doc?.description}
        </p>
      </div>
    </Link>
  );
}
