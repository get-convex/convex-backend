import FunnelIcon from "@heroicons/react/24/outline/FunnelIcon";
import { ChevronUpIcon, ChevronDownIcon } from "@radix-ui/react-icons";
import { Button } from "dashboard-common";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { filterMenuId } from "./DataFilters";

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
            (filter) =>
              filter.field !== undefined && filter.value !== undefined,
          )
          .map((filter) => filter.field),
      )
    : new Set([]);
  const filterButtonContent =
    validFilterNames.size === 0 ? (
      "Filter"
    ) : (
      <div>
        Filtered by:{" "}
        {validFilterNames.size > 3 ? (
          `${validFilterNames.size} fields`
        ) : (
          <code>{Array.from(validFilterNames).join(", ")}</code>
        )}
      </div>
    );

  return (
    <div>
      <Button
        size="sm"
        variant="neutral"
        onClick={onClick}
        aria-controls={filterMenuId}
        icon={<FunnelIcon className="h-4 w-4" />}
        focused={open}
      >
        {filterButtonContent}
        {/*  TODO: Post-icon in button */}
        <span className="ml-1">
          {open ? <ChevronUpIcon /> : <ChevronDownIcon />}
        </span>
      </Button>
    </div>
  );
}
