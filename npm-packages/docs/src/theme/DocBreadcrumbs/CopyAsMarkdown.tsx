import { CheckIcon, CopyIcon } from "@radix-ui/react-icons";
import clsx from "clsx";
import React, { useCallback, useState, useRef, useEffect } from "react";
import { toast } from "sonner";
import { useLocation } from "@docusaurus/router";

export function CopyAsMarkdown() {
  const [isCopied, setIsCopied] = useState(false);
  const [hasBeenCopied, setHasBeenCopied] = useState(false);
  const [markdownContent, setMarkdownContent] = useState<string | null>(null);
  const copyTimeout = useRef<number | undefined>(undefined);

  const location = useLocation();
  const pathWithoutTrailingSlash = location.pathname.endsWith("/")
    ? location.pathname.slice(0, -1)
    : location.pathname;
  const markdownUrl =
    pathWithoutTrailingSlash === "/home"
      ? "/llms.txt"
      : `${pathWithoutTrailingSlash}.md`;

  useEffect(() => {
    const fetchMarkdown = async () => {
      setMarkdownContent(null);

      try {
        const response = await fetch(markdownUrl);

        if (!response.ok) {
          throw new Error(
            `Received non-OK Markdown response on ${markdownUrl}: ${response.status}`,
          );
        }

        const content = await response.text();
        setMarkdownContent(content);
      } catch (error) {
        console.error("Failed to fetch page as Markdown:", error);
      }
    };

    void fetchMarkdown();
  }, [markdownUrl]);

  const handleCopyAsMarkdown = useCallback(async () => {
    if (markdownContent === null) return;

    try {
      await navigator.clipboard.writeText(markdownContent);

      setIsCopied(true);
      setHasBeenCopied(true);
      copyTimeout.current = window.setTimeout(() => {
        setIsCopied(false);
      }, 2000);
    } catch (error) {
      toast.error("Can’t write to clipboard.");
      console.error("Failed to copy as Markdown:", error);
    }
  }, [markdownContent]);

  return (
    <button
      type="button"
      className="font-[inherit] appearance-none bg-transparent border p-0 border-(--convex-breadcrumb-font-color)/50 hover:border-(--convex-breadcrumb-font-color)/80 rounded text-(--convex-breadcrumb-font-color) transition-colors cursor-pointer disabled:opacity-0 focus-visible:ring-2 focus-visible:ring-blue-500 focus:outline-none overflow-hidden relative"
      onClick={handleCopyAsMarkdown}
      disabled={markdownContent === null}
      {...(markdownContent === null && { inert: "inert" })}
    >
      <div
        className={clsx(
          "px-2 py-1 flex gap-1.5 items-center",
          hasBeenCopied && !isCopied && "animate-slideToTop",
          isCopied && "opacity-0",
        )}
        {...(isCopied && { inert: "inert" })}
      >
        <CopyIcon />
        Copy as Markdown
      </div>
      <div
        className={clsx(
          "absolute inset-0 px-2 py-1 flex gap-1.5 items-center",
          isCopied && "animate-slideToTop",
          !isCopied && "opacity-0",
        )}
        {...(!isCopied && { inert: "inert" })}
      >
        <CheckIcon className="text-green-g3" />
        Copied!
      </div>
    </button>
  );
}
