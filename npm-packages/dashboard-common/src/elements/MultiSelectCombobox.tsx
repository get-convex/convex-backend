import {
  CheckIcon,
  CaretSortIcon,
  MagnifyingGlassIcon,
} from "@radix-ui/react-icons";
import { Combobox } from "@headlessui/react";
import React, { useRef, useState } from "react";
import classNames from "classnames";
import { cn } from "lib/cn";
import { useHoverDirty } from "react-use";
import { test } from "fuzzy";
import { Button } from "./Button";

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

  const filteredOptions =
    query === ""
      ? options
      : options.filter((option) => test(query, processFilterOption(option)));

  const count = selectedOptions.filter((name) => name !== "_other").length;
  const displayValue =
    selectedOptions.length === options.length
      ? `All ${unitPlural}`
      : `${count} ${count !== 1 ? unitPlural : unit}`;

  return (
    <Combobox
      as="div"
      value={selectedOptions}
      onChange={setSelectedOptions}
      multiple
    >
      {({ open }) => (
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
            <div className={cn("relative flex items-center")}>
              <Combobox.Button
                className={classNames(
                  "flex gap-2 w-full justify-between",
                  "truncate relative rounded py-2 px-3 text-left text-sm text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
                  "border",
                  "focus:border-border-selected focus:outline-none bg-background-secondary hover:bg-background-tertiary",
                  open && "border-border-selected",
                )}
              >
                {displayValue}
                <CaretSortIcon className="relative z-30 -ml-6 h-5 w-5 text-content-primary" />
              </Combobox.Button>
            </div>

            <Combobox.Options className="absolute z-50 max-h-60 min-w-full max-w-[20rem] overflow-auto rounded border bg-background-secondary pb-1 text-xs shadow scrollbar focus:outline-none">
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
          </div>
        </>
      )}
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
