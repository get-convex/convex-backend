import { useState } from "react";
import {
  Button,
  formatDateTime,
  DeploymentAuditLogEvent,
  DetailPanel,
} from "dashboard-common";
import classNames from "classnames";
import { GearIcon } from "@radix-ui/react-icons";
import { TeamMemberLink } from "elements/TeamMemberLink";
import {
  DeploymentEventContent,
  ActionText,
} from "elements/DeploymentEventContent";
import { ITEM_SIZE } from "./LogListItem";

export function DeploymentEventListItem({
  event,
  inline = false,
}: {
  event: DeploymentAuditLogEvent;
  inline?: boolean;
}) {
  const [showDetails, setShowDetails] = useState(false);
  const timestamp = formatDateTime(new Date(event._creationTime));

  return (
    <Button
      className={classNames(
        "group pl-3 flex items-center gap-3 w-full hover:bg-background-tertiary text-xs",
        inline ? "items-start" : "pl-1 items-center",
      )}
      style={{
        height: ITEM_SIZE,
      }}
      onClick={() => setShowDetails(true)}
      variant="unstyled"
    >
      <div className="min-w-[7.5rem] whitespace-nowrap text-left font-mono text-content-primary">
        {timestamp}
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

      {showDetails && (
        <DetailPanel
          onClose={() => setShowDetails(false)}
          header="Deployment Event"
          content={<DeploymentEventContent event={event} />}
        />
      )}
    </Button>
  );
}
