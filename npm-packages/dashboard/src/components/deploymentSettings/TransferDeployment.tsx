import { useContext, useMemo, useState } from "react";
import { useRouter } from "next/router";
import { useDebounce } from "react-use";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Combobox, MAX_DISPLAYED_OPTIONS } from "@ui/Combobox";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useTransferDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import { usePaginatedProjects } from "api/projects";
import { useCurrentTeam } from "api/teams";

export function TransferDeployment() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const team = useCurrentTeam();
  const router = useRouter();

  const isProd = deployment?.deploymentType === "prod";
  const isLocal = deployment?.kind === "local";

  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );

  const [filter, setFilter] = useState("");
  const [debouncedFilter, setDebouncedFilter] = useState("");

  useDebounce(
    () => {
      setDebouncedFilter(filter);
    },
    300,
    [filter],
  );

  const paginatedProjects = usePaginatedProjects(team?.id, {
    q: debouncedFilter,
    limitOverride: MAX_DISPLAYED_OPTIONS,
  });

  const projects = paginatedProjects?.items;

  // Filter out the current project and prepare options with duplicate name handling
  const { projectOptions, slugByProjectId } = useMemo(() => {
    const otherProjects =
      projects?.filter((p) => p.id !== deployment?.projectId) ?? [];

    const nameCountMap = new Map<string, number>();
    otherProjects.forEach((p) => {
      nameCountMap.set(p.name, (nameCountMap.get(p.name) || 0) + 1);
    });

    const slugMap = new Map<number, string>();
    const options = otherProjects.map((p) => {
      const isDuplicate = (nameCountMap.get(p.name) || 0) > 1;
      if (isDuplicate && p.slug) {
        slugMap.set(p.id, p.slug);
      }
      const label = isDuplicate && p.slug ? `${p.name} (${p.slug})` : p.name;
      return { label, value: p.id };
    });

    return { projectOptions: options, slugByProjectId: slugMap };
  }, [projects, deployment?.projectId]);

  const [destinationProjectId, setDestinationProjectId] = useState<
    number | null
  >(null);
  const [showConfirmation, setShowConfirmation] = useState(false);

  const transferDeployment = useTransferDeployment(deployment?.name ?? "");

  const destinationProject = projects?.find(
    (p) => p.id === destinationProjectId,
  );

  // Prod deployments require project admin on both source and destination
  const canTransfer = isProd ? hasAdminPermissions : true;

  if (!deployment || isLocal) {
    return null;
  }

  return (
    <Sheet>
      <h3 className="mb-4">Transfer Deployment</h3>
      <p className="mb-5 max-w-prose text-sm text-content-primary">
        Transfer this deployment to another project within the same team.
        {isProd && (
          <span className="font-semibold">
            {" "}
            Transferring a production deployment requires project admin
            permissions on both projects.
          </span>
        )}
      </p>
      <div className="mb-4 flex flex-col gap-1">
        <Combobox
          label="Destination Project"
          labelHidden={false}
          placeholder="Select a project"
          allowCustomValue
          onFilterChange={setFilter}
          isLoadingOptions={
            !!paginatedProjects?.isLoading && debouncedFilter === filter
          }
          buttonProps={{
            tip:
              !canTransfer && isProd
                ? "You must be a project admin to transfer production deployments."
                : undefined,
          }}
          options={projectOptions}
          selectedOption={destinationProjectId}
          setSelectedOption={setDestinationProjectId}
          disabled={!canTransfer}
          Option={({ label, value }) => {
            const slug =
              value !== null ? slugByProjectId.get(value) : undefined;
            if (slug) {
              const name = label.replace(` (${slug})`, "");
              return (
                <span>
                  {name}{" "}
                  <span className="text-content-secondary">({slug})</span>
                </span>
              );
            }
            return <span>{label}</span>;
          }}
        />
      </div>
      <Button
        variant="primary"
        disabled={!destinationProjectId || !canTransfer}
        tip={
          !canTransfer && isProd
            ? "You must be a project admin to transfer production deployments."
            : !destinationProjectId
              ? "Select a project to transfer this deployment to."
              : undefined
        }
        onClick={() => setShowConfirmation(true)}
      >
        Transfer
      </Button>
      {deployment &&
        destinationProject &&
        showConfirmation &&
        destinationProjectId && (
          <ConfirmationDialog
            confirmText="Transfer"
            validationText={`Transfer ${deployment.reference} to ${destinationProject.slug}`}
            dialogTitle={`Transfer Deployment to ${destinationProject.name}?`}
            dialogBody={
              <div className="flex flex-col gap-2">
                <p>
                  This will move the deployment from its current project to{" "}
                  <span className="font-semibold">
                    {destinationProject.name}
                  </span>
                  .
                </p>
                <p className="text-sm text-content-secondary">
                  All data, files, and configuration will remain unchanged. The
                  deployment will appear in the destination project after
                  transfer.
                </p>
              </div>
            }
            onConfirm={async () => {
              await transferDeployment({
                destinationProjectId: destinationProjectId,
              });
              // Use window.location to do a full navigation instead of
              // client-side routing. This avoids a 404 flash caused by SWR
              // revalidating the old project's deployment list (which no
              // longer contains this deployment) before the redirect completes.
              const teamSlug = router.query.team as string;
              window.location.href = `/t/${teamSlug}/${destinationProject.slug}/${deployment.name}/settings`;
            }}
            variant="primary"
            onClose={() => setShowConfirmation(false)}
          />
        )}
    </Sheet>
  );
}
