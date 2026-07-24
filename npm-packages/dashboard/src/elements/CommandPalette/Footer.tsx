import { useCommandState } from "cmdk";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { cn } from "@ui/cn";
import { REMOTE_VALUE_PREFIX } from "./navigation";

export const KBD_CLASSES =
  "rounded-sm border bg-background-tertiary px-1 text-content-secondary";

// Values of items whose Enter action is direct navigation but that can also
// be browsed (drilled into) with the modifier — project and deployment
// results.
function isBrowsableItemValue(value: string | undefined): boolean {
  return (
    value !== undefined &&
    (value.startsWith(`${REMOTE_VALUE_PREFIX}project:`) ||
      value.startsWith("deployment:") ||
      value.startsWith(`${REMOTE_VALUE_PREFIX}deployment:`))
  );
}

export function Footer({
  inSubPage,
  status,
}: {
  inSubPage: boolean;
  // Optional status published by the active page, shown in the right gutter.
  status?: React.ReactNode;
}) {
  // cmdk keeps its selection in sync with both keyboard focus and pointer
  // hover, so this covers "hovering or focusing" a browsable item.
  const selectedValue = useCommandState((state) => state.value);
  return (
    <div className="-mx-2 mt-2 flex items-center gap-4 border-t px-5 pt-2 text-xs text-content-tertiary select-none">
      <span className="flex items-center gap-1">
        <KeyboardShortcut value={["Up", "Down"]} className={KBD_CLASSES} />
        Navigate
      </span>
      <span className="flex items-center gap-1">
        <KeyboardShortcut value={["Return"]} className={KBD_CLASSES} />
        Select
      </span>
      {isBrowsableItemValue(selectedValue) && (
        <span className="flex animate-fadeInFromLoading items-center gap-1">
          <KeyboardShortcut
            value={["Shift", "Return"]}
            className={KBD_CLASSES}
          />
          Browse
        </span>
      )}
      {inSubPage && (
        <span className="flex items-center gap-1">
          <KeyboardShortcut value={["Backspace"]} className={KBD_CLASSES} />
          Back
        </span>
      )}
      {status && (
        <span className="ml-auto text-content-secondary">{status}</span>
      )}
      <span className={cn("flex items-center gap-1", !status && "ml-auto")}>
        <KeyboardShortcut value={["Esc"]} className={KBD_CLASSES} />
        {inSubPage ? "Back" : "Close"}
      </span>
    </div>
  );
}
