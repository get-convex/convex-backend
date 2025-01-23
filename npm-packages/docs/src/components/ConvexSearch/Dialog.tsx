import { cn } from "@site/src/lib/cn";
import React, { useEffect, useState, useCallback } from "react";
import ReactDOM from "react-dom";
import AskAI from "./AskAI";
import KeyboardLegend from "./KeyboardLegend";
import Results from "./Results";
import SearchBox from "./SearchBox";

type Props = {
  open: boolean;
  onClose: () => void;
};

const Dialog = ({ open, onClose }: Props) => {
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [container] = useState(() => document.createElement("div"));

  // Debounce the query to reduce search requests.
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      setDebouncedQuery(query);
    }, 250);

    return () => {
      clearTimeout(timeoutId);
    };
  }, [query]);

  // Append the container to the body, outside of the current component tree.
  useEffect(() => {
    document.body.appendChild(container);
    return () => {
      document.body.removeChild(container);
    };
  }, [container]);

  // Toggle scrolling on the body to avoid scrolling away from the dialog.
  useEffect(() => {
    document.body.style.overflow = open ? "hidden" : "auto";
  }, [open]);

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setQuery(value);
  };

  const handleClear = () => {
    setQuery("");
  };

  const handleClose = useCallback(() => {
    setQuery("");
    onClose();
  }, [onClose]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        handleClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleClose]);

  const dialogContent = open && (
    <div className="h-screen left-0 top-0 overflow-y-auto fixed w-screen z-[1000] md:flex md:justify-center">
      <div
        className={cn(
          "bg-gradient-to-b from-neutral-white to-neutral-n2 h-full w-full p-4 flex flex-col gap-4 md:z-[1001] md:rounded-lg md:shadow-lg md:h-[calc(min(80vh,60rem))] md:mt-20 md:w-[45rem] dark:from-neutral-n11 dark:to-neutral-n13",
          {
            "md:h-20": query === "",
          },
        )}
      >
        <div className="flex gap-2">
          <SearchBox
            className="grow"
            value={query}
            onChange={handleChange}
            onClear={handleClear}
          />
          <button
            className="border-none bg-transparent font-sans cursor-pointer md:hidden"
            onClick={handleClose}
          >
            Cancel
          </button>
        </div>
        {query !== "" && (
          <>
            <AskAI onClick={handleClose} query={query} />
            <Results query={debouncedQuery} />
            <KeyboardLegend />
          </>
        )}
      </div>
      <button
        className="hidden bg-neutral-n12/50 dark:bg-neutral-n12/80 inset-0 absolute border-none p-0 w-screen h-screen backdrop-blur-sm md:block"
        aria-label="Close search"
        onClick={handleClose}
      />
    </div>
  );

  return ReactDOM.createPortal(dialogContent, container);
};

export default Dialog;
