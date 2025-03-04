import {
  CheckCircledIcon,
  ExclamationTriangleIcon,
  PlusIcon,
  TrashIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button } from "dashboard-common/elements/Button";
import { Tooltip } from "dashboard-common/elements/Tooltip";
import { Combobox } from "dashboard-common/elements/Combobox";
import { Callout, LocalDevCallout } from "dashboard-common/elements/Callout";
import {
  ENVIRONMENT_VARIABLES_ROW_CLASSES,
  ENVIRONMENT_VARIABLE_NAME_COLUMN,
} from "dashboard-common/features/settings/components/EnvironmentVariables";
import { Sheet } from "dashboard-common/elements/Sheet";
import { ConfirmationDialog } from "dashboard-common/elements/ConfirmationDialog";
import { TextInput } from "dashboard-common/elements/TextInput";
import { useFormik } from "formik";
import { useDeployments } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useHasProjectAdminPermissions } from "api/roles";
import Link from "next/link";
import { useState, useMemo } from "react";
import {
  Team,
  VanityDomainRequestArgs,
  VanityDomainResponse,
} from "generatedApi";
import {
  useListVanityDomains,
  useCreateVanityDomain,
  useDeleteVanityDomain,
} from "api/vanityDomains";
import { useDeploymentUrl } from "dashboard-common/lib/deploymentApi";
import { DeploymentInfoProvider } from "providers/DeploymentInfoProvider";
import { MaybeDeploymentApiProvider } from "providers/MaybeDeploymentApiProvider";
import { WaitForDeploymentApi } from "dashboard-common/lib/deploymentContext";
import { useQuery } from "convex/react";
import udfs from "dashboard-common/udfs";
import { useUpdateCanonicalUrl } from "hooks/deploymentApi";
import { useLaunchDarkly } from "../../hooks/useLaunchDarkly";

export function CustomDomains({
  team,
  hasEntitlement,
}: {
  team: Team;
  hasEntitlement: boolean;
}) {
  const project = useCurrentProject();
  const deployment = useDeployments(project?.id).deployments?.find(
    (d) => d.deploymentType === "prod",
  );
  const hasAdminPermissions = useHasProjectAdminPermissions(project?.id);
  const vanityDomains = useListVanityDomains(
    hasEntitlement ? deployment?.name : undefined,
  );
  const hasEditAccess = hasEntitlement && hasAdminPermissions;
  const { canonicalCustomDomains } = useLaunchDarkly();

  const proCallout = hasEntitlement ? null : (
    <Callout>
      <div>
        Custom domains are{" "}
        <span className="font-semibold">only available on paid plans</span>.{" "}
        <Link
          href={`/${team?.slug}/settings/billing`}
          className="text-content-link"
        >
          Upgrade to get access.
        </Link>{" "}
      </div>
    </Callout>
  );

  return (
    <Sheet>
      <div className="flex flex-col gap-4">
        <div>
          <h3 className="mb-2">Custom Domains</h3>
          <p className="max-w-prose">
            Add a custom domain to your Production Convex deployment. Domains
            for the Convex API (your queries, mutations, and actions) and your
            HTTP actions are configured separately.
          </p>
        </div>
        {proCallout}
        {(hasEntitlement || (vanityDomains && vanityDomains.length > 0)) && (
          <div>
            {deployment ? (
              <VanityDomainForm
                disabled={!hasEditAccess}
                deploymentName={deployment.name}
              />
            ) : (
              <span className="text-content-secondary">
                This project does not have a Production deployment yet.
              </span>
            )}
            {vanityDomains && vanityDomains.length > 0 && (
              <>
                <div
                  className={classNames(
                    "hidden md:grid",
                    ENVIRONMENT_VARIABLES_ROW_CLASSES,
                  )}
                >
                  <div
                    className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
                  >
                    <span className="text-xs text-content-secondary">
                      Domain
                    </span>
                  </div>
                  <div className="flex flex-col gap-1">
                    <span className="text-xs text-content-secondary">
                      Request Destination{" "}
                    </span>
                  </div>
                </div>
                <div className="divide-y divide-border-transparent border-t">
                  {vanityDomains
                    .sort((a, b) => a.creationTime - b.creationTime)
                    .reverse()
                    .map((domain, index) => (
                      <DisplayVanityDomain
                        key={index}
                        vanityDomain={domain}
                        enabled={hasEditAccess}
                      />
                    ))}
                </div>
              </>
            )}
            {deployment && canonicalCustomDomains && (
              <div className="border-t">
                <ProdProvider deploymentName={deployment.name}>
                  <CanonicalDomainForm
                    deploymentName={deployment.name}
                    vanityDomains={vanityDomains}
                  />
                </ProdProvider>
              </div>
            )}
          </div>
        )}
      </div>
      {!hasEntitlement && (
        <LocalDevCallout
          className="flex-col"
          tipText="Tip: Run this to enable custom domains locally:"
          command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement custom_domains_enabled --team-id ${team?.id} --reason "local" true --for-real`}
        />
      )}
    </Sheet>
  );
}

function ProdProvider({
  children,
  deploymentName,
}: {
  children: React.ReactNode;
  deploymentName: string;
}) {
  return (
    <DeploymentInfoProvider deploymentOverride={deploymentName}>
      <MaybeDeploymentApiProvider deploymentOverride={deploymentName}>
        <WaitForDeploymentApi sizeClass="hidden">
          {children}
        </WaitForDeploymentApi>
      </MaybeDeploymentApiProvider>
    </DeploymentInfoProvider>
  );
}

function CanonicalDomainForm({
  deploymentName,
  vanityDomains,
}: {
  deploymentName: string;
  vanityDomains?: VanityDomainResponse[];
}) {
  const deploymentUrl = useDeploymentUrl();
  const canonicalCloudUrl = useQuery(udfs.convexCloudUrl.default);
  const canonicalSiteUrl = useQuery(udfs.convexSiteUrl.default);
  const defaultSiteUrl = `https://${deploymentName}.convex.site`;

  return (
    <div className="flex flex-col gap-3">
      <h4 className="mt-3">Override Production Environment Variables</h4>
      <CanonicalUrlCombobox
        label={
          <span className="flex items-center gap-2">
            <code>process.env.CONVEX_CLOUD_URL</code>
            <Tooltip
              tip={
                <span>
                  This is also used by{" "}
                  <code>npx convex deploy --cmd "..."</code> to connect your
                  frontend to your Convex backend
                </span>
              }
            >
              <QuestionMarkCircledIcon />
            </Tooltip>
          </span>
        }
        defaultUrl={deploymentUrl}
        canonicalUrl={canonicalCloudUrl}
        vanityDomains={vanityDomains}
        requestDestination="convexCloud"
      />
      <CanonicalUrlCombobox
        label={
          <span className="flex flex-row items-center gap-1">
            <code>process.env.CONVEX_SITE_URL</code>
            <Tooltip
              tip={
                <span>
                  If you use Convex Auth, this would also be used in{" "}
                  <code>auth.config.ts</code> as the issuer
                </span>
              }
            >
              <QuestionMarkCircledIcon />
            </Tooltip>
          </span>
        }
        defaultUrl={defaultSiteUrl}
        canonicalUrl={canonicalSiteUrl}
        vanityDomains={vanityDomains}
        requestDestination="convexSite"
      />
    </div>
  );
}

function CanonicalUrlCombobox({
  label,
  defaultUrl,
  canonicalUrl,
  vanityDomains,
  requestDestination,
}: {
  label: React.ReactNode;
  defaultUrl: string;
  canonicalUrl: string | undefined;
  vanityDomains?: VanityDomainResponse[];
  requestDestination: "convexCloud" | "convexSite";
}) {
  const vanityDomainsForRequestDestination = useMemo(
    () =>
      vanityDomains?.filter(
        (v) => v.requestDestination === requestDestination,
      ) || [],
    [vanityDomains, requestDestination],
  );
  const isDisconnected =
    canonicalUrl !== undefined &&
    canonicalUrl !== defaultUrl &&
    !vanityDomainsForRequestDestination.some(
      (v) => `https://${v.domain}` === canonicalUrl,
    );
  const updateCanonicalUrl = useUpdateCanonicalUrl(
    requestDestination,
    defaultUrl,
  );
  const options = useMemo(
    () => [
      {
        label: `${defaultUrl} (default)`,
        value: defaultUrl,
      },
      ...(isDisconnected
        ? [
            {
              label: `${canonicalUrl} (disconnected)`,
              value: canonicalUrl,
            },
          ]
        : []),
      ...vanityDomainsForRequestDestination.map((v) => ({
        label: `https://${v.domain}${
          v.verificationTime ? "" : " (unverified)"
        }`,
        value: `https://${v.domain}`,
      })),
    ],
    [
      defaultUrl,
      canonicalUrl,
      isDisconnected,
      vanityDomainsForRequestDestination,
    ],
  );
  const disabled = options.length <= 1;

  return (
    <div className="flex flex-col gap-1">
      <Combobox
        label={label}
        labelHidden={false}
        disabled={disabled}
        buttonProps={{
          tip: disabled ? `Add a custom domain first` : undefined,
        }}
        buttonClasses="w-fit"
        optionsWidth="fit"
        options={options}
        selectedOption={canonicalUrl}
        setSelectedOption={async (value: string | null) => {
          if (value === null) {
            return;
          }
          await updateCanonicalUrl(value);
        }}
        disableSearch
      />
    </div>
  );
}

function VanityDomainForm({
  deploymentName,
  disabled,
}: {
  deploymentName: string;
  disabled?: boolean;
}) {
  const createVanityDomain = useCreateVanityDomain(deploymentName);
  const formState = useFormik<VanityDomainRequestArgs>({
    validateOnChange: true,
    initialValues: {
      domain: "",
      requestDestination: "convexSite",
    },
    validate: (values: { domain?: string }) => {
      const errors: Partial<VanityDomainRequestArgs> = {};
      if (
        !values.domain ||
        values.domain === "" ||
        !values.domain.includes(".")
      ) {
        errors.domain = "Enter a valid domain name";
      }
      return errors;
    },
    onSubmit: async (values: VanityDomainRequestArgs) => {
      await createVanityDomain({
        domain: values.domain,
        requestDestination: values.requestDestination,
      });
      formState.resetForm();
    },
  });

  return (
    <form
      className="mb-4 flex flex-col gap-2 overflow-x-clip pt-2 md:flex-row"
      onSubmit={(e) => {
        e.preventDefault();
        formState.handleSubmit();
      }}
    >
      <VanityDomainInputs
        formState={formState}
        disabled={disabled}
        deploymentName={deploymentName}
      />
      <Button
        className="flex w-fit"
        type="submit"
        color="primary"
        disabled={
          disabled ||
          formState.isSubmitting ||
          !formState.isValid ||
          formState.values.domain === ""
        }
        tip={
          disabled
            ? "You do not have permission to add custom domains"
            : undefined
        }
        icon={<PlusIcon />}
        variant="primary"
      >
        Add Domain
      </Button>
    </form>
  );
}

export function VanityDomainInputs({
  formState,
  deploymentName,
  disabled = false,
}: {
  formState: ReturnType<typeof useFormik<VanityDomainRequestArgs>>;
  deploymentName: string;
  disabled?: boolean;
}) {
  return (
    <div className="flex w-full flex-col gap-2 md:flex-row">
      <div className="flex grow flex-col gap-1">
        <TextInput
          placeholder="Custom domain URL"
          error={formState.errors.domain}
          onChange={formState.handleChange}
          value={formState.values.domain}
          id="domain"
          labelHidden
          disabled={disabled}
        />
      </div>
      <Combobox
        buttonClasses="w-fit"
        optionsWidth="full"
        label="Request Destination"
        options={[
          {
            label: `HTTP Actions (${deploymentName}.convex.site)`,
            value: "convexSite",
          },
          {
            label: `Convex API (${deploymentName}.convex.cloud)`,
            value: "convexCloud",
          },
        ]}
        selectedOption={formState.values.requestDestination}
        setSelectedOption={async (
          value: "convexSite" | "convexCloud" | null,
        ) => {
          if (value === null) {
            return;
          }
          await formState.setFieldValue("requestDestination", value);
        }}
        disableSearch
        disabled={disabled}
      />
    </div>
  );
}

function DisplayVanityDomain({
  vanityDomain,
  enabled,
}: {
  vanityDomain: VanityDomainResponse;
  enabled: boolean;
}) {
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  return (
    <div className="flex flex-col">
      <div className={ENVIRONMENT_VARIABLES_ROW_CLASSES}>
        <div
          className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
        >
          <div className="flex h-[2.375rem] items-center truncate text-content-primary md:col-span-1">
            {vanityDomain.domain}
            {vanityDomain.verificationTime && (
              <Tooltip
                tip="This domain is verified and can receive traffic."
                side="right"
              >
                <CheckCircledIcon className="m-3 text-green-700 dark:text-green-200" />
              </Tooltip>
            )}
          </div>
        </div>

        <div className="flex h-[2.375rem] items-center gap-1">
          <span className=" truncate text-content-primary">
            {vanityDomain.requestDestination === "convexCloud"
              ? "Convex API"
              : "HTTP Actions"}
          </span>
        </div>
        <div className="flex items-center justify-end gap-2">
          <Button
            tip={
              !enabled
                ? "You do not have permission to delete custom domains."
                : "Delete"
            }
            type="button"
            onClick={() => setShowDeleteModal(true)}
            variant="danger"
            size="sm"
            inline
            icon={<TrashIcon />}
            disabled={!enabled}
          />
          {showDeleteModal && (
            <DeleteDomainModal
              onClose={() => setShowDeleteModal(false)}
              domain={vanityDomain}
            />
          )}
        </div>
      </div>
      {!vanityDomain.verificationTs && (
        <>
          <Callout className="mb-4 mt-0 w-72 gap-2 align-middle">
            <div className="ml-1 flex w-full gap-3">
              <ExclamationTriangleIcon className="mt-1" />
              <span className="font-semibold">
                This domain is not verified yet.
              </span>
            </div>
          </Callout>
          <span className="mb-4 font-semibold">
            Set the following records on your DNS provider:
          </span>
          <div className="rounded border p-2">
            <div className="grid grid-cols-1 p-2 md:grid md:grid-cols-[2fr_6fr_3fr] md:gap-2">
              {/* Header */}
              {["Type", "Name", "Value"].map((header) => (
                <div className="hidden font-semibold text-content-secondary md:block">
                  {header}
                </div>
              ))}

              {/* Records */}
              <code className="truncate break-words font-bold md:font-normal">
                CNAME
              </code>
              <code className="truncate break-words">
                {vanityDomain.domain}
              </code>
              <code className="truncate break-words">convex.domains</code>

              <code className="truncate break-words font-bold md:font-normal">
                TXT
              </code>
              <code className="truncate break-words">
                _convex_domains.{vanityDomain.domain}
              </code>
              <code className="truncate break-words">
                {vanityDomain.instanceName}
              </code>
            </div>
          </div>
          <span className="my-4 font-light">
            It may take up to 30 minutes to verify your domain and start serving
            traffic.
          </span>
        </>
      )}
    </div>
  );
}

function DeleteDomainModal({
  domain,
  onClose,
}: {
  domain: VanityDomainResponse;
  onClose: () => void;
}) {
  const deleteVanityDomain = useDeleteVanityDomain(domain.instanceName);
  const handleDelete = async () => {
    await deleteVanityDomain({
      domain: domain.domain,
      requestDestination: domain.requestDestination,
    });
  };

  return (
    <ConfirmationDialog
      onClose={onClose}
      onConfirm={handleDelete}
      confirmText="Delete"
      dialogTitle="Delete Custom Domain"
      variant="danger"
      dialogBody={
        <>
          Are you sure you want to delete the custom domain{" "}
          <span className="font-semibold">{domain.domain}</span>?
        </>
      }
    />
  );
}
