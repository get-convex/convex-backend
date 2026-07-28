import React from "react";
import { CaretRightIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { PalettePage, pageLabel } from "./pages";

export function Breadcrumbs({
  pages,
  // Navigate back to a given depth in the drill-in stack: 0 is Home (the root),
  // n keeps the first n pages. Clicking a crumb pops everything after it.
  onNavigate,
}: {
  pages: PalettePage[];
  onNavigate: (depth: number) => void;
}) {
  return (
    <div className="flex animate-fadeInFromLoading items-center gap-1 px-3 pt-2 select-none">
      <Crumb onClick={() => onNavigate(0)}>Home</Crumb>
      {pages.map((page, i) => {
        const isCurrent = i === pages.length - 1;
        return (
          <React.Fragment key={i}>
            <CaretRightIcon className="size-3 text-content-secondary" />
            {isCurrent ? (
              <span className="max-w-48 truncate rounded-sm border bg-background-tertiary px-1.5 py-0.5 text-xs text-content-primary">
                {pageLabel(page)}
              </span>
            ) : (
              <Crumb onClick={() => onNavigate(i + 1)}>{pageLabel(page)}</Crumb>
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
}

// A clickable crumb. Higher-contrast text than the current-page crumb plus a
// hover state so it reads as a "go back here" target.
function Crumb({
  children,
  onClick,
}: {
  children: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <Button
      variant="unstyled"
      onClick={onClick}
      className="max-w-48 truncate rounded-sm border bg-background-tertiary px-1.5 py-0.5 text-xs text-content-primary transition-colors hover:border-border-selected hover:bg-background-primary"
    >
      {children}
    </Button>
  );
}
