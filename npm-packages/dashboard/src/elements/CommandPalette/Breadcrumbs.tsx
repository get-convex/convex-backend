import React from "react";
import { CaretRightIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { PalettePage, pageLabel } from "./pages";

export function Breadcrumbs({
  pages,
  // The number of pages that form the fixed base of the current view. In the
  // global palette this is 0, so the trail is rooted at "Home". In a contextual
  // menu (anchored to a switcher) it's the depth the menu opened at, so the
  // trail is rooted at that menu's own base page instead of the whole palette.
  baseDepth = 0,
  // Navigate back to a given depth in the drill-in stack: `baseDepth` returns
  // to the base view, n keeps the first n pages. Clicking a crumb pops
  // everything after it.
  onNavigate,
}: {
  pages: PalettePage[];
  baseDepth?: number;
  onNavigate: (depth: number) => void;
}) {
  // In a contextual menu, start the trail at the menu's base page rather than
  // the palette's Home root.
  const startIndex = baseDepth > 0 ? baseDepth - 1 : 0;
  return (
    <div className="-mx-1 -mt-1.5 mb-1.5 flex animate-fadeInFromLoading items-center gap-1 border-b bg-background-tertiary/40 px-3 pt-3 pb-2 select-none">
      {baseDepth === 0 && <Crumb onClick={() => onNavigate(0)}>Home</Crumb>}
      {pages.map((page, i) => {
        if (i < startIndex) {
          return null;
        }
        const isCurrent = i === pages.length - 1;
        // The first crumb shown when rooted at a contextual base leads its row,
        // so it needs no separator before it.
        const needsSeparator = baseDepth === 0 || i > startIndex;
        return (
          <React.Fragment key={i}>
            {needsSeparator && (
              <CaretRightIcon className="size-3 text-content-secondary" />
            )}
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
