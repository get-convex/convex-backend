import { useContext, useState, useRef, useEffect } from "react";
import classNames from "classnames";
import { GearIcon } from "@radix-ui/react-icons";
import {
  DeploymentEventContent,
  ActionText,
} from "@common/elements/DeploymentEventContent";
import { ITEM_SIZE } from "@common/features/logs/components/LogListItem";
import { formatDateTime } from "@common/lib/format";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";
import { DetailPanel } from "@common/elements/DetailPanel";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function DeploymentEventListItem({
  event,
  inline = false,
  focused = false,
  hitBoundary,
  setShownLog,
  onCloseDialog,
  newLogsPageSidepanel,
}: {
  event: DeploymentAuditLogEvent;
  inline?: boolean;
  focused?: boolean;
  hitBoundary?: "top" | "bottom" | null;
  setShownLog?: () => void;
  onCloseDialog?: () => void;
  newLogsPageSidepanel?: boolean;
}) {
  const { TeamMemberLink } = useContext(DeploymentInfoContext);
  const [showDetails, setShowDetails] = useState(false);
  const timestamp = formatDateTime(new Date(event._creationTime));
  const ref = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const prevFocusedRef = useRef(focused);

  // Focus the button when focused prop changes to true
  useEffect(() => {
    if (newLogsPageSidepanel && focused) {
      buttonRef.current?.focus();
    }
  }, [focused, newLogsPageSidepanel]);

  // Scroll into view when transitioning to focused (only in side panel mode)
  useEffect(() => {
    if (
      focused &&
      !prevFocusedRef.current &&
      ref.current &&
      newLogsPageSidepanel
    ) {
      ref.current.scrollIntoView({
        block: "center",
        inline: "nearest",
      });
    }
    prevFocusedRef.current = focused;
  }, [focused, ref, newLogsPageSidepanel]);

  // When the button receives focus and setShownLog is available, call it
  const handleFocus = () => {
    if (setShownLog) {
      setShownLog();
    }
  };

  const handleClick = () => {
    if (!newLogsPageSidepanel) {
      setShowDetails(true);
    }
    if (setShownLog) {
      setShownLog();
    }
  };

  // Only show boundary animation on the focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      ref={ref}
      className={classNames(
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
    >
      {/* eslint-disable-next-line react/forbid-elements */}
      <button
        ref={buttonRef}
        type="button"
        className={classNames(
          "group pl-3 flex items-center gap-3 w-full text-xs",
          inline ? "items-start" : "pl-1 items-center",
          setShownLog && "hover:bg-background-tertiary/70",
          setShownLog &&
            "focus:outline-none focus:border focus:border-border-selected",
        )}
        style={{
          height: ITEM_SIZE,
        }}
        onClick={handleClick}
        onFocus={handleFocus}
        tabIndex={setShownLog ? 0 : -1}
      >
        <div className="min-w-[9.25rem] text-left font-mono whitespace-nowrap text-content-primary">
          {timestamp}
          <span className="text-content-secondary">
            .
            {new Date(event._creationTime)
              .toISOString()
              .split(".")[1]
              .slice(0, -1)}
          </span>
        </div>

        {/* Blank lines for when deployment event list items are displayed as items in the logs page */}
        {!inline && <hr className="min-w-[10.375rem] bg-background-tertiary" />}

        <div className="flex h-6 items-center gap-2 truncate">
          <GearIcon className="shrink-0" />
          <span className="truncate">
            <TeamMemberLink
              memberId={Number(event.member_id)}
              name={event.memberName}
            />{" "}
            <ActionText event={event} />
          </span>
        </div>
      </button>

      {showDetails && (
        <DetailPanel
          onClose={() => {
            setShowDetails(false);
            onCloseDialog?.();
          }}
          header="Deployment Event"
          content={<DeploymentEventContent event={event} />}
        />
      )}
    </div>
  );
}
