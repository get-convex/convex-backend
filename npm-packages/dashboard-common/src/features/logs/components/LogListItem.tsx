import classNames from "classnames";
import React, { useRef, useState, useEffect } from "react";
import { Portal } from "@headlessui/react";
import { LogStatusLine } from "@common/features/logs/components/LogStatusLine";
import { UdfLog } from "@common/lib/useLogs";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { LogLevel } from "@common/elements/LogLevel";
import { LogOutput } from "@common/elements/LogOutput";
import { msFormat } from "@common/lib/format";
import { cn } from "@ui/cn";
import { useHotkeys } from "react-hotkeys-hook";
import { CopiedPopper } from "@common/elements/CopiedPopper";
import { TimestampTooltip } from "@common/features/logs/components/TimestampTooltip";
import { formatUdfLogToString } from "@common/features/logs/lib/formatLog";

type LogListItemProps = {
  log: UdfLog;
  setShownLog: () => void;
  focused: boolean;
  selected?: boolean;
  hitBoundary?: "top" | "bottom" | null;
  logKey?: string;
  highlight?: string;
  onClick?: (e: React.MouseEvent) => void;
  hasRowSelection?: boolean;
};

export const ITEM_SIZE = 24;

export function LogListItem({
  log,
  setShownLog,
  focused,
  selected = false,
  hitBoundary,
  logKey,
  highlight,
  onClick,
  hasRowSelection = false,
}: LogListItemProps) {
  const wrapperRef = useRef<HTMLButtonElement | HTMLSpanElement>(null);
  const [didJustCopy, setDidJustCopy] = useState(false);
  const [copiedPopperElement, setCopiedPopperElement] =
    useState<HTMLDivElement | null>(null);
  const [copyMessage, setCopyMessage] = useState("Copied log line");

  // Reset copied state after 800ms
  useEffect(() => {
    if (didJustCopy) {
      const timeout = setTimeout(() => setDidJustCopy(false), 800);
      return () => clearTimeout(timeout);
    }
  }, [didJustCopy]);

  // Copy entire log line on Cmd/Ctrl+C
  useHotkeys(
    ["meta+c", "ctrl+c"],
    (e) => {
      const selection = window.getSelection();
      if (selection !== null && !selection.isCollapsed) {
        // The user has selected some text, so let them copy the text they selected.
        return;
      }
      if (hasRowSelection) {
        // Let the global copy handler use the selected rows.
        return;
      }
      e.preventDefault();
      const logText = formatUdfLogToString(log);
      setCopyMessage("Copied log line");
      void (async () => {
        if (!navigator.clipboard?.writeText) {
          return;
        }
        try {
          await navigator.clipboard.writeText(logText);
          setDidJustCopy(true);
        } catch {
          // Ignore clipboard errors (permissions/unsupported).
        }
      })();
    },
    {
      enabled: focused,
    },
    [log, focused, hasRowSelection],
  );

  const isFailure =
    log.kind === "outcome" ? !!log.error : log.output.level === "ERROR";

  // Only show boundary animation on the focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      className={classNames(
        "relative flex gap-2",
        isFailure && "bg-background-error/50 text-content-error",
        selected && !focused && !isFailure && "bg-background-tertiary/50",
        selected && !focused && isFailure && "bg-background-error/60",
        focused && "bg-background-highlight",
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
      style={{
        height: ITEM_SIZE,
      }}
    >
      <Wrapper
        setShownLog={setShownLog}
        logKey={logKey}
        onClick={onClick}
        ref={wrapperRef}
      >
        <div className={classNames("flex gap-4 items-center", "p-0.5 ml-2")}>
          <div className="min-w-[9.25rem] text-left whitespace-nowrap">
            <TimestampTooltip timestamp={log.timestamp}>
              <span>
                {log.localizedTimestamp}
                <span
                  className={classNames(
                    isFailure ? "text-content-error" : "text-content-secondary",
                  )}
                >
                  .
                  {new Date(log.timestamp)
                    .toISOString()
                    .split(".")[1]
                    .slice(0, -1)}
                </span>
              </span>
            </TimestampTooltip>
          </div>
          <div
            className={cn(
              "-ml-0.5 min-w-8 overflow-hidden rounded-sm border px-0.5 py-[1px] text-[10px] group-hover:border-border-selected",
              isFailure && "border-background-errorSecondary",
            )}
          >
            {log.requestId.slice(0, 4)}
          </div>

          {log.kind === "outcome" ? (
            <div className="flex min-w-[7rem] items-center gap-2">
              <LogStatusLine outcome={log.outcome} />{" "}
              <div className="w-8 min-w-[2rem] text-right whitespace-nowrap">
                {log.cachedResult ? (
                  <span className="text-xs font-medium text-content-success">
                    (cached)
                  </span>
                ) : (
                  log.executionTimeMs !== null &&
                  log.executionTimeMs > 0 &&
                  msFormat(log.executionTimeMs)
                )}
              </div>
            </div>
          ) : (
            <hr
              className={classNames(
                "min-w-[7rem]",
                // eslint-disable-next-line no-restricted-syntax
                isFailure ? "bg-content-error" : "bg-background-tertiary",
              )}
            />
          )}
          <div className="flex items-center gap-2">
            <p
              className={cn(
                "rounded-sm p-0.5 px-1 text-[11px]",
                isFailure ? "bg-background-error" : "bg-background-tertiary/80",
              )}
            >
              {log.udfType.charAt(0).toUpperCase()}
            </p>
            <FunctionNameOption
              label={
                log.kind === "log"
                  ? (log.output.subfunction ?? log.call)
                  : log.call
              }
              oneLine
              maxChars={32}
              error={isFailure}
            />
          </div>
        </div>
        {log.kind === "log" && log.output.level && (
          <LogLevel level={log.output.level} />
        )}
        {log.kind === "log" && (
          <LogOutput output={log.output} secondary highlight={highlight} />
        )}
        {log.kind === "outcome" && log.error && (
          <LogOutput
            output={{
              isTruncated: false,
              messages: [log.error],
              level: "FAILURE",
            }}
            secondary
            highlight={highlight}
          />
        )}
      </Wrapper>
      <Portal>
        <CopiedPopper
          referenceElement={wrapperRef.current}
          copiedPopperElement={copiedPopperElement}
          setCopiedPopperElement={setCopiedPopperElement}
          show={didJustCopy}
          message={copyMessage}
          placement="bottom"
        />
      </Portal>
    </div>
  );
}

const Wrapper = React.forwardRef<
  HTMLButtonElement | HTMLSpanElement,
  {
    children: React.ReactNode;
    setShownLog: () => void;
    logKey?: string;
    onClick?: (e: React.MouseEvent) => void;
  }
>(function Wrapper({ children, setShownLog, logKey, onClick }, ref) {
  return (
    // We do not use Button here because it's expensive and this table needs to be fast
    // eslint-disable-next-line react/forbid-elements
    <button
      type="button"
      data-log-key={logKey}
      ref={ref as React.Ref<HTMLButtonElement>}
      className={classNames(
        "flex gap-2 truncate p-0.5 animate-fadeInFromLoading",
        "group w-full font-mono text-xs",
        "hover:bg-background-tertiary/70",
        "focus:outline-none focus:border-y focus:border-border-selected",
        "items-center",
        // Make space for the focus outline
        "h-[calc(100%-1px)]",
        "select-text",
      )}
      onClick={(e) => {
        if (onClick) {
          onClick(e);
        } else {
          setShownLog();
        }
      }}
      onFocus={setShownLog}
      tabIndex={0}
    >
      {children}
    </button>
  );
});
