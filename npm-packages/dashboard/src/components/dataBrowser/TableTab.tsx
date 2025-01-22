import { Tooltip, sidebarLinkClassNames } from "dashboard-common";
import { useIsOverflowing } from "hooks/useIsOverflowing";
import { useRef } from "react";
import Link from "next/link";
import { useRouter } from "next/router";
import omit from "lodash/omit";

export function TableTab({
  selectedTable,
  table,
  onSelectTable,
  isMissingFromSchema,
}: {
  selectedTable: string | null;
  table: string;
  onSelectTable?: () => void;
  isMissingFromSchema?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const isOverflowing = useIsOverflowing(ref);
  const { pathname, query } = useRouter();

  return (
    <Tooltip
      tip={
        isOverflowing || isMissingFromSchema ? (
          <div className="break-all">
            {isOverflowing ? table : null}
            {isMissingFromSchema && (
              <div>This table is not defined in your schema.</div>
            )}
          </div>
        ) : undefined
      }
      side="right"
    >
      <div className="relative">
        <Tooltip
          tip={
            isMissingFromSchema && "This table is not defined in your schema."
          }
          className="flex w-full items-start gap-0.5"
          side="right"
          wrapsButton
        >
          <Link
            href={{
              pathname,
              query: {
                ...omit(query, "filters"),
                table,
              },
            }}
            key={table}
            className={sidebarLinkClassNames({
              isActive: selectedTable === table,
              small: true,
            })}
            onClick={() => onSelectTable?.()}
          >
            <div className="flex w-full max-w-full items-start gap-0.5">
              <div className="shrink truncate" ref={ref}>
                {table}
              </div>
              {isMissingFromSchema && (
                <div className="font-sans text-sm">*</div>
              )}
            </div>
          </Link>
        </Tooltip>
      </div>
    </Tooltip>
  );
}
