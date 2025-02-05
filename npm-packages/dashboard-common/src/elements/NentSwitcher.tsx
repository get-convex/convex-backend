import { cn } from "@common/lib/cn";
import { Combobox } from "@common/elements/Combobox";
import { PuzzlePieceIcon } from "@common/elements/icons";
import { Tooltip } from "@common/elements/Tooltip";
import { NENT_APP_PLACEHOLDER, Nent, useNents } from "@common/lib/useNents";

export function NentSwitcher({
  onChange,
}: {
  onChange?: (nent: string | null) => void;
}) {
  const { nents: allNents, selectedNent, setSelectedNent } = useNents();
  if (!allNents || allNents.length <= 1) {
    return null;
  }
  const nents = allNents.filter((nent) => nent.name !== null);

  const mounted = nents.filter((nent) => nent.state === "active");
  const unmounted = nents.filter((nent) => nent.state !== "active");

  return (
    <div className="mb-4">
      <Combobox
        buttonProps={{
          tip: "Switch between components installed in this deployment.",
          tipSide: "right",
        }}
        label="Select component"
        selectedOption={selectedNent || undefined}
        buttonClasses="text-right truncate w-full"
        innerButtonClasses={
          selectedNent
            ? "bg-yellow-100/50 dark:bg-yellow-600/20 hover:bg-yellow-100 dark:hover:bg-yellow-600/50"
            : ""
        }
        className="static truncate"
        Option={NentNameOption}
        setSelectedOption={(nent) => {
          nent === null
            ? void setSelectedNent(undefined)
            : void setSelectedNent(nent?.id ?? undefined);
          onChange && onChange(nent?.id || null);
        }}
        searchPlaceholder="Search components..."
        options={[
          { label: NENT_APP_PLACEHOLDER, value: undefined },
          ...mounted.map((nent) => ({
            label: nent.path,
            value: nent,
          })),
          ...unmounted.map((nent) => ({
            label: nent.path,
            value: nent,
          })),
        ]}
      />
    </div>
  );
}

export function NentNameOption({
  label,
  value,
  inButton,
}: {
  label: string;
  value?: Nent;
  inButton: boolean;
}) {
  return (
    <Tooltip
      tip={
        value && value.state !== "active"
          ? "This component is unmounted"
          : undefined
      }
      side="right"
      className={cn(
        "w-full flex items-center",
        value && value.state !== "active" && "text-content-tertiary",
      )}
    >
      <div className="flex items-center gap-1 truncate">
        {inButton && <PuzzlePieceIcon className="mt-[1px] min-w-[13px]" />}
        <span className="truncate">
          {label === NENT_APP_PLACEHOLDER ? "app" : label}
          {value && value.state !== "active" && "*"}
        </span>
      </div>
    </Tooltip>
  );
}
