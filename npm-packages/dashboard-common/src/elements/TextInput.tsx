import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import React, { forwardRef } from "react";
import { Button } from "elements/Button";

type InputProps = {
  label?: string;
  labelHidden?: boolean;
  outerClassname?: string;
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
  Icon?: React.FC<{ className: string | undefined }>;
  SearchIcon?: React.FC<{ className: string | undefined }>;
  action?: () => void;
  error?: string;
  description?: React.ReactNode;
  id: string;
  type?: "text" | "search" | "email" | "time" | "password";
};

export const TextInput = forwardRef<
  HTMLInputElement,
  InputProps & Omit<React.HTMLProps<HTMLInputElement>, "onChange">
>(
  (
    {
      outerClassname,
      label,
      labelHidden = false,
      Icon,
      SearchIcon,
      action = () => {},
      error,
      description,
      className,
      onChange,
      type = "text",
      id,
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
        {type === "search" && (
          <div className="pointer-events-none absolute inset-y-0 left-3 flex items-center">
            {SearchIcon ? (
              <SearchIcon className="text-content-secondary" />
            ) : (
              <MagnifyingGlassIcon className="text-content-secondary" />
            )}
          </div>
        )}
        <input
          onChange={onChange}
          type={type}
          spellCheck={false}
          id={id}
          name={id}
          className={classNames(
            error && "focus:border-content-error",
            !error && "focus:border-border-selected text-content-primary",
            "block rounded px-3 py-2 bg-background-secondary",
            "disabled:text-content-secondary disabled:bg-background-tertiary placeholder-content-tertiary border focus:outline-none",
            "text-sm shrink grow disabled:cursor-not-allowed truncate",
            "min-w-0",
            type === "search" && "pl-9",
            className,
          )}
          {...rest}
        />
        {Icon && (
          <Button
            size="sm"
            onClick={action}
            className="float-right ml-[-2.375rem] mr-1.5"
            variant={error ? "danger" : "neutral"}
            inline
            icon={<Icon className="h-3.5 w-3.5" />}
          />
        )}
      </div>
      {error && (
        <p
          className="flex max-w-prose animate-fadeInFromLoading gap-1 text-xs text-content-errorSecondary"
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
