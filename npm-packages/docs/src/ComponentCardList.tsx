import React from "react";
import Link from "@docusaurus/Link";
import { ReactNode } from "@mdx-js/react/lib";
import Heading from "@theme/Heading";

type Item = {
  href: string;
  label: string;
  description?: string;
  icon?: ReactNode;
  invertIcon?: true;
};

export function ComponentCardList(props: { items: Item[] }) {
  const { items } = props;
  return (
    <div className="component-cards">
      {items.map((item, index) => (
        <CardLink key={index} item={item} />
      ))}
    </div>
  );
}

function CardLink({ className, item }: { className?: string; item: Item }) {
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
        <p className="text--truncate" title={item.description}>
          {item.description}
        </p>
      </div>
    </Link>
  );
}
