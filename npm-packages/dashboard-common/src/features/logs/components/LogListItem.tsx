import classNames from "classnames";
import React, { useRef, useState, useEffect } from "react";
import { Portal } from "@headlessui/react";
import { LogStatusLine } from "@common/features/logs/components/LogStatusLine";
import { UdfLog } from "@common/lib/useLogs";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { LogLevel } from "@common/elements/LogLevel";
import { LogOutput, messagesToString } from "@common/elements/LogOutput";
import { msFormat } from "@common/lib/format";
import { cn } from "@ui/cn";
import { useHotkeys } from "react-hotkeys-hook";
import {
  displayName,
  functionIdentifierFromValue,
} from "@common/lib/functions/generateFileTree";
import { CopiedPopper } from "@common/elements/CopiedPopper";

type LogListItemProps = {
  log: UdfLog;
  setShownLog?: () => void;
  focused: boolean;
  hitBoundary?: "top" | "bottom" | null;
  newLogsPageSidepanel?: boolean;
  logKey?: string;
};

export const ITEM_SIZE = 24;

export function LogListItem({
  log,
  setShownLog,
  focused,
  hitBoundary,
  newLogsPageSidepanel,
  logKey,
}: LogListItemProps) {
  const wrapperRef = useRef<HTMLButtonElement | HTMLSpanElement>(null);
  const [didJustCopy, setDidJustCopy] = useState(false);
  const [copiedPopperElement, setCopiedPopperElement] =
    useState<HTMLDivElement | null>(null);

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
      e.preventDefault();
      const logText = formatLogToString(log);
      void navigator.clipboard.writeText(logText);
      setDidJustCopy(true);
    },
    {
      enabled: focused && !!setShownLog,
      preventDefault: true,
    },
    [log, focused, setShownLog],
  );

  // When the item receives focus and setShownLog is available, call it
  const handleFocus = () => {
    if (setShownLog) {
      setShownLog();
    }
  };
  const isFailure =
    log.kind === "outcome" ? !!log.error : log.output.level === "ERROR";

  // Only show boundary animation on the focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      className={classNames(
        "relative flex gap-2",
        isFailure && "bg-background-error/50 text-content-error",
        focused && "bg-background-highlight",
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
      style={{
        height: setShownLog ? ITEM_SIZE : undefined,
      }}
    >
      <Wrapper
        setShownLog={setShownLog}
        onFocus={handleFocus}
        newLogsPageSidepanel={newLogsPageSidepanel}
        logKey={logKey}
        ref={wrapperRef}
      >
        <div
          className={classNames(
            "flex gap-4 items-center",
            setShownLog ? "p-0.5 ml-2" : "w-full",
          )}
        >
          <div className="min-w-[9.25rem] text-left whitespace-nowrap">
            {log.localizedTimestamp}
            <span
              className={classNames(
                isFailure ? "text-content-error" : "text-content-secondary",
              )}
            >
              .
              {new Date(log.timestamp).toISOString().split(".")[1].slice(0, -1)}
            </span>
          </div>
          {setShownLog && (
            <div
              className={cn(
                "-ml-0.5 min-w-8 overflow-hidden rounded-sm border px-0.5 py-[1px] text-[10px] group-hover:border-border-selected",
                isFailure && "border-background-errorSecondary",
              )}
            >
              {log.requestId.slice(0, 4)}
            </div>
          )}

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
            setShownLog && (
              <hr
                className={classNames(
                  "min-w-[7rem]",
                  // eslint-disable-next-line no-restricted-syntax
                  isFailure ? "bg-content-error" : "bg-background-tertiary",
                )}
              />
            )
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
              maxChars={setShownLog ? 32 : 60}
              error={isFailure}
            />
          </div>
        </div>
        {log.kind === "log" && log.output.level && (
          <LogLevel level={log.output.level} />
        )}
        {log.kind === "log" && (
          <LogOutput output={log.output} wrap={!setShownLog} secondary />
        )}
        {log.kind === "outcome" && log.error && (
          <LogOutput
            output={{
              isTruncated: false,
              messages: [log.error],
              level: "FAILURE",
            }}
            secondary
            wrap={!setShownLog}
          />
        )}
      </Wrapper>
      <Portal>
        <CopiedPopper
          referenceElement={wrapperRef.current}
          copiedPopperElement={copiedPopperElement}
          setCopiedPopperElement={setCopiedPopperElement}
          show={didJustCopy}
          message="Copied log line"
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
    setShownLog?: () => void;
    onFocus?: () => void;
    newLogsPageSidepanel?: boolean;
    logKey?: string;
  }
>(function Wrapper(
  { children, setShownLog, onFocus, newLogsPageSidepanel, logKey },
  ref,
) {
  return setShownLog ? (
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
        "focus:outline-none focus:border focus:border-border-selected",
        "items-center",
        // Make space for the focus outline
        "h-[calc(100%-1px)]",
      )}
      onClick={() => setShownLog()}
      onFocus={newLogsPageSidepanel ? onFocus : undefined}
      tabIndex={0}
    >
      {children}
    </button>
  ) : (
    <span
      className="flex w-full flex-col items-start gap-2 p-2"
      ref={ref as React.Ref<HTMLSpanElement>}
    >
      {children}
    </span>
  );
});

function formatLogToString(log: UdfLog): string {
  const timestamp = log.localizedTimestamp;
  const milliseconds = new Date(log.timestamp)
    .toISOString()
    .split(".")[1]
    .slice(0, -1);
  const fullTimestamp = `${timestamp}.${milliseconds}`;

  // Parse the call field to get identifier and componentPath
  let functionName: string;
  if (log.kind === "log" && log.output.subfunction) {
    const { identifier, componentPath } = functionIdentifierFromValue(
      log.output.subfunction,
    );
    functionName = displayName(identifier, componentPath);
  } else {
    const { identifier, componentPath } = functionIdentifierFromValue(log.call);
    functionName = displayName(identifier, componentPath);
  }

  const udfType = log.udfType.charAt(0).toUpperCase();

  let content = "";
  if (log.kind === "log") {
    const level = log.output.level ? `[${log.output.level}] ` : "";
    const message = messagesToString(log.output);
    content = `${level}${message}`;
  } else if (log.error) {
    content = log.error;
  } else {
    content = log.outcome.status;
  }

  const executionTime =
    log.kind === "outcome" &&
    log.executionTimeMs !== null &&
    log.executionTimeMs > 0
      ? ` ${msFormat(log.executionTimeMs)}`
      : "";

  return `${fullTimestamp} ${udfType} ${functionName}${executionTime} ${content}`;
}
