import React, { useContext, useEffect, useState } from "react";
import { useQuery } from "convex/react";
import Link from "next/link";
import { Sheet } from "@common/elements/Sheet";
import { Loading } from "@common/elements/Loading";
import { Button } from "@common/elements/Button";
import { Callout } from "@common/elements/Callout";
import { ConfirmationDialog } from "@common/elements/ConfirmationDialog";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useChangeDeploymentState } from "../lib/api";

// TODO insert link to docs here
const RESUME_EXPLANATION: string[] = [
  "New function calls can be made.",
  "Any functions scheduled before the deployment was paused will run.",
  "Cron jobs will resume according to their schedule.",
];

const PAUSE_EXPLANATION: string[] = [
  "New function calls will return an error.",
  "Scheduled jobs will queue and run when the deployment is resumed.",
  "Cron jobs will be skipped.",
];

export function PauseDeployment() {
  const deploymentState = useQuery(udfs.deploymentState.deploymentState);
  const { useCurrentDeployment, useHasProjectAdminPermissions } = useContext(
    DeploymentInfoContext,
  );
  const deployment = useCurrentDeployment();
  const deploymentType = deployment?.deploymentType ?? "prod";
  const [paused, setPaused] = useState(false);
  const [showConfirmation, setShowConfirmation] = useState(false);

  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canPauseOrResume =
    deployment?.deploymentType !== "prod" || hasAdminPermissions;

  const changeDeploymentState = useChangeDeploymentState();
  useEffect(() => {
    if (deploymentState) {
      setPaused(deploymentState.state === "paused");
    }
  }, [deploymentState]);
  async function toggle() {
    await changeDeploymentState(paused ? "running" : "paused");
  }
  function changeVerb(isPaused: boolean) {
    return isPaused ? "Resume" : "Pause";
  }

  const { useCurrentTeam, useTeamUsageState } = useContext(
    DeploymentInfoContext,
  );
  // Prevent direct access to this page if the team is disabled/paused
  const team = useCurrentTeam();
  const teamUsageState = useTeamUsageState(team?.id ?? null);
  if (teamUsageState === "Paused" || teamUsageState === "Disabled") {
    return (
      <Sheet>
        <h3 className="mb-4">Pause Deployment</h3>
        <Callout variant="error">
          Your projects are currently disabled. See the banner above for next
          steps.
        </Callout>
      </Sheet>
    );
  }

  return (
    <div>
      {deploymentState === undefined ? (
        <Loading />
      ) : (
        <Sheet className="flex w-full flex-col gap-4 lg:grid lg:grid-cols-[1fr_auto]">
          {showConfirmation && (
            <ConfirmationDialog
              onClose={() => setShowConfirmation(false)}
              onConfirm={() => Promise.resolve(toggle())}
              confirmText={
                changeVerb(paused) +
                (deploymentType === "prod" ? " Production" : "")
              }
              dialogTitle={`${changeVerb(paused)} Deployment`}
              dialogBody={
                <>
                  Are you sure you want to {changeVerb(paused).toLowerCase()}{" "}
                  this{" "}
                  {deploymentType === "prod" ? (
                    <span className="font-semibold">Production</span>
                  ) : null}{" "}
                  deployment?
                </>
              }
              variant={paused ? undefined : "danger"}
            />
          )}
          <div>
            <h3 className="mb-4">Pause Deployment</h3>
            <p>
              This deployment is currently{" "}
              <b>{paused ? "paused" : "running"}</b>.
            </p>
          </div>
          <div className="flex items-center lg:row-span-2">
            <Button
              className="lg:order-2"
              variant={paused ? "primary" : "danger"}
              onClick={() => setShowConfirmation(true)}
              disabled={!canPauseOrResume}
              tip={
                !canPauseOrResume
                  ? "You do not have permission to pause or resume production."
                  : ""
              }
            >
              {paused ? "Resume Deployment" : "Pause Deployment"}
            </Button>
          </div>
          <div className="lg:order-1">
            When a deployment is {`${changeVerb(paused).toLowerCase()}d`}:
            <ul className="list-outside list-disc pl-4">
              {(paused ? RESUME_EXPLANATION : PAUSE_EXPLANATION).map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
            <br />
            <Link
              passHref
              href="https://docs.convex.dev/production/pause-deployment"
              className="text-content-link hover:underline dark:underline"
              target="_blank"
            >
              Learn more
            </Link>
            .
          </div>
        </Sheet>
      )}
    </div>
  );
}
