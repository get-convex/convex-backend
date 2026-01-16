import { Tooltip } from "@ui/Tooltip";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

type EnvironmentInfo = {
  workosEnvironmentId: string;
  workosEnvironmentName: string;
  workosClientId: string;
  isProduction?: boolean;
};

interface WorkOSEnvironmentInfoProps {
  environment: EnvironmentInfo;
}

export function WorkOSEnvironmentInfo({
  environment,
}: WorkOSEnvironmentInfoProps) {
  return (
    <Tooltip
      maxWidthClassName="max-w-sm"
      tip={
        <div className="flex flex-col gap-2 p-1 text-left">
          <div className="flex flex-col gap-1">
            <div className="text-xs font-semibold">Environment Name</div>
            <div className="text-xs">{environment.workosEnvironmentName}</div>
          </div>

          <div className="flex flex-col gap-1">
            <div className="text-xs font-semibold">WorkOS Environment ID</div>
            <div className="w-full overflow-x-auto">
              <CopyTextButton
                text={environment.workosEnvironmentId}
                className="font-mono text-xs break-all"
              />
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <div className="text-xs font-semibold">WorkOS Client ID</div>
            <div className="w-full overflow-x-auto">
              <CopyTextButton
                text={environment.workosClientId}
                className="font-mono text-xs break-all"
              />
            </div>
          </div>

          <div className="text-xs text-content-secondary">
            This is a{" "}
            <span className="font-semibold">
              {environment.isProduction ? "production" : "non-production"}
            </span>{" "}
            environment.
          </div>
        </div>
      }
    >
      <QuestionMarkCircledIcon className="h-3.5 w-3.5 flex-shrink-0 text-content-tertiary" />
    </Tooltip>
  );
}
