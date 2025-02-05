import { useQuery } from "convex/react";
import udfs from "udfs";
import { cn } from "@common/lib/cn";
import { HealthCard } from "@common/elements/HealthCard";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Loading } from "@common/elements/Loading";

export function LastDeployed() {
  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  const content =
    lastPushEvent === undefined ? (
      <Loading className="h-5 w-24" />
    ) : !lastPushEvent ? (
      <span
        className={cn(
          "text-content-secondary text-sm animate-fadeInFromLoading",
        )}
      >
        Never
      </span>
    ) : (
      <TimestampDistance
        date={new Date(lastPushEvent?._creationTime || 0)}
        className="w-fit animate-fadeInFromLoading text-sm text-content-primary"
      />
    );

  return (
    <HealthCard
      title="Last Deployed"
      size="sm"
      tip="The last time functions were deployed."
    >
      <div className="h-full w-full grow px-2 pb-2">{content}</div>
    </HealthCard>
  );
}
