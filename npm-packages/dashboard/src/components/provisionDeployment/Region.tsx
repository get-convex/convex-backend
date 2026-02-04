import type { RegionName } from "generatedApi";
import { DeploymentRegionMetadata } from "@convex-dev/platform/managementApi";
import { Field, Radio, Label } from "@headlessui/react";
import { cn } from "@ui/cn";
import { Tooltip } from "@ui/Tooltip";
import Link from "next/link";

export function Region({
  region,
  teamSlug,
}: {
  region: DeploymentRegionMetadata;
  teamSlug?: string;
}) {
  const radioContent = (
    <div className="flex text-start">
      <Field disabled={!region.available} className="flex-1">
        <Radio
          value={region.name}
          className={cn(
            "group relative flex cursor-pointer rounded-xl border px-4 py-3 focus:outline-none sm:flex sm:justify-between",
            "h-full",
            "[--region-selector-border:transparent]",
            "data-checked:[--region-selector-border:var(--border-selected)]",
            "data-focus:[--region-selector-border:var(--border-selected)]",
            "bg-background-secondary transition-colors hover:bg-background-primary",
            "data-disabled:cursor-not-allowed data-disabled:bg-background-tertiary",
          )}
        >
          <span
            className={cn(
              "border border-(--region-selector-border)",
              "pointer-events-none absolute -inset-px rounded-xl",
            )}
            aria-hidden="true"
          />
          <div className="flex w-full items-center justify-between">
            <div className="flex items-center gap-3">
              <span
                // eslint-disable-next-line no-restricted-syntax
                className="text-2xl leading-none"
                role="presentation"
              >
                {getRegionFlag(region.name)}
              </span>
              <div className="flex flex-col gap-0.5">
                {(() => {
                  const { mainName, placeName } = parseRegionLabel(
                    region.displayName,
                  );
                  return (
                    <>
                      <Label
                        className={cn(
                          "text-sm leading-tight font-semibold",
                          !region.available
                            ? "text-content-secondary"
                            : "text-content-primary",
                        )}
                      >
                        {mainName}
                      </Label>
                      {placeName && (
                        <span className="text-xs leading-tight text-content-secondary">
                          {placeName}
                        </span>
                      )}
                    </>
                  );
                })()}
              </div>
            </div>
            <div className="ml-6 flex items-center gap-3">
              <div className="flex size-3.5 items-center justify-center rounded-full border border-border-transparent bg-background-secondary group-data-checked:border-util-accent group-data-checked:bg-util-accent">
                <span className="invisible size-1.5 rounded-full bg-white group-data-checked:visible" />
              </div>
            </div>
          </div>
        </Radio>
      </Field>
    </div>
  );

  return !region.available ? (
    <Tooltip
      className="w-full"
      tip={
        <div className="flex flex-col gap-1">
          <p>This region is not available on your current plan.</p>
          {teamSlug && (
            <p>
              <Link
                href={`/t/${teamSlug}/settings/billing`}
                className="text-content-link hover:underline"
                onClick={(e) => e.stopPropagation()}
              >
                Upgrade your plan
              </Link>{" "}
              to access this region.
            </p>
          )}
        </div>
      }
      side="top"
    >
      {radioContent}
    </Tooltip>
  ) : (
    radioContent
  );
}

function getRegionFlag(regionValue: RegionName): string {
  if (regionValue === "aws-us-east-1") return "🇺🇸";
  if (regionValue === "aws-eu-west-1") return "🇪🇺";
  return "🏳️";
}

function parseRegionLabel(label: string): {
  mainName: string;
  placeName: string;
} {
  const match = label.match(/^(.+?)\s*\((.+?)\)$/);
  if (match) {
    return { mainName: match[1].trim(), placeName: match[2].trim() };
  }
  return { mainName: label, placeName: "" };
}
