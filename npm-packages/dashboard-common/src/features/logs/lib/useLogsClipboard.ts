import { useEffect } from "react";
import { InterleavedLog, getLogKey } from "@common/features/logs/lib/interleaveLogs";
import { formatInterleavedLogToString } from "@common/features/logs/lib/formatLog";

/**
 * A sophisticated hook to handle clipboard operations for logs.
 * Supports "Clean Copy" for multiple selections by mapping DOM nodes back to log data.
 */
export function useLogsClipboard(
  interleavedLogs: InterleavedLog[],
  containerRef: React.RefObject<HTMLDivElement>,
) {
  useEffect(() => {
    const handleCopy = (e: ClipboardEvent) => {
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed) return;

      // Ensure the selection started inside our logs container
      if (!containerRef.current?.contains(selection.anchorNode)) return;

      // Extract the range to find which log items are selected
      const range = selection.getRangeAt(0);
      const container = document.createElement("div");
      container.appendChild(range.cloneContents());

      // Find all log items in the selection using the data-log-key attribute
      const logElements = Array.from(container.querySelectorAll("[data-log-key]"));
      
      // If the user selected multiple logs, we provide a structured format.
      // If it's just one log or fragments of text, we let the default browser behavior handle it.
      if (logElements.length > 1) {
        e.preventDefault();
        
        const selectedLogsText = logElements
          .map((el) => {
            const key = el.getAttribute("data-log-key");
            const log = interleavedLogs.find((l) => getLogKey(l) === key);
            return log ? formatInterleavedLogToString(log) : el.textContent;
          })
          .filter(Boolean)
          .join("\n");

        if (selectedLogsText) {
          e.clipboardData?.setData("text/plain", selectedLogsText);
        }
      }
    };

    document.addEventListener("copy", handleCopy);
    return () => document.removeEventListener("copy", handleCopy);
  }, [interleavedLogs, containerRef]);
}
