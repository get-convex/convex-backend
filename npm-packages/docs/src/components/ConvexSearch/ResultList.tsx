import { cn } from "@site/src/lib/cn";
import React, { useEffect, useRef, useState } from "react";
import { Result } from "./types";

const labelForUrl = (url: string) => {
  if (url.includes("stack.convex.dev")) {
    return "STACK";
  }
  if (url.includes("discord.com")) {
    return "DISCORD";
  }
  return "DOCS";
};

interface ResultListProps {
  results: Result[];
}

export default function ResultList({ results }: ResultListProps) {
  const [selectedResult, setSelectedResult] = useState(0);
  const [usingKeyboard, setUsingKeyboard] = useState(false);
  const listRef = useRef<HTMLUListElement>(null);
  const selectedResultRef = useRef<HTMLLIElement>(null);

  // Use the up and down arrow keys to navigate the results.
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "ArrowUp") {
        event.preventDefault();
        if (selectedResult > 0) {
          setSelectedResult(selectedResult - 1);
        }
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        if (selectedResult !== null && selectedResult < results.length - 1) {
          setSelectedResult(selectedResult + 1);
        }
      }
      setUsingKeyboard(true);
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [selectedResult, results]);

  // Scroll to the selected hit using the ref when it changes
  useEffect(() => {
    if (usingKeyboard && selectedResultRef.current) {
      selectedResultRef.current.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  }, [selectedResult, usingKeyboard]);

  // Whenever the hits change, select the first one.
  useEffect(() => {
    setSelectedResult(0);
  }, [results]);

  // Detect mouse movement to switch modes back.
  useEffect(() => {
    const handleMouseMove = () => {
      setUsingKeyboard(false);
    };

    document.addEventListener("mousemove", handleMouseMove);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
    };
  }, []);

  return (
    <ul
      className="flex flex-col list-none p-0! gap-1 m-0"
      role="list"
      ref={listRef}
    >
      {results.map((result, index) => (
        <li
          key={result.url}
          ref={(element) => {
            if (index === selectedResult) {
              selectedResultRef.current = element;
            }
          }}
          role="listitem"
          // This is referenced by SearchBox.
          aria-selected={index === selectedResult}
          data-hit-index={index}
          className={cn(
            "js-hitList-item border-2 border-solid border-transparent p-2 rounded-md overflow-hidden transition-all",
            {
              "border-plum-p4/50 shadow-sm bg-neutral-white dark:bg-neutral-n12 dark:border-plum-p3/80":
                index === selectedResult,
            },
          )}
          onMouseEnter={() => {
            if (!usingKeyboard) {
              setSelectedResult(index);
            }
          }}
        >
          <a
            href={result.url}
            className="text-neutral-n10! flex gap-4 items-center hover:no-underline hover:text-neutral-n10! w-full dark:text-neutral-n2!"
          >
            <div className="flex flex-col grow min-w-0">
              <div className="font-bold text-sm">{result.title}</div>
              {result.subtext && (
                <div className="text-sm text-neutral-n7 overflow-hidden whitespace-nowrap text-ellipsis min-w-0">
                  {result.subtext}
                </div>
              )}
            </div>
            <span className="text-xs text-neutral-n8 font-semibold bg-neutral-n2 rounded-sm shrink-0 px-2 py-1">
              {labelForUrl(result.url)}
            </span>
          </a>
        </li>
      ))}
    </ul>
  );
}
