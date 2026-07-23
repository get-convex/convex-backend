import React from "react";
import { CaretRightIcon } from "@radix-ui/react-icons";
import { PalettePage, pageLabel } from "./pages";

export function Breadcrumbs({ pages }: { pages: PalettePage[] }) {
  return (
    <div className="flex animate-fadeInFromLoading items-center gap-1 px-3 pt-2 select-none">
      <span className="rounded-sm bg-background-tertiary px-1.5 py-0.5 text-xs text-content-secondary">
        Home
      </span>
      {pages.map((page, i) => (
        <React.Fragment key={i}>
          <CaretRightIcon className="size-3 text-content-tertiary" />
          <span className="max-w-48 truncate rounded-sm bg-background-tertiary px-1.5 py-0.5 text-xs text-content-secondary">
            {pageLabel(page)}
          </span>
        </React.Fragment>
      ))}
    </div>
  );
}
