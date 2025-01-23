import { ChangeEventHandler, useRef } from "react";
import { classNames } from "../utils";

type InputProps = {
  placeholder?: string;
  value?: string;
  onChange?: ChangeEventHandler<HTMLTextAreaElement>;
  className?: string;
  inputProps?: React.DetailedHTMLProps<
    React.TextareaHTMLAttributes<HTMLTextAreaElement>,
    HTMLTextAreaElement
  >;
  error?: string;
  disabled?: boolean;
};

export default function TextInput({
  placeholder,
  value = "",
  onChange,
  className,
  inputProps,
  error,
  disabled,
}: InputProps) {
  const inputRef = useRef<HTMLTextAreaElement>(null);
  return (
    <div className="flex w-full flex-col gap-1">
      <div
        className={classNames("flex items-center justify-between", className)}
      >
        <textarea
          ref={inputRef}
          id="input-field"
          aria-multiline
          disabled={disabled}
          placeholder={placeholder}
          name="input"
          onChange={onChange}
          value={value}
          spellCheck={false}
          {...inputProps}
          className={classNames(
            "bg-light-background-secondary dark:bg-dark-background-tertiary",
            error &&
              "focus:border-light-content-errorSecondary dark:focus:border-dark-content-errorSecondary",
            !error &&
              "focus:border-light-border-selected dark:focus:border-dark-border-selected text-light-content-primary dark:text-dark-content-primary",
            "block rounded-md px-4 py-2",
            "disabled:text-light-content-secondary dark:disabled:text-dark-content-secondary disabled:bg-light-background-tertiary dark:disabled:bg-dark-background-tertiary placeholder-light-content-secondary dark:placeholder-dark-content-secondary border border-slate-400 focus:outline-none",
            "text-sm shrink grow disabled:cursor-not-allowed truncate",
            inputProps?.className,
          )}
        />
      </div>
    </div>
  );
}
