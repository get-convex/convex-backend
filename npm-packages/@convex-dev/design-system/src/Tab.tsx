import { Tab as HeadlessTab } from "@headlessui/react";
import { Fragment, PropsWithChildren } from "react";
import { Button, ButtonProps } from "@ui/Button";
import { cn } from "@ui/cn";

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
            "cursor-pointer px-3 py-2 text-sm whitespace-nowrap",
            !disabled && selected
              ? "border-b-2 border-content-primary text-content-primary"
              : "text-content-secondary",
            disabled
              ? "cursor-not-allowed disabled:text-content-secondary"
              : "hover:text-content-primary",
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
