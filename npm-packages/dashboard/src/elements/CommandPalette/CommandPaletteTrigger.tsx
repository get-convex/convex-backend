import React from "react";
import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useCommandPaletteOpen } from "./CommandPalette";

// The header search bar that opens the command palette.
export function CommandPaletteTrigger() {
  const { commandPalette } = useLaunchDarkly();
  const [, setOpen] = useCommandPaletteOpen();

  if (!commandPalette) {
    return null;
  }

  return (
    <Button
      variant="unstyled"
      onClick={() => setOpen(true)}
      className="mx-2 hidden w-56 items-center gap-2 rounded-full border bg-background-secondary px-3 py-1.5 text-sm text-content-tertiary transition-colors hover:bg-background-tertiary md:flex"
    >
      <MagnifyingGlassIcon className="size-4 shrink-0" />
      <span className="select-none">Find anything</span>
      <kbd className="ml-auto rounded-sm border bg-background-tertiary px-1.5 font-sans text-xs text-content-secondary">
        /
      </kbd>
    </Button>
  );
}
