import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

export function UsageNoDataError({ entity }: { entity: string }) {
  return (
    <TeamUsageError
      title={`No ${entity}`}
      description={`No ${entity} could be found for the period shown.`}
    />
  );
}

export function UsageDataNotAvailable({ entity }: { entity: string }) {
  return (
    <TeamUsageError
      title={`${entity} not available`}
      description={`${entity} is not available for the period shown.`}
    />
  );
}

export function UsageChartUnavailable() {
  return (
    <TeamUsageError
      title="Chart not available"
      description="This chart isn’t available when filtering on a particular project."
    />
  );
}

export function TeamUsageError({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div className="flex h-56 animate-fadeInFromLoading flex-col justify-center p-2 text-center">
      <QuestionMarkCircledIcon className="mx-auto h-6 w-6 text-content-tertiary" />
      <h5 className="mt-2">{title}</h5>
      <p className="mt-1 text-sm text-content-secondary">{description}</p>
    </div>
  );
}
