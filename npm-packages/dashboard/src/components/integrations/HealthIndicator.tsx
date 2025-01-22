import classNames from "classnames";
import { Integration } from "system-udfs/convex/_system/frontend/common";
import { Tooltip } from "dashboard-common";

type HealthStatusMetadata = {
  lightTextColor: string;
  darkTextColor: string;
};

function statusToColors(
  status: Integration["status"]["type"],
): HealthStatusMetadata {
  switch (status) {
    case "active":
      return {
        lightTextColor: "text-green-700",
        darkTextColor: "dark:text-green-200",
      };
    case "pending":
      return {
        lightTextColor: "text-yellow-700",
        darkTextColor: "dark:text-yellow-200",
      };
    case "failed":
      return {
        lightTextColor: "text-content-error",
        darkTextColor: "text-content-error",
      };
    case "deleting":
      return {
        lightTextColor: "text-slate-700",
        darkTextColor: "dark:text-slate-200",
      };
    default:
      // eslint-disable-next-line no-case-declarations
      const _: never = status;
      throw new Error(`Unrecognized health status ${status}`);
  }
}

export function HealthIndicator({ status }: { status: Integration["status"] }) {
  const { lightTextColor, darkTextColor } = statusToColors(status.type);

  return (
    <div
      className={classNames("text-xs", `${lightTextColor} ${darkTextColor}`)}
    >
      {status.type === "failed" ? (
        <Tooltip tip={`Reason: ${status.reason}`}>Failed</Tooltip>
      ) : (
        status.type.charAt(0).toUpperCase() + status.type.slice(1)
      )}
    </div>
  );
}
