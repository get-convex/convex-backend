import { ChevronDownIcon, MixerHorizontalIcon } from "@radix-ui/react-icons";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@common/elements/Button";
import { cn } from "@common/lib/cn";
import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export const filterMenuId = "filterMenu";

export function FilterButton({
  filters,
  onClick,
  open,
}: {
  filters?: FilterExpression;
  onClick(): void;
  open: boolean;
}) {
  const validFilterNames = filters
    ? new Set(
        filters.clauses
          .filter(
            (filter) => filter.field !== undefined && filter.enabled !== false,
          )
          .map((filter) => filter.field),
      )
    : new Set([]);
  const indexFilters = filters?.index?.clauses.filter(
    (clause) => clause.enabled,
  );

  const regularFilters = filters?.clauses.filter(
    (filter) => filter.enabled !== false,
  );

  const { enableIndexFilters } = useContext(DeploymentInfoContext);

  const filterButtonContent = (
    <div className="flex items-center gap-2">
      <span>{enableIndexFilters ? "Filter & Sort" : "Filter"}</span>
      {(indexFilters?.length || validFilterNames.size > 0) && (
        <span className="rounded-full border bg-background-primary px-1 py-0 text-xs tabular-nums leading-[14px] text-content-secondary">
          {(indexFilters?.length || 0) + (regularFilters?.length || 0)}
        </span>
      )}
    </div>
  );

  return (
    <Button
      size="xs"
      variant="neutral"
      onClick={onClick}
      aria-controls={filterMenuId}
      aria-expanded={open}
      aria-haspopup="menu"
      aria-label="Filter"
      icon={<MixerHorizontalIcon className="size-3.5" />}
      focused={open}
      className={cn(
        "w-fit rounded-l-none text-xs border border-border-transparent",
        open && "rounded-b-none border-b-0",
      )}
      inline
    >
      {filterButtonContent}
      <ChevronDownIcon
        className={cn("transition-all", open && "-rotate-180")}
      />
    </Button>
  );
}
