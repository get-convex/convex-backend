import { ChevronDownIcon, MixerHorizontalIcon } from "@radix-ui/react-icons";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { Button } from "@ui/Button";
import { cn } from "@ui/cn";
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

  const hasAnyEnabledFilters =
    indexFilters?.length || validFilterNames.size > 0;

  const filterButtonContent = (
    <div className="flex items-center gap-2">
      <span>{enableIndexFilters ? "Filter & Sort" : "Filter"}</span>
      {hasAnyEnabledFilters && (
        <span className="rounded-full border border-content-primary px-1 py-0 text-xs tabular-nums leading-[14px]">
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
        "w-fit rounded-l-none text-xs border-0 border-l",
        hasAnyEnabledFilters &&
          "bg-blue-100/50 dark:bg-blue-700/50 hover:bg-blue-100/70 dark:hover:bg-blue-700/70",
        open && "rounded-b-none",
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
