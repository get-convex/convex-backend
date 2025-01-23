import { LightningBoltIcon, StackIcon } from "@radix-ui/react-icons";
import React from "react";
import { DocCard } from "./DocCard";

const docs = [
  {
    href: "https://docs.convex.dev",
    title: "Docs",
    description: "Learn more about Convex",
    icon: LightningBoltIcon,
    bgColor: "bg-blue-500",
  },
  {
    href: "https://stack.convex.dev",
    title: "Stack",
    description: "Get tips and tricks on using Convex",
    icon: StackIcon,
    bgColor: "bg-util-brand-purple",
  },
];
export function DocsGrid() {
  return (
    <div className="my-12 animate-fadeInFromLoading">
      <h4>Learn about Convex</h4>
      <ul className="flex justify-between gap-y-6 py-6">
        {docs.map((doc) => (
          <DocCard key={doc.title} {...doc} />
        ))}
      </ul>
    </div>
  );
}
