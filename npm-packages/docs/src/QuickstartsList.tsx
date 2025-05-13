declare module "*.svg" {
  const content: React.FunctionComponent<React.SVGProps<SVGSVGElement>>;
  export default content;
}

import ExpoLogo from "@site/static/img/expo-logo.svg";
import NextJSLogo from "@site/static/img/nextjs-logo.svg";
import HtmlLogo from "@site/static/img/html-logo.svg";
import JsLogo from "@site/static/img/js-logo.svg";
import NodeLogo from "@site/static/img/node-logo.svg";
import BunLogo from "@site/static/img/bun-logo.svg";
import PythonLogo from "@site/static/img/python-logo.svg";
import ReactLogo from "@site/static/img/react-logo.svg";
import RemixLogo from "@site/static/img/remix-logo.svg";
import RustLogo from "@site/static/img/rust-logo.svg";
import SvelteLogo from "@site/static/img/svelte-logo.svg";
import VueLogo from "@site/static/img/vue-logo.svg";
import AndroidLogo from "@site/static/img/android-logo.svg";
import SwiftLogo from "@site/static/img/swift-logo.svg";
import TanStackLogo from "@site/static/img/tanstack-logo.svg";
import React from "react";
import Link from "@docusaurus/Link";
import { ReactNode } from "@mdx-js/react/lib";
import Heading from "@theme/Heading";

// This is a variant of DocsCardList.tsx specifically for the Quickstarts page.
type Item = {
  docId: string;
  href: string;
  label: string;
  icon?: ReactNode;
  invertIcon?: true;
};

// Add this new type after the existing Item type
type LargeCardItem = {
  href: string;
  title: string;
  description: string;
};

export function DocCardList(props: { items: Item[] }) {
  const { items } = props;
  return (
    <div className="qs-cards">
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
      </div>
    </Link>
  );
}

// Add this new component before Quick*List
export function LargeCardList(props: { items: LargeCardItem[] }) {
  return (
    <div className="large-cards">
      {props.items.map((item, index) => (
        <Link key={index} href={item.href} className="large-card">
          <Heading as="h2">{item.title}</Heading>
          <p>{item.description}</p>
        </Link>
      ))}
    </div>
  );
}

// End DocsCardList.tsx variant for Quickstarts page

export function QuickFrameworksList() {
  return (
    <DocCardList
      items={[
        {
          icon: <ReactLogo height={40} />,
          href: "/quickstart/react",
          docId: "quickstart/react",
          label: "React",
        },
        {
          icon: <NextJSLogo height={40} />,
          invertIcon: true,
          href: "/quickstart/nextjs",
          docId: "quickstart/nextjs",
          label: "Next.js",
        },
        {
          icon: <RemixLogo height={40} />,
          invertIcon: true,
          href: "/quickstart/remix",
          docId: "quickstart/remix",
          label: "Remix",
        },
        {
          icon: <TanStackLogo height={40} />,
          href: "/quickstart/tanstack-start",
          docId: "quickstart/tanstack-start",
          label: "TanStack Start",
        },
        {
          icon: <ExpoLogo height={40} />,
          invertIcon: true,
          href: "/quickstart/react-native",
          docId: "quickstart/react-native",
          label: "React Native",
        },
        {
          icon: <VueLogo height={40} />,
          href: "/quickstart/vue",
          docId: "quickstart/vue",
          label: "Vue",
        },
        {
          icon: <SvelteLogo height={40} />,
          href: "/quickstart/svelte",
          docId: "quickstart/svelte",
          label: "Svelte",
        },
        {
          icon: <NodeLogo height={40} />,
          href: "/quickstart/nodejs",
          docId: "quickstart/nodejs",
          label: "Node.js",
        },
        {
          icon: <BunLogo height={40} />,
          href: "/quickstart/bun",
          docId: "quickstart/bun",
          label: "Bun",
        },
        {
          icon: <HtmlLogo height={40} />,
          invertIcon: true,
          href: "/quickstart/script-tag",
          docId: "quickstart/script-tag",
          label: "Script tag",
        },
      ]}
    />
  );
}

export function QuickLanguagesList() {
  return (
    <DocCardList
      items={[
        {
          icon: <JsLogo height={40} />,
          invertIcon: true,
          href: "/client/javascript",
          docId: "client/javascript",
          label: "JavaScript",
        },
        {
          icon: <PythonLogo height={40} />,
          href: "/quickstart/python",
          docId: "quickstart/python",
          label: "Python",
        },
        {
          icon: <SwiftLogo height={40} />,
          href: "/quickstart/swift",
          docId: "quickstart/swift",
          label: "iOS Swift",
        },
        {
          icon: <AndroidLogo height={40} />,
          href: "/quickstart/android",
          docId: "quickstart/android",
          label: "Android Kotlin",
        },
        {
          icon: <RustLogo height={40} width={40} />,
          href: "/quickstart/rust",
          docId: "quickstart/rust",
          label: "Rust",
        },
      ]}
    />
  );
}
