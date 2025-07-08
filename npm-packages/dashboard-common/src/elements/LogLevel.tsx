import classNames from "classnames";
import { LogLevel as Level } from "system-udfs/convex/_system/frontend/common";

const levelToColor: Record<Level | "FAILURE", string> = {
  INFO: "bg-background-tertiary/50",
  LOG: "bg-background-tertiary/50",
  ERROR: "bg-background-error/50",
  FAILURE: "bg-background-error/50",
  WARN: "bg-background-warning/50",
  DEBUG: "bg-blue-100/50 dark:bg-blue-700/50",
};

export function LogLevel({ level }: { level: Level | "FAILURE" }) {
  return (
    <span className={classNames("p-0.5 px-1 rounded-sm", levelToColor[level])}>
      {level.toLowerCase()}
    </span>
  );
}
