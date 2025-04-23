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

  // Keeps track of the whether we should prevent the dropdown from closing
  // This is used to prevent the dropdown from closing when clicking on a link or tooltip
  const [preventClose, setPreventClose] = useState(false);

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

  const selectedOptionData = options.find((o) =>
    isEqual(selectedOption, o.value),
  );

  // Update popper position when dropdown opens
  useEffect(() => {
    if (isOpen && update) {
      void update();
    }
  }, [isOpen, update]);

  // Reset preventClose after a short delay
  useEffect(() => {
    if (preventClose) {
      const timer = setTimeout(() => {
        setPreventClose(false);
      }, 200);

      return () => clearTimeout(timer);
    }
  }, [preventClose]);

  return (
    <HeadlessCombobox
      value={
        options.find((o) => isEqual(selectedOption, o.value))?.value || null
      }
      onChange={(option) => {
        if (!preventClose) {
          setSelectedOption(option);
          setQuery("");
        }
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
              className="text-content-primary text-left text-sm"
            >
              {label}
            </HeadlessCombobox.Label>
            <div className={cn("relative", className)}>
              <div
                ref={setReferenceElement}
                className={cn("relative flex items-center w-60", buttonClasses)}
              >
                <HeadlessCombobox.Button
                  as={Button}
                  variant="unstyled"
                  data-testid={`combobox-button-${label}`}
                  className={cn(
                    "flex gap-1 w-full items-center group",
                    "truncate relative text-left text-content-primary rounded-md disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                    "border focus-visible:z-10 focus-visible:border-border-selected focus-visible:outline-none bg-background-secondary text-sm",
                    "hover:bg-background-tertiary",
                    "cursor-pointer",
                    open && "border-border-selected z-10",
                    size === "sm" && "py-1 px-1.5 text-xs",
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
                        "text-content-primary ml-auto size-4 transition-all",
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
                        "mt-1 max-h-[14.75rem] overflow-auto rounded-md bg-background-secondary pb-1 text-xs shadow scrollbar border",
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
                          <div className="bg-background-secondary sticky top-0 z-10 flex w-full items-center gap-2 border-b px-3 pt-1">
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
                            disabled={option.disabled}
                            className={({ active }) =>
                              cn(
                                "w-fit min-w-full relative cursor-pointer select-none py-1.5 px-3 text-content-primary",
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
                                onPointerDownCapture={(e) => {
                                  // Stop propagation if clicking on a link or tooltip
                                  if (e.target instanceof HTMLElement) {
                                    const closest = e.target.closest(
                                      'a, [role="tooltip"]',
                                    );
                                    if (closest) {
                                      e.stopPropagation();
                                      e.preventDefault();

                                      // Set preventClose to true
                                      setPreventClose(true);
                                    }
                                  }
                                }}
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
                          <div className="text-content-primary overflow-hidden text-ellipsis py-1 pl-4">
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
