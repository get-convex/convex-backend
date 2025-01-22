import React from "react";
import Link from "@docusaurus/Link";
import useBaseUrl from "@docusaurus/useBaseUrl";
import { translate } from "@docusaurus/Translate";
export default function HomeBreadcrumbItem() {
  const homeHref = useBaseUrl("/");
  return (
    <li className="breadcrumbs__item">
      <Link
        aria-label={translate({
          id: "theme.docs.breadcrumbs.home",
          message: "Home page",
          description: "The ARIA label for the home page in the breadcrumbs",
        })}
        className="breadcrumbs__link"
        href={homeHref}
      >
        Home
      </Link>
    </li>
  );
}
