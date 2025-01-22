import classNames from "classnames";

export function DeploymentTimes({
  deploymentTimes,
}: {
  deploymentTimes?: string[];
}) {
  return deploymentTimes && deploymentTimes.length > 0 ? (
    <div
      className={classNames(
        "flex mt-1 gap-1",
        deploymentTimes.length > 1 && "flex-col",
      )}
    >
      <div className="text-left font-semibold">Deployed at:</div>
      {deploymentTimes.slice(0, 3).map((time) => (
        <div key={time}>{time}</div>
      ))}
      {deploymentTimes.length > 3 && (
        <div>
          ...and {deploymentTimes.length - 3} more time
          {deploymentTimes.length - 3 === 1 ? "" : "s"}
        </div>
      )}
    </div>
  ) : null;
}
