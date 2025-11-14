import {
  CheckCircledIcon,
  ExclamationTriangleIcon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Combobox, Option } from "@ui/Combobox";
import { LocalDevCallout } from "@common/elements/LocalDevCallout";
import { Callout } from "@ui/Callout";
import { captureMessage } from "@sentry/nextjs";
import {
  ENVIRONMENT_VARIABLES_ROW_CLASSES,
  ENVIRONMENT_VARIABLE_NAME_COLUMN,
} from "@common/features/settings/components/EnvironmentVariables";
import { Sheet } from "@ui/Sheet";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TextInput } from "@ui/TextInput";
import { useFormik } from "formik";
import { useDeployments } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useHasProjectAdminPermissions } from "api/roles";
import Link from "next/link";
import { useState, useMemo } from "react";
import {
  TeamResponse,
  PlatformDeleteCustomDomainArgs,
  VanityDomainResponse,
} from "generatedApi";
import {
  useListVanityDomains,
  useCreateVanityDomain,
  useDeleteVanityDomain,
} from "api/vanityDomains";
import { useDeploymentUrl } from "@common/lib/deploymentApi";
import { DeploymentInfoProvider } from "providers/DeploymentInfoProvider";
import { MaybeDeploymentApiProvider } from "providers/MaybeDeploymentApiProvider";
import { WaitForDeploymentApi } from "@common/lib/deploymentContext";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { useUpdateCanonicalUrl } from "hooks/deploymentApi";
import { Loading } from "@ui/Loading";

export function CustomDomains({
  team,
  hasEntitlement,
}: {
  team: TeamResponse;
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

        <div>
          {!hasEntitlement && (
            <>
              <Callout>
                <div>
                  Custom domains are{" "}
                  <span className="font-semibold">
                    only available on the Pro plan
                  </span>
                  .{" "}
                  <Link
                    href={`/${team?.slug}/settings/billing`}
                    className="underline"
                  >
                    Upgrade to get access.
                  </Link>
                </div>
              </Callout>
              <LocalDevCallout
                className="flex-col"
                tipText="Tip: Run this to enable custom domains locally:"
                command={`cargo run --bin big-brain-tool -- --dev grant-entitlement --team-entitlement custom_domains_enabled --team-id ${team?.id} --reason "local" true --for-real`}
              />
            </>
          )}

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
                  <span className="text-xs text-content-secondary">Domain</span>
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
          {deployment && (
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
      </div>
    </Sheet>
  );
}

export function ProdProvider({
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

  return (
    <div className="flex flex-col gap-3">
      <h4 className="mt-3">Override Production Environment Variables</h4>
      <p className="max-w-prose">
        Replace your production environment variables everywhere in your app.
        All internal references to your <code>CONVEX_SITE_URL</code> and{" "}
        <code>CONVEX_CLOUD_URL</code> will change to the url you set. This can
        affect WebSocket and HTTP clients, storage urls, and Convex Auth.
      </p>
      <CanonicalUrlCombobox
        label={
          <span className="flex items-center gap-2">
            <code>process.env.CONVEX_CLOUD_URL</code>
          </span>
        }
        defaultUrl={{ kind: "default", url: deploymentUrl }}
        canonicalUrl={
          canonicalCloudUrl === undefined
            ? { kind: "loading" }
            : { kind: "loaded", url: canonicalCloudUrl }
        }
        vanityDomains={vanityDomains}
        requestDestination="convexCloud"
      />
      <CanonicalUrlCombobox
        label={
          <span className="flex flex-row items-center gap-1">
            <code>process.env.CONVEX_SITE_URL</code>
          </span>
        }
        defaultUrl={
          deploymentUrl === `https://${deploymentName}.convex.cloud`
            ? { kind: "default", url: `https://${deploymentName}.convex.site` }
            : { kind: "unknownDefault" }
        }
        canonicalUrl={
          canonicalSiteUrl === undefined
            ? { kind: "loading" }
            : { kind: "loaded", url: canonicalSiteUrl }
        }
        vanityDomains={vanityDomains}
        requestDestination="convexSite"
      />
    </div>
  );
}

type DefaultUrlOption =
  | {
      // The default url for this deployment, before any overrides were applied.
      kind: "default";
      url: string;
    }
  | {
      // In this case, we don't know what the default url should be.
      // In some cases the deployment doesn't expose its default url through an API.
      kind: "unknownDefault";
    };

type UrlOption =
  | {
      // Either a default url or a canonical url that has yet to be loaded
      // from the deployment. This should be a transient state.
      kind: "loading";
    }
  | DefaultUrlOption
  | {
      // The current canonical url for the deployment, which doesn't match the
      // default or any custom domains.
      // If you switch away from this URL, you probably won't be able to switch
      // back to it.
      kind: "disconnectedCanonical";
      url: string;
    }
  | {
      kind: "custom";
      url: string;
    };

type CanonicalUrl =
  | {
      kind: "loading";
    }
  | {
      kind: "loaded";
      url: string;
    };

export function CanonicalUrlCombobox({
  label,
  defaultUrl,
  canonicalUrl,
  vanityDomains,
  requestDestination,
}: {
  label: React.ReactNode;
  defaultUrl: DefaultUrlOption;
  canonicalUrl: CanonicalUrl;
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
  const canonicalIsKnownDefault =
    canonicalUrl.kind === "loaded" &&
    defaultUrl.kind === "default" &&
    canonicalUrl.url === defaultUrl.url;
  const canonicalIsCustom =
    canonicalUrl.kind === "loaded" &&
    vanityDomainsForRequestDestination.some(
      (v) => `https://${v.domain}` === canonicalUrl.url,
    );
  // If we don't know what the default should be, and we don't recognize the canonical URL,
  // assume that the canonical URL is the default.
  const canonicalIsUnknownDefault =
    canonicalUrl.kind === "loaded" &&
    defaultUrl.kind === "unknownDefault" &&
    !canonicalIsCustom;
  const canonicalIsDefault =
    canonicalIsKnownDefault || canonicalIsUnknownDefault;
  const effectiveDefaultUrl: UrlOption = useMemo(() => {
    if (canonicalIsUnknownDefault) {
      return { kind: "default", url: canonicalUrl.url };
    }
    return defaultUrl;
  }, [canonicalIsUnknownDefault, canonicalUrl, defaultUrl]);
  const canonicalUrlOption: UrlOption = useMemo(() => {
    if (canonicalUrl.kind === "loading") {
      return { kind: "loading" };
    }
    if (canonicalIsCustom) {
      return { kind: "custom", url: canonicalUrl.url };
    }
    if (canonicalIsDefault) {
      return { kind: "default", url: canonicalUrl.url };
    }
    return { kind: "disconnectedCanonical", url: canonicalUrl.url };
  }, [canonicalIsCustom, canonicalIsDefault, canonicalUrl]);
  const updateCanonicalUrl = useUpdateCanonicalUrl(requestDestination);
  const options: Option<UrlOption>[] = useMemo(
    () => [
      {
        label:
          effectiveDefaultUrl.kind === "default"
            ? `${effectiveDefaultUrl.url} (default)`
            : "default",
        value: effectiveDefaultUrl,
      },
      ...(canonicalUrlOption.kind === "disconnectedCanonical"
        ? [
            {
              label: `${canonicalUrlOption.url} (disconnected)`,
              value: canonicalUrlOption,
            },
          ]
        : []),
      ...vanityDomainsForRequestDestination.map((v) => ({
        label: `https://${v.domain}${
          v.verificationTime ? "" : " (unverified)"
        }`,
        value: { kind: "custom" as const, url: `https://${v.domain}` },
      })),
    ],
    [
      effectiveDefaultUrl,
      canonicalUrlOption,
      vanityDomainsForRequestDestination,
    ],
  );
  const disabled = options.length <= 1;

  if (canonicalUrlOption.kind === "loading") {
    return <Loading className="h-8 w-full" fullHeight={false} />;
  }

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
        selectedOption={canonicalUrlOption}
        setSelectedOption={async (value: UrlOption | null) => {
          if (value === null || value.kind === "loading") {
            captureMessage(
              "Unexpected value selected in CanonicalUrlCombobox",
              "error",
            );
            return;
          }
          await updateCanonicalUrl(
            value.kind === "default" || value.kind === "unknownDefault"
              ? null
              : value.url,
          );
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
  const formState = useFormik<PlatformDeleteCustomDomainArgs>({
    validateOnChange: true,
    initialValues: {
      domain: "",
      requestDestination: "convexSite",
    },
    validate: (values: { domain?: string }) => {
      const errors: Partial<PlatformDeleteCustomDomainArgs> = {};
      if (
        !values.domain ||
        values.domain === "" ||
        !values.domain.includes(".")
      ) {
        errors.domain = "Enter a valid domain name";
      }
      return errors;
    },
    onSubmit: async (values: PlatformDeleteCustomDomainArgs) => {
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
  formState: ReturnType<typeof useFormik<PlatformDeleteCustomDomainArgs>>;
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
          <span className="truncate text-content-primary">
            {vanityDomain.requestDestination === "convexCloud"
              ? "Convex API"
              : "HTTP Actions"}
          </span>
        </div>
        <div className="flex h-full items-center justify-end gap-2">
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
      {!vanityDomain.verificationTime && (
        <>
          <Callout className="mt-0 mb-4 w-72 gap-2 align-middle">
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
          <div className="rounded-sm border p-2">
            <div className="grid grid-cols-1 p-2 md:grid md:grid-cols-[2fr_6fr_3fr] md:gap-2">
              {/* Header */}
              {["Type", "Name", "Value"].map((header) => (
                <div className="hidden font-semibold text-content-secondary md:block">
                  {header}
                </div>
              ))}

              {/* Records */}
              <code className="truncate font-bold break-words md:font-normal">
                CNAME
              </code>
              <code className="truncate break-words">
                {vanityDomain.domain}
              </code>
              <code className="truncate break-words">convex.domains</code>

              <code className="truncate font-bold break-words md:font-normal">
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
