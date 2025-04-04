import { Tab as HeadlessTab } from "@headlessui/react";
import { Fragment, PropsWithChildren } from "react";
import { Button, ButtonProps } from "@common/elements/Button";
import { cn } from "@common/lib/cn";

export function Tab({
  disabled,
  tip,
  children,
  large = false,
  className,
  ...props
}: ButtonProps &
  PropsWithChildren<{ disabled?: boolean; tip?: string; large?: boolean }>) {
  return (
    <HeadlessTab as={Fragment}>
      {({ selected }) => (
        <Button
          disabled={disabled}
          tip={tip}
          variant="unstyled"
          className={cn(
            "p-2 text-sm rounded whitespace-nowrap cursor-pointer",
            !disabled && selected
              ? "text-content-primary"
              : "text-content-secondary",
            disabled
              ? "disabled:text-content-secondary cursor-not-allowed"
              : "hover:bg-background-tertiary",
            selected &&
              "font-semibold underline underline-offset-8 decoration-2",
            // It's OK for tabs.
            // eslint-disable-next-line no-restricted-syntax
            large && "text-lg",
            className,
          )}
          {...props}
        >
          {children}
        </Button>
      )}
    </HeadlessTab>
  );
}
