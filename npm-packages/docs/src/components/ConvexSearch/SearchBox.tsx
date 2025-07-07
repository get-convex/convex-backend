import React, { useEffect, useRef } from "react";
import { cn } from "@site/src/lib/cn";
import { MagnifyingGlassIcon, Cross1Icon } from "@radix-ui/react-icons";

interface SearchBoxProps {
  value: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  onClear: () => void;
  className?: string;
}

export default function SearchBox({
  value,
  onChange,
  onClear,
  className,
}: SearchBoxProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  const focusInput = () => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  const handleClear = () => {
    onClear();
    focusInput();
  };

  // Focus the search input when the component mounts.
  useEffect(() => {
    focusInput();
  }, []);

  // Navigate to the selected hit when pressing enter.
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Enter") {
        const linkElement = document.querySelector(
          '.js-hitList-item[aria-selected="true"] a',
        );
        if (linkElement) {
          const url = linkElement.getAttribute("href");
          const target = linkElement.getAttribute("target") || "_self";
          window.open(url, target);
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  return (
    <div
      className={cn(
        "border-2 border-solid border-neutral-n7 rounded-md flex p-2 justify-center gap-1 bg-neutral-white dark:bg-neutral-n12 dark:border-neutral-n9",
        className,
      )}
    >
      <MagnifyingGlassIcon className="h-7 w-7 text-plum-p4 dark:text-plum-p3" />
      <input
        className="bg-transparent border-none grow text-lg font-sans text-neutral-n11 focus:outline-hidden dark:text-neutral-n2"
        type="text"
        placeholder="Search across Docs, Stack, and Discord..."
        value={value}
        onChange={onChange}
        ref={inputRef}
      />
      {value !== "" && (
        <button
          className="border-none bg-transparent py-0 px-1 flex items-center cursor-pointer"
          onClick={handleClear}
          aria-label="Clear search"
        >
          <Cross1Icon className="h-5 w-5 text-neutral-n9 dark:text-neutral-n6" />
        </button>
      )}
    </div>
  );
}
