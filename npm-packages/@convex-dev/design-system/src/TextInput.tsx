import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import React, { forwardRef } from "react";
import { Button } from "@ui/Button";
import { cn } from "@ui/cn";

type InputProps = {
  label?: string;
  labelHidden?: boolean;
  outerClassname?: string;
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
  SearchIcon?: React.FC<{ className: string | undefined }>;
  /** A non-interactive element appearing to the left of the input. */
  leftAddon?: React.ReactNode;
  /** A non-interactive element appearing to the right of the input. */
  rightAddon?: React.ReactNode;
  /** An interactive element appearing to the right of the input. */
  Icon?: React.FC<{ className: string | undefined }>;
  iconTooltip?: string;
  /** The action on `Icon`. */
  action?: () => void;
  error?: string;
  description?: React.ReactNode;
  id: string;
  type?: "text" | "search" | "email" | "time" | "password" | "number";
  size?: "sm" | "md";
};

export const TextInput = forwardRef<
  HTMLInputElement,
  InputProps & Omit<React.HTMLProps<HTMLInputElement>, "onChange" | "size">
>(
  (
    {
      outerClassname,
      label,
      labelHidden = false,
      Icon,
      iconTooltip,
      SearchIcon,
      leftAddon,
      rightAddon,
      action = () => {},
      error,
      description,
      className,
      onChange,
      type = "text",
      id,
      size = "md",
      ...rest
    },
    ref,
  ) => (
    <div ref={ref} className="flex w-full flex-col gap-1">
      <label
        className="text-left text-sm text-content-primary"
        htmlFor={id}
        hidden={type === "search" || labelHidden}
      >
        {label || id}
      </label>
      <div
        className={classNames(
          "relative flex items-center justify-between",
          outerClassname,
        )}
      >
        {(type === "search" || leftAddon !== undefined) && (
          <div className="pointer-events-none absolute inset-y-0 left-1.5 flex items-center">
            {leftAddon ??
              (SearchIcon ? (
                <SearchIcon className="text-content-secondary" />
              ) : (
                <MagnifyingGlassIcon className="text-content-secondary" />
              ))}
          </div>
        )}
        <input
          onChange={onChange}
          type={type}
          spellCheck={false}
          id={id}
          name={id}
          className={cn(
            error && "focus:border-content-error",
            !error && "focus:border-border-selected text-content-primary",
            "block rounded-md bg-background-secondary",
            size === "sm" ? "px-1.5 py-1 text-xs" : "p-1.5 px-2 text-sm",
            "disabled:text-content-secondary disabled:bg-background-tertiary placeholder-content-tertiary border focus:outline-none",
            "shrink grow disabled:cursor-not-allowed truncate",
            "min-w-0",
            (type === "search" || leftAddon !== undefined) && "pl-6",
            rightAddon !== undefined && "pr-6",
            Icon && "pr-10",
            className,
          )}
          {...rest}
        />
        {rightAddon !== undefined && (
          <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center">
            {rightAddon}
          </div>
        )}
        {Icon && (
          <Button
            size="sm"
            onClick={action}
            className="float-right ml-[-2.375rem] mr-1.5"
            variant={error ? "danger" : "neutral"}
            inline
            icon={<Icon className="h-3.5 w-3.5" />}
            tip={iconTooltip}
          />
        )}
      </div>
      {error && (
        <p
          className="flex max-w-full animate-fadeInFromLoading gap-1 break-words text-xs text-content-errorSecondary"
          role="alert"
        >
          {error}
        </p>
      )}
      {description && !error && (
        <p className="max-w-prose animate-fadeInFromLoading text-xs text-content-secondary">
          {description}
        </p>
      )}
    </div>
  ),
);

TextInput.displayName = "TextInput";
