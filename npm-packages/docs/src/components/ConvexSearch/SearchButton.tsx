import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import React from "react";

type Props = {
  onClick: () => void;
};

function SearchButton({ onClick }: Props) {
  const isMac = /Mac|iPhone|iPad/.test(window.navigator.userAgent);

  return (
    <button
      className="flex items-center gap-1 px-1 py-0 font-sans text-sm transition-colors border-2 border-transparent border-solid rounded-lg dark:hover:bg-neutral-n12 hover:border-neutral-n8 focus:border-neutral-n8 bg-neutral-n2 hover:bg-neutral-white dark:bg-neutral-n11 h-11 hover:cursor-pointer"
      onClick={onClick}
      aria-label="Search"
    >
      <MagnifyingGlassIcon className="w-6 text-neutral-n9 dark:text-neutral-n6 shrink-0 h-6" />
      <span className="hidden mr-1 sm:block text-neutral-n11 dark:text-neutral-n6 whitespace-nowrap">
        Search docs and more...
      </span>

      <span className="items-center hidden p-1 text-xs rounded-md md:flex aspect-square bg-neutral-n4 dark:bg-neutral-n9 text-neutral-n8 dark:text-neutral-n6">
        {isMac ? "⌘" : "^"}K
      </span>
    </button>
  );
}

export default SearchButton;
