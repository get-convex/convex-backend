import classNames from "classnames";
import React, { useEffect, useRef } from "react";
import { LogStatusLine } from "@common/features/logs/components/LogStatusLine";
import { UdfLog } from "@common/lib/useLogs";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { LogLevel } from "@common/elements/LogLevel";
import { LogOutput } from "@common/elements/LogOutput";
import { msFormat } from "@common/lib/format";

type LogListItemProps = {
  log: UdfLog;
  setShownLog?(shown: UdfLog | undefined): void;
  focused?: boolean;
  selected?: boolean;
  hitBoundary?: "top" | "bottom" | null;
};

export const ITEM_SIZE = 24;

export function LogListItem({
  log,
  setShownLog,
  focused = false,
  selected = false,
  hitBoundary,
}: LogListItemProps) {
  const ref = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const prevFocusedRef = useRef(focused);

  // Pass the button ref to parent
  useEffect(() => {
    if (focused) {
      buttonRef.current?.focus();
    }
  }, [focused]);

  useEffect(() => {
    // Only scroll into view when transitioning to focused (not already focused)
    if (focused && !prevFocusedRef.current && ref.current) {
      ref.current.scrollIntoView({
        block: "center",
        inline: "nearest",
      });
    }
    prevFocusedRef.current = focused;
  }, [focused, ref]);

  // When the item receives focus and setShownLog is available, call it
  const handleFocus = () => {
    if (setShownLog) {
      setShownLog(log);
    }
  };
  const isFailure =
    log.kind === "outcome" ? !!log.error : log.output.level === "ERROR";

  // Only show boundary animation on the selected/focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      ref={ref}
      className={classNames(
        "flex gap-2",
        isFailure && "bg-background-error/50 text-content-error",
        setShownLog && "hover:bg-background-tertiary/70",
        selected && "bg-background-tertiary",
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
      style={{
        height: setShownLog ? ITEM_SIZE : undefined,
      }}
    >
      <Wrapper
        setShownLog={setShownLog ? () => setShownLog(log) : undefined}
        buttonRef={buttonRef}
        onFocus={handleFocus}
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
            <div className="-ml-0.5 min-w-8 overflow-hidden rounded-sm border px-0.5 py-[1px] text-[10px] group-hover:border-border-selected">
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
            <p className="rounded-sm bg-background-tertiary/80 p-0.5 px-1 text-[11px]">
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
    </div>
  );
}

function Wrapper({
  children,
  setShownLog,
  buttonRef,
  onFocus,
}: {
  children: React.ReactNode;
  setShownLog?: () => void;
  buttonRef?: React.RefObject<HTMLButtonElement>;
  onFocus?: () => void;
}) {
  return setShownLog ? (
    // We do not use Button here because it's expensive and this table needs to be fast
    // eslint-disable-next-line react/forbid-elements
    <button
      ref={buttonRef}
      type="button"
      className={classNames(
        "flex gap-2 truncate p-0.5",
        "group w-full font-mono text-xs",
        "focus:outline-none",
        "items-center",
        // Make space for the focus outline
        "h-[calc(100%-1px)]",
      )}
      onClick={() => setShownLog()}
      onFocus={onFocus}
      tabIndex={0}
    >
      {children}
    </button>
  ) : (
    <span className="flex w-full flex-col items-start gap-2 p-2">
      {children}
    </span>
  );
}
