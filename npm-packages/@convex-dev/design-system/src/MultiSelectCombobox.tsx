import {
  CheckIcon,
  ChevronDownIcon,
  MagnifyingGlassIcon,
} from "@radix-ui/react-icons";
import { Combobox } from "@headlessui/react";
import React, { useRef, useState, useEffect } from "react";
import classNames from "classnames";
import { cn } from "@ui/cn";
import { useHoverDirty } from "react-use";
import { test } from "fuzzy";
import { Button } from "@ui/Button";
import { createPortal } from "react-dom";
import { usePopper } from "react-popper";

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
  selectedOptions: string[];
  setSelectedOptions(newValue: string[]): void;
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

  const count = selectedOptions.filter((name) => name !== "_other").length;
  const displayValue =
    selectedOptions.length === options.length
      ? `All ${unitPlural}`
      : `${count} ${count !== 1 ? unitPlural : unit}`;

  // Update popper position when dropdown opens
  useEffect(() => {
    if (isOpen && update) {
      void update();
    }
  }, [isOpen, update]);

  return (
    <Combobox
      as="div"
      value={selectedOptions}
      onChange={setSelectedOptions}
      multiple
    >
      {({ open }) => {
        // Update isOpen state when open changes
        if (open !== isOpen) {
          setIsOpen(open);
        }

        return (
          <>
            <Combobox.Label
              className={classNames(
                "flex gap-1 text-sm font-semibold",
                labelHidden ? "hidden" : "mb-2",
              )}
              hidden={labelHidden}
            >
              {label}
            </Combobox.Label>

            <div className="relative">
              <div
                ref={setReferenceElement}
                className={cn("relative flex items-center")}
              >
                <Combobox.Button
                  className={classNames(
                    "flex gap-2 w-full justify-between",
                    "truncate relative rounded-md py-1.5 px-1.5 text-left text-sm text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                    "border",
                    "focus:border-border-selected focus:outline-none bg-background-secondary hover:bg-background-tertiary",
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
                </Combobox.Button>
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
                    <Combobox.Options
                      static
                      className="max-h-60 w-fit min-w-full max-w-80 overflow-auto rounded-md border bg-background-secondary pb-1 text-xs shadow scrollbar focus:outline-none"
                    >
                      <div className="min-w-fit">
                        {!disableSearch && (
                          <div className="sticky left-0 top-0 z-20 flex w-full items-center gap-1 border-b bg-background-secondary px-2 pt-1">
                            <MagnifyingGlassIcon className="h-4 w-4 text-content-secondary" />
                            <Combobox.Input
                              onChange={(event) => setQuery(event.target.value)}
                              value={query}
                              placeholder={`Search ${unitPlural}...`}
                              className={classNames(
                                "placeholder:text-content-tertiary relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                                "focus:outline-none bg-background-secondary",
                              )}
                            />
                          </div>
                        )}
                        {/* eslint-disable-next-line react/forbid-elements */}
                        <button
                          type="button"
                          className="w-full cursor-pointer p-2 pl-7 text-left text-content-primary hover:bg-background-tertiary"
                          onClick={() =>
                            setSelectedOptions(
                              options.length === selectedOptions.length
                                ? []
                                : [...options],
                            )
                          }
                        >
                          {options.length === selectedOptions.length
                            ? "Deselect all"
                            : "Select all"}
                        </button>

                        {filteredOptions.map((option) => (
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
                      </div>
                    </Combobox.Options>
                  </div>,
                  document.body,
                )}
            </div>
          </>
        );
      }}
    </Combobox>
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
    <Combobox.Option
      value={value}
      className={({ active }) =>
        classNames(
          "w-fit min-w-full flex gap-1 cursor-pointer select-none p-2 text-content-primary group",
          active && "bg-background-tertiary",
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
    </Combobox.Option>
  );
}
