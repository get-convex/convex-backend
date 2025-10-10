import { useState, useEffect } from "react";
import { Combobox as HeadlessCombobox } from "@headlessui/react";
import { ChevronDownIcon, MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import { isEqual } from "lodash-es";
import fuzzy from "fuzzy";
import { Button, ButtonProps } from "@ui/Button";
import { createPortal } from "react-dom";
import { usePopper } from "react-popper";

const { test } = fuzzy;

const MAX_DISPLAYED_OPTIONS = 100;

export type Option<T> = { label: string; value: T; disabled?: boolean };

export function Combobox<T>({
  options,
  optionsHeader,
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
  size = "md",
  icon,
}: {
  label: React.ReactNode;
  labelHidden?: boolean;
  className?: string;
  optionsHeader?: React.ReactNode;
  options: Readonly<Option<T>[]>;
  placeholder?: string;
  searchPlaceholder?: string;
  disableSearch?: boolean;
  // "full" only works if the options dropdown
  // fits inside of the ComboBox's ancestor elements,
  // or if the ancestors allow overflow.
  optionsWidth?: "full" | "fixed" | "fit";
  selectedOption?: T | null;
  setSelectedOption: (option: T | null) => void;
  buttonClasses?: string;
  buttonProps?: Omit<ButtonProps, "href">;
  innerButtonClasses?: string;
  allowCustomValue?: boolean;
  Option?: React.ComponentType<{
    label: string;
    value: T;
    inButton: boolean;
    disabled?: boolean;
  }>;
  disabled?: boolean;
  unknownLabel?: (value: T) => string;
  processFilterOption?: (option: string) => string;
  size?: "sm" | "md";
  icon?: React.ReactNode;
}) {
  const [query, setQuery] = useState("");
  const [referenceElement, setReferenceElement] =
    useState<HTMLDivElement | null>(null);
  const [popperElement, setPopperElement] = useState<HTMLDivElement | null>(
    null,
  );

  // Force tabindex to 0
  useEffect(() => {
    if (referenceElement?.children[0]) {
      (referenceElement.children[0] as HTMLElement).tabIndex = 0;
    }
  }, [referenceElement]);

  const [isOpen, setIsOpen] = useState(false);

  const { styles, attributes, update } = usePopper(
    referenceElement,
    popperElement,
    {
      placement: "bottom-start",
      modifiers: [
        {
          name: "offset",
          options: {
            offset: [0, 4], // x, y offset in pixels
          },
        },
      ],
    },
  );

  // Calculate width based on optionsWidth prop
  const getOptionsWidth = () => {
    if (!referenceElement) return undefined;

    if (optionsWidth === "full") {
      return `${referenceElement.offsetWidth}px`;
    }
    if (optionsWidth === "fixed") {
      return "240px";
    }
    return undefined; // auto width for "fit"
  };

  const filtered =
    query === ""
      ? options
      : options.filter((option) =>
          test(query, processFilterOption(option.label)),
        );

  const hasMoreThanMaxOptions = filtered.length > MAX_DISPLAYED_OPTIONS;
  const displayedOptions = hasMoreThanMaxOptions
    ? filtered.slice(0, MAX_DISPLAYED_OPTIONS)
    : filtered;

  const selectedOptionData = options.find((o) =>
    isEqual(selectedOption, o.value),
  );

  // Update popper position when dropdown opens
  useEffect(() => {
    if (isOpen && update) {
      void update();
    }
  }, [isOpen, update]);

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
      {({ open }) => {
        // Update isOpen state when open changes
        // This effect runs on every render, but we only need to update
        // isOpen when open changes, so it's safe to call here
        if (open !== isOpen) {
          setIsOpen(open);
        }

        return (
          <>
            <HeadlessCombobox.Label
              hidden={labelHidden}
              className="text-left text-sm text-content-primary"
            >
              {label}
            </HeadlessCombobox.Label>
            <div className={cn("relative", className)}>
              <div
                ref={setReferenceElement}
                className={cn("relative flex w-60 items-center", buttonClasses)}
              >
                <HeadlessCombobox.Button
                  as={Button}
                  variant="unstyled"
                  data-testid={`combobox-button-${label}`}
                  className={cn(
                    "group flex w-full items-center gap-1",
                    "relative truncate rounded-md text-left text-content-primary disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-background-secondary",
                    "border bg-background-secondary text-sm focus-visible:z-10 focus-visible:border-border-selected focus-visible:outline-hidden",
                    "hover:bg-background-tertiary",
                    "cursor-pointer",
                    open && "z-10 border-border-selected",
                    size === "sm" && "px-1.5 py-1 text-xs",
                    size === "md" && "p-1.5",
                    innerButtonClasses,
                  )}
                  {...buttonProps}
                >
                  {icon}
                  <div className="truncate">
                    {!!Option && !!selectedOptionData ? (
                      <Option
                        inButton
                        label={selectedOptionData.label}
                        value={selectedOptionData.value}
                        disabled={selectedOptionData.disabled}
                      />
                    ) : (
                      selectedOptionData?.label || (
                        <span className="text-content-tertiary">
                          {selectedOption && unknownLabel(selectedOption)}
                        </span>
                      )
                    )}
                    {!selectedOptionData && (
                      <span className="text-content-tertiary">
                        {placeholder}
                      </span>
                    )}
                  </div>
                  {size === "md" && (
                    <ChevronDownIcon
                      className={cn(
                        "ml-auto size-4 text-content-primary transition-all",
                        open && "rotate-180",
                      )}
                    />
                  )}
                </HeadlessCombobox.Button>
              </div>
              {open &&
                createPortal(
                  <div
                    ref={setPopperElement}
                    style={{
                      ...styles.popper,
                      width: getOptionsWidth(),
                    }}
                    {...attributes.popper}
                    className="z-50"
                  >
                    <HeadlessCombobox.Options
                      static
                      className={cn(
                        "mt-1 scrollbar max-h-[14.75rem] overflow-auto rounded-md border bg-background-secondary pb-1 text-xs shadow-sm",
                      )}
                      ref={(el) => {
                        el && "scrollTo" in el && el.scrollTo(0, 0);
                      }}
                    >
                      {optionsHeader && (
                        <div className="border-b p-1 pb-2">{optionsHeader}</div>
                      )}
                      <div className="min-w-fit">
                        {!disableSearch && (
                          <div className="sticky top-0 z-10 flex w-full items-center gap-2 border-b bg-background-secondary px-3 pt-1">
                            <MagnifyingGlassIcon className="text-content-secondary" />
                            <HeadlessCombobox.Input
                              onChange={(event) => setQuery(event.target.value)}
                              value={query}
                              autoFocus
                              className={cn(
                                "relative w-full truncate py-1.5 text-left text-xs text-content-primary placeholder:text-content-tertiary disabled:cursor-not-allowed disabled:bg-background-tertiary disabled:text-content-secondary",
                                "bg-background-secondary focus:outline-hidden",
                              )}
                              placeholder={searchPlaceholder}
                            />
                          </div>
                        )}
                        {displayedOptions.map((option, idx) => (
                          <HeadlessCombobox.Option
                            key={idx}
                            value={option.value}
                            disabled={option.disabled}
                            className={({ active }) =>
                              cn(
                                "relative w-fit min-w-full cursor-pointer px-3 py-1.5 text-content-primary select-none",
                                active && "bg-background-tertiary",
                                option.disabled &&
                                  "cursor-not-allowed text-content-secondary opacity-75",
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

                        {hasMoreThanMaxOptions && (
                          <div className="relative w-fit min-w-full cursor-default px-3 py-1.5 text-content-tertiary select-none">
                            Too many options to display, use the searchbar to
                            filter.
                          </div>
                        )}

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
                          <div className="overflow-hidden py-1 pl-4 text-ellipsis text-content-primary">
                            No options matching "{query}".
                          </div>
                        )}
                      </div>
                    </HeadlessCombobox.Options>
                  </div>,
                  document.body,
                )}
            </div>
          </>
        );
      }}
    </HeadlessCombobox>
  );
}
