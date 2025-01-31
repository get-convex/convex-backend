import { useState } from "react";
import { Combobox as HeadlessCombobox } from "@headlessui/react";
import { CaretSortIcon, MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { cn } from "lib/cn";
import isEqual from "lodash/isEqual";
import { test } from "fuzzy";
import { Button, ButtonProps } from "elements/Button";

export type Option<T> = { label: string; value: T };

export function Combobox<T>({
  options,
  optionsWidth = "fixed",
  selectedOption,
  setSelectedOption,
  buttonClasses,
  innerButtonClasses,
  className,
  allowCustomValue = false,
  label,
  Option,
  searchPlaceholder = "Search...",
  disableSearch = false,
  buttonProps,
  disabled = false,
  unknownLabel = () => "Unknown option",
  labelHidden = true,
  processFilterOption = (option: string) => option,
  placeholder = "Select an option",
}: {
  label: React.ReactNode;
  labelHidden?: boolean;
  className?: string;
  options: Readonly<Option<T>[]>;
  placeholder?: string;
  searchPlaceholder?: string;
  disableSearch?: boolean;
  // "full" only works if the options dropdown
  // fits inside of the ComboBox's ancestor elements,
  // or if the ancestors allow overflow.
  optionsWidth?: "full" | "fixed";
  selectedOption?: T | null;
  setSelectedOption: (option: T | null) => void;
  buttonClasses?: string;
  buttonProps?: Omit<ButtonProps, "href">;
  innerButtonClasses?: string;
  allowCustomValue?: boolean;
  Option?: React.ComponentType<{ label: string; value: T; inButton: boolean }>;
  disabled?: boolean;
  unknownLabel?: (value: T) => string;
  processFilterOption?: (option: string) => string;
}) {
  const [query, setQuery] = useState("");
  const filtered =
    query === ""
      ? options
      : options.filter((option) =>
          test(query, processFilterOption(option.label)),
        );

  const selectedOptionData = options.find((o) =>
    isEqual(selectedOption, o.value),
  );
  return (
    <HeadlessCombobox
      value={
        options.find((o) => isEqual(selectedOption, o.value))?.value || null
      }
      onChange={(option) => {
        setSelectedOption(option);
        setQuery("");
      }}
      disabled={disabled}
    >
      {({ open }) => (
        <>
          <HeadlessCombobox.Label
            hidden={labelHidden}
            className="text-left text-sm text-content-primary"
          >
            {label}
          </HeadlessCombobox.Label>
          <div className={cn("relative", className)}>
            <div
              className={cn("relative flex items-center w-60", buttonClasses)}
            >
              <HeadlessCombobox.Button
                as={Button}
                variant="unstyled"
                data-testid={`combobox-button-${label}`}
                className={cn(
                  "flex gap-1 w-full items-center group",
                  "truncate relative text-left text-content-primary rounded disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                  "border focus:border-border-selected focus:outline-none bg-background-secondary text-sm py-2 px-3",
                  "hover:bg-background-tertiary",
                  open && "border-border-selected",
                  "cursor-pointer",
                  innerButtonClasses,
                )}
                {...buttonProps}
              >
                <div className="truncate">
                  {!!Option && !!selectedOptionData ? (
                    <Option
                      inButton
                      label={selectedOptionData.label}
                      value={selectedOptionData.value}
                    />
                  ) : (
                    selectedOptionData?.label || (
                      <span className="text-content-tertiary">
                        {selectedOption && unknownLabel(selectedOption)}
                      </span>
                    )
                  )}
                  {!selectedOptionData && (
                    <span className="text-content-tertiary">{placeholder}</span>
                  )}
                </div>
                <CaretSortIcon
                  className={cn("text-content-primary", "h-5 w-5 ml-auto")}
                />
              </HeadlessCombobox.Button>
            </div>
            {open && (
              <HeadlessCombobox.Options
                static
                className={cn(
                  "mt-1 absolute z-50 max-h-[14.75rem] overflow-auto rounded bg-background-secondary pb-1 text-xs shadow scrollbar border",
                  optionsWidth === "full" ? "w-full" : "w-60",
                )}
              >
                <div className="min-w-fit">
                  {!disableSearch && (
                    <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3 pt-1">
                      <MagnifyingGlassIcon className="text-content-secondary" />
                      <HeadlessCombobox.Input
                        onChange={(event) => setQuery(event.target.value)}
                        value={query}
                        autoFocus
                        className={cn(
                          "placeholder:text-content-tertiary truncate relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                          "focus:outline-none bg-background-secondary",
                        )}
                        placeholder={searchPlaceholder}
                      />
                    </div>
                  )}
                  {filtered.map((option, idx) => (
                    <HeadlessCombobox.Option
                      key={idx}
                      value={option.value}
                      className={({ active }) =>
                        cn(
                          "w-fit min-w-full relative cursor-pointer select-none py-1.5 px-3 text-content-primary",
                          active && "bg-background-tertiary",
                        )
                      }
                    >
                      {({ selected }) => (
                        <span
                          className={cn(
                            "block w-full whitespace-nowrap",
                            selected && "font-semibold",
                          )}
                        >
                          {Option ? (
                            <Option
                              label={option.label}
                              value={option.value}
                              inButton={false}
                            />
                          ) : (
                            option.label
                          )}
                        </span>
                      )}
                    </HeadlessCombobox.Option>
                  ))}

                  {/* Allow users to type a custom value */}
                  {allowCustomValue &&
                    query.length > 0 &&
                    !filtered.some((x) => x.value === query) && (
                      <HeadlessCombobox.Option
                        value={query}
                        className={({ active }) =>
                          `text-content-primary relative cursor-pointer w-60 select-none py-1 px-3 text-xs ${
                            active ? "bg-background-tertiary" : ""
                          }`
                        }
                      >
                        Unknown option: "{query}"
                      </HeadlessCombobox.Option>
                    )}

                  {filtered.length === 0 && !allowCustomValue && (
                    <div className="overflow-hidden text-ellipsis py-1 pl-4 text-content-primary">
                      No options matching “{query}”.
                    </div>
                  )}
                </div>
              </HeadlessCombobox.Options>
            )}
          </div>
        </>
      )}
    </HeadlessCombobox>
  );
}
