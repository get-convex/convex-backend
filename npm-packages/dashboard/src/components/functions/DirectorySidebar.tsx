import {
  useFunctionSearchTerm,
  useRootEntries,
  NentSwitcher,
} from "dashboard-common";
import { ChangeEvent, useRef } from "react";
import classNames from "classnames";
import { FileTree } from "./FileTree";

export function DirectorySidebar({
  onChangeFunction,
}: {
  onChangeFunction: () => void;
}) {
  const [searchTerm, setSearchTerm] = useFunctionSearchTerm();
  const rootEntries = useRootEntries();
  const onChange = (input: ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(input.currentTarget.value);
  };

  const ref = useRef<HTMLDivElement>(null);

  return (
    <div
      className={classNames(
        "flex h-full w-full pt-4 flex-col bg-background-secondary overflow-x-hidden scrollbar mb-2",
      )}
      ref={ref}
    >
      <div className="flex flex-col px-3">
        <NentSwitcher />
        <h5>Function Explorer</h5>
      </div>
      <input
        id="Search functions"
        placeholder="Search functions..."
        onChange={onChange}
        defaultValue={searchTerm}
        type="search"
        className={classNames(
          "placeholder:text-content-tertiary truncate relative w-full py-1.5 text-left text-xs text-content-primary disabled:bg-background-tertiary disabled:text-content-secondary disabled:cursor-not-allowed",
          "focus:outline-none bg-background-secondary font-normal border-b px-3",
        )}
      />
      <div className="w-full overflow-x-hidden pt-1 scrollbar">
        <FileTree
          tree={rootEntries}
          onChangeFunction={onChangeFunction}
          defaultOpen
          nestingLevel={0}
        />
      </div>
    </div>
  );
}
