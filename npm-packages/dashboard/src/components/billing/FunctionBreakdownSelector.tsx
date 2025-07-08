import { Listbox, Transition } from "@headlessui/react";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { Fragment } from "react";

const OPTIONS = [
  <>Function Calls</>,
  <>Database Bandwidth</>,
  <>Action Compute</>,
  <>Vector Bandwidth</>,
];

export function FunctionBreakdownSelector({
  value,
  onChange,
}: {
  value: number;
  onChange: (newValue: number) => void;
}) {
  const optionClass = ({ active }: { active: boolean }) =>
    classNames(
      "cursor-pointer w-full items-center rounded-sm p-2 text-left text-sm text-content-primary transition",
      active && "bg-background-tertiary",
    );

  return (
    <div className="relative">
      <Listbox value={value ?? ""} onChange={onChange}>
        <Listbox.Button
          className={classNames(
            "relative h-full w-full appearance-none rounded-sm border",
            "bg-background-secondary py-2 pl-3 pr-10 text-left text-sm font-medium text-content-primary hover:bg-background-tertiary focus:outline-hidden focus-visible:ring-2 focus-visible:ring-background-secondary/75 focus-visible:ring-offset-2 focus-visible:ring-offset-content-accent",
          )}
        >
          {OPTIONS[value]}
        </Listbox.Button>

        <Transition
          as={Fragment}
          leave="transition ease-in duration-100"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <Listbox.Options className="absolute right-0 z-50 mt-2 max-h-60 w-full min-w-[256px] overflow-auto rounded-sm border bg-background-secondary px-3 py-4 shadow-sm">
            {OPTIONS.map((option, index) => (
              <Listbox.Option className={optionClass} key={index} value={index}>
                {option}
              </Listbox.Option>
            ))}
          </Listbox.Options>
        </Transition>
      </Listbox>

      <div className="pointer-events-none absolute right-0 top-0 z-50 flex h-full place-items-center pr-2">
        <ChevronDownIcon
          className="h-5 w-5 text-content-tertiary"
          aria-hidden="true"
        />
      </div>
    </div>
  );
}
