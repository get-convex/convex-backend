import { Team } from "generatedApi";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Spinner } from "@ui/Spinner";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import {
  useTeamEntitlements,
  useGetSSO,
  useEnableSSO,
  useDisableSSO,
  useGenerateSSOConfigurationLink,
} from "api/teams";
import { useState } from "react";
import { Tooltip } from "@ui/Tooltip";
import {
  QuestionMarkCircledIcon,
  ExclamationTriangleIcon,
} from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import { useProfileEmails } from "api/profile";
import Link from "next/link";
import { LoadingTransition } from "@ui/Loading";

function ManageDomainsButton({
  onClick,
  disabled,
  loading,
  variant,
}: {
  onClick: () => Promise<void>;
  disabled: boolean;
  loading: boolean;
  variant: "primary" | "neutral";
}) {
  return (
    <Button
      variant={variant}
      className="w-fit"
      size="sm"
      loading={loading}
      onClick={onClick}
      disabled={disabled}
    >
      Manage domains
    </Button>
  );
}

function ManageSSOConfigurationButton({
  onClick,
  disabled,
  loading,
  variant,
  tooltip,
}: {
  onClick: () => Promise<void>;
  disabled: boolean;
  loading: boolean;
  variant: "primary" | "neutral";
  tooltip?: string;
}) {
  const button = (
    <Button
      variant={variant}
      className="w-fit"
      size="sm"
      loading={loading}
      onClick={onClick}
      disabled={disabled}
    >
      Manage SSO configuration
    </Button>
  );

  if (tooltip) {
    return <Tooltip tip={tooltip}>{button}</Tooltip>;
  }

  return button;
}

export function TeamSSO({ team }: { team: Team }) {
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();
  const entitlements = useTeamEntitlements(team.id);
  const { data: ssoOrganization, isLoading: isSSOLoading } = useGetSSO(team.id);
  const enableSSO = useEnableSSO(team.id);
  const disableSSO = useDisableSSO(team.id);
  const generateSSOConfigurationLink = useGenerateSSOConfigurationLink(team.id);
  const profileEmails = useProfileEmails();

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isGeneratingDomainsLink, setIsGeneratingDomainsLink] = useState(false);
  const [isGeneratingSSOLink, setIsGeneratingSSOLink] = useState(false);
  const [showDisableConfirmation, setShowDisableConfirmation] = useState(false);
  const [disableError, setDisableError] = useState<string>();

  const isGeneratingAnyLink = isGeneratingDomainsLink || isGeneratingSSOLink;

  const ssoEnabled = entitlements?.ssoEnabled ?? false;
  const isSSOConfigured = !!ssoOrganization;
  const domains = ssoOrganization?.domains ?? [];

  // Determine if any domain needs verification
  const hasFailedDomain = domains.some((d) => d.state === "failed");
  const hasVerifiedDomain = domains.some(
    (d) => d.state === "verified" || d.state === "legacyVerified",
  );

  return (
    <>
      <h2>Single Sign-On (SSO)</h2>

      {entitlements && !ssoEnabled && (
        <Callout variant="upsell">
          SSO is not available on your plan. Upgrade your plan to use SSO.
        </Callout>
      )}

      {isSSOConfigured && hasFailedDomain && (
        <Callout variant="error">
          Domain verification failed for:{" "}
          {domains
            .filter((d) => d.state === "failed")
            .map((d) => d.domain)
            .join(", ")}
          . Please verify your domain(s) to continue using SSO.
        </Callout>
      )}

      <Sheet>
        <h3 className="mb-2">Configuration</h3>
        <p className="mb-4 text-xs text-content-secondary">
          Configure Single Sign-On (SSO) for your team to enable secure
          authentication through your identity provider.
        </p>

        <LoadingTransition>
          {!isSSOLoading && (
            <div className="flex flex-col">
              <Tooltip
                tip={
                  !hasAdminPermissions
                    ? "You do not have permission to change SSO settings."
                    : !ssoEnabled
                      ? "SSO is not available on your plan."
                      : undefined
                }
              >
                <label className="ml-px flex items-center gap-2 text-sm">
                  <Checkbox
                    checked={isSSOConfigured}
                    disabled={
                      isSubmitting || !hasAdminPermissions || !ssoEnabled
                    }
                    onChange={async () => {
                      if (isSSOConfigured) {
                        setShowDisableConfirmation(true);
                      } else {
                        // Enable SSO
                        setIsSubmitting(true);
                        try {
                          await enableSSO({});
                        } finally {
                          setIsSubmitting(false);
                        }
                      }
                    }}
                  />
                  Enable SSO
                  {isSubmitting && (
                    <div>
                      <Spinner />
                    </div>
                  )}
                </label>
              </Tooltip>

              {isSSOConfigured && (
                <div className="mt-5 space-y-4">
                  <div className="flex flex-col gap-4">
                    {domains.length > 0 && (
                      <div className="flex flex-col gap-2">
                        <span className="flex items-center gap-1 text-sm font-semibold">
                          {domains.length === 1
                            ? "Current Domain"
                            : "Current Domains"}
                          <Tooltip
                            tip="You may remove all domains by disable and re-enabling SSO. This will require re-configuring SSO."
                            side="right"
                          >
                            <QuestionMarkCircledIcon />
                          </Tooltip>
                        </span>
                        <div className="flex flex-col gap-1">
                          {domains.map((d) => {
                            const isVerified =
                              d.state === "verified" ||
                              d.state === "legacyVerified";
                            const isPending = d.state === "pending";
                            const isError = d.state === "failed";

                            const tooltipText = isVerified
                              ? "This domain has been verified and may be used with SSO"
                              : isPending
                                ? "This domain has not yet completed verification. Check the status by clicking 'Verify domain' below."
                                : "An error occured verifying this domain. Check the status by clicking 'Verify domain' below.";

                            // Check if user has a verified email for this domain
                            const hasVerifiedEmailForDomain =
                              profileEmails?.some((email) => {
                                if (!email.isVerified) return false;
                                const emailDomain = email.email
                                  .split("@")[1]
                                  ?.toLowerCase();
                                return emailDomain === d.domain.toLowerCase();
                              });

                            return (
                              <div
                                key={d.id}
                                className="flex items-center gap-2"
                              >
                                <span>{d.domain}</span>
                                <Tooltip tip={tooltipText} side="right">
                                  <span
                                    className={cn(
                                      "rounded-full border px-2 py-0.5 text-xs",
                                      isVerified &&
                                        "bg-background-success text-content-success",
                                      isPending &&
                                        "bg-background-warning text-content-warning",
                                      isError &&
                                        "bg-background-error text-content-error",
                                    )}
                                  >
                                    {isVerified
                                      ? "Verified"
                                      : isPending
                                        ? "Pending"
                                        : "Error"}
                                  </span>
                                </Tooltip>
                                {!hasVerifiedEmailForDomain && (
                                  <Tooltip
                                    tip={
                                      <div className="flex flex-col gap-1">
                                        <span>
                                          You do not have a verified email on
                                          your Convex account matching this
                                          domain. If you do not add your email,
                                          you will not be able to log in with
                                          SSO with this domain.
                                        </span>
                                        <Link
                                          href="/profile"
                                          className="text-content-link hover:underline"
                                        >
                                          You may verify an email on the profile
                                          page.
                                        </Link>
                                      </div>
                                    }
                                    side="right"
                                  >
                                    <span className="flex items-center rounded-md border bg-background-warning p-1 text-content-warning">
                                      <ExclamationTriangleIcon className="text-content-warning" />
                                    </span>
                                  </Tooltip>
                                )}
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    )}
                    <div className="flex gap-2">
                      <ManageDomainsButton
                        variant="neutral"
                        loading={isGeneratingDomainsLink}
                        onClick={async () => {
                          setIsGeneratingDomainsLink(true);
                          try {
                            const result = await generateSSOConfigurationLink({
                              intent: "domainVerification",
                            });
                            if (result?.link) {
                              window.open(result.link, "_blank");
                            }
                          } finally {
                            setIsGeneratingDomainsLink(false);
                          }
                        }}
                        disabled={
                          isSubmitting ||
                          !hasAdminPermissions ||
                          isGeneratingAnyLink
                        }
                      />
                      <ManageSSOConfigurationButton
                        variant="primary"
                        loading={isGeneratingSSOLink}
                        onClick={async () => {
                          setIsGeneratingSSOLink(true);
                          try {
                            const result = await generateSSOConfigurationLink({
                              intent: "sso",
                            });
                            if (result?.link) {
                              window.open(result.link, "_blank");
                            }
                          } finally {
                            setIsGeneratingSSOLink(false);
                          }
                        }}
                        disabled={
                          isSubmitting ||
                          !hasAdminPermissions ||
                          !hasVerifiedDomain ||
                          isGeneratingAnyLink
                        }
                        tooltip={
                          !hasVerifiedDomain
                            ? "You must verify at least ene domain before managing the SSO configuration."
                            : undefined
                        }
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </LoadingTransition>
      </Sheet>

      {showDisableConfirmation && (
        <ConfirmationDialog
          onClose={() => {
            setShowDisableConfirmation(false);
            setDisableError(undefined);
          }}
          onConfirm={async () => {
            setIsSubmitting(true);
            try {
              await disableSSO();
              setShowDisableConfirmation(false);
            } catch (e: any) {
              setDisableError(e.message);
              throw e;
            } finally {
              setIsSubmitting(false);
            }
          }}
          confirmText="Disable"
          variant="danger"
          dialogTitle="Disable Single Sign-On"
          dialogBody="Disabling Single Sign-on will remove all configuration related to SSO. You will need to re-configure all settings to re-enable SSO."
          error={disableError}
          validationText="DISABLE SSO"
        />
      )}
    </>
  );
}
