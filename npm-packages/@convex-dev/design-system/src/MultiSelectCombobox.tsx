import {
  CheckIcon,
  ChevronDownIcon,
  MagnifyingGlassIcon,
} from "@radix-ui/react-icons";
import {
  Combobox as HeadlessCombobox,
  ComboboxButton as HeadlessComboboxButton,
  ComboboxOptions as HeadlessComboboxOptions,
  ComboboxOption as HeadlessComboboxOption,
  ComboboxInput as HeadlessComboboxInput,
  Label,
} from "@headlessui/react";
import React, { useRef, useState, useEffect } from "react";
import classNames from "classnames";
import { cn } from "@ui/cn";
import { useHoverDirty } from "react-use";
import { test } from "fuzzy";
import { Button } from "@ui/Button";
import { createPortal } from "react-dom";
import { usePopper } from "react-popper";

const MAX_DISPLAYED_OPTIONS = 100;

export type MultiSelectValue = string[] | "all";

export function MultiSelectCombobox({
  options,
  selectedOptions,
  setSelectedOptions,
  unit,
  unitPlural,
  label,
  labelHidden = false,
  Option,
  disableSearch = false,
  processFilterOption = (option) => option,
}: {
  options: string[];
  selectedOptions: MultiSelectValue;
  setSelectedOptions(newValue: MultiSelectValue): void;
  unit: string;
  unitPlural: string;
  label: string;
  labelHidden?: boolean;
  Option?: React.ComponentType<{ label: string; inButton: boolean }>;
  disableSearch?: boolean;
  processFilterOption?: (option: string) => string;
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
            offset: [0, 4],
          },
        },
      ],
    },
  );

  // Get the width for the dropdown
  const getOptionsWidth = () => {
    if (!referenceElement) return undefined;
    return `${referenceElement.offsetWidth}px`;
  };

  const filteredOptions =
    query === ""
      ? options
      : options.filter((option) => test(query, processFilterOption(option)));

  const hasMoreThanMax = filteredOptions.length > MAX_DISPLAYED_OPTIONS;
  const displayedOptions = hasMoreThanMax
    ? filteredOptions.slice(0, MAX_DISPLAYED_OPTIONS)
    : filteredOptions;

  // Convert to internal array representation for Combobox
  const selectedArray = selectedOptions === "all" ? options : selectedOptions;

  const count =
    selectedOptions === "all"
      ? options.length
      : selectedOptions.filter((name) => name !== "_other").length;

  const displayValue =
    selectedOptions === "all"
      ? `All ${unitPlural}`
      : `${count} ${count !== 1 ? unitPlural : unit}`;

  // Update popper position when dropdown opens
  useEffect(() => {
    if (isOpen && update) {
      void update();
    }
  }, [isOpen, update]);

  const handleSelectAll = () => {
    if (selectedOptions === "all") {
      setSelectedOptions([]);
    } else {
      setSelectedOptions("all");
    }
  };

  return (
    <HeadlessCombobox
      value={selectedArray}
      onChange={(newSelection) => {
        // Check if all options are selected and convert to "all" state
        if (newSelection.length === options.length) {
          setSelectedOptions("all");
        } else {
          setSelectedOptions(newSelection);
        }
      }}
      multiple
    >
      {({ open }) => {
        // Update isOpen state when open changes
        if (open !== isOpen) {
          setIsOpen(open);
        }

        return (
          <>
            <Label
              className={classNames(
                "flex gap-1 text-sm font-semibold",
                labelHidden ? "hidden" : "mb-2",
              )}
              hidden={labelHidden}
            >
              {label}
            </Label>

            <div className="relative">
              <div
                ref={setReferenceElement}
                className={cn("relative flex items-center")}
              >
                <HeadlessComboboxButton
                  className={classNames(
                    "flex gap-2 w-full justify-between",
                    "truncate relative rounded-md py-1.5 px-1.5 text-left text-sm text-content-primary disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-background-secondary",
                    "border",
                    "focus:border-border-selected focus:outline-hidden bg-background-secondary hover:bg-background-tertiary",
                    open && "border-border-selected",
                  )}
                >
                  {displayValue}
                  <ChevronDownIcon
                    className={cn(
                      "relative z-30 -ml-6 h-5 w-5 text-content-primary transition-all",
                      open && "rotate-180",
                    )}
                  />
                </HeadlessComboboxButton>
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
                    <HeadlessComboboxOptions
                      modal={false}
                      static
                      className="scrollbar max-h-60 w-fit max-w-80 min-w-full overflow-auto rounded-md border bg-background-secondary pb-1 text-xs shadow-sm focus:outline-hidden"
                    >
                      <div className="min-w-fit">
                        {!disableSearch && (
                          <div className="sticky top-0 left-0 z-20 flex w-full items-center gap-1 border-b bg-background-secondary px-2 pt-1">
                            <MagnifyingGlassIcon className="h-4 w-4 text-content-secondary" />
                            <HeadlessComboboxInput
                              onChange={(event) => setQuery(event.target.value)}
                              value={query}
                              autoFocus
                              placeholder={`Search ${unitPlural}...`}
                              className={classNames(
                                "placeholder:text-content-tertiary relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                                "focus:outline-hidden bg-background-secondary",
                              )}
                            />
                          </div>
                        )}
                        {/* eslint-disable-next-line react/forbid-elements */}
                        <button
                          type="button"
                          className="w-full cursor-pointer p-2 pl-7 text-left text-content-primary hover:bg-background-tertiary"
                          onClick={handleSelectAll}
                        >
                          {selectedOptions === "all"
                            ? "Deselect all"
                            : "Select all"}
                        </button>

                        {displayedOptions.map((option) => (
                          <ComboboxOption
                            key={option}
                            value={option}
                            label={
                              Option ? (
                                <Option label={option} inButton={false} />
                              ) : (
                                option
                              )
                            }
                            onOnly={() => {
                              setSelectedOptions([option]);
                            }}
                          />
                        ))}

                        {hasMoreThanMax && (
                          <div className="w-fit min-w-full cursor-default px-2 py-1.5 text-content-tertiary select-none">
                            Too many items to display, use the searchbar to
                            filter {unitPlural}.
                          </div>
                        )}
                      </div>
                    </HeadlessComboboxOptions>
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

function ComboboxOption({
  value,
  label,
  onOnly,
}: {
  value: string;
  label: React.ReactNode | string;
  onOnly: () => void;
}) {
  const onlyRefs = useRef(null);
  const isHoveringOnly = useHoverDirty(onlyRefs);
  return (
    <HeadlessComboboxOption
      value={value}
      className={({ focus }) =>
        classNames(
          "w-fit min-w-full flex gap-1 cursor-pointer select-none p-2 text-content-primary group",
          focus && "bg-background-tertiary",
        )
      }
      disabled={isHoveringOnly}
    >
      {({ selected }) => (
        <>
          {selected ? (
            <CheckIcon
              className="h-4 min-w-[1rem] text-neutral-7 dark:text-neutral-4"
              aria-hidden="true"
            />
          ) : (
            <span className="min-w-[1rem]" />
          )}
          <span
            className={classNames(
              "flex gap-2 w-full whitespace-nowrap",
              selected && "font-semibold",
            )}
          >
            {label}
            <Button
              ref={onlyRefs}
              className="invisible text-xs font-normal text-content-secondary group-hover:visible hover:underline"
              variant="unstyled"
              onClick={onOnly}
            >
              only
            </Button>
          </span>
        </>
      )}
    </HeadlessComboboxOption>
  );
}
