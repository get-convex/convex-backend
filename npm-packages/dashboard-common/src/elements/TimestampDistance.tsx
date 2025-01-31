import { formatDistanceToNow } from "date-fns";
import { useRefresh } from "lib/useRefresh";
import { cn } from "lib/cn";
import { Tooltip } from "elements/Tooltip";

export function TimestampDistance({
  prefix = "",
  date,
  className = "",
}: {
  prefix?: string;

  date: Date;
  className?: string;
}) {
  return (
    <Tooltip tip={date.toLocaleString()}>
      <div className={cn("text-xs text-content-secondary", className)}>
        {`${prefix} ${formatDistanceToNow(date, {
          addSuffix: true,
        }).replace("about ", "")}`}
      </div>
    </Tooltip>
  );
}

export function LiveTimestampDistance({
  prefix,
  date,
  className,
}: {
  prefix?: string;
  date: Date;
  className?: string;
}) {
  return (
    <Tooltip tip={date.toLocaleString()} className="truncate">
      <div className={cn("text-xs text-content-secondary truncate", className)}>
        {prefix} <LiveTimestampDistanceInner date={date} />
      </div>
    </Tooltip>
  );
}

export function LiveTimestampDistanceInner({ date }: { date: Date }) {
  useRefresh();
  return (
    <>
      {formatDistanceToNow(date, {
        addSuffix: true,
      }).replace("about ", "")}
    </>
  );
}
