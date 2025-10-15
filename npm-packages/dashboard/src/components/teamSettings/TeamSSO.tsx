import { Team } from "generatedApi";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { Checkbox } from "@ui/Checkbox";
import { Spinner } from "@ui/Spinner";
import { TextInput } from "@ui/TextInput";
import { Button } from "@ui/Button";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import {
  useTeamEntitlements,
  useGetSSO,
  useEnableSSO,
  useUpdateSSODomain,
  useDisableSSO,
} from "api/teams";
import { useState, useMemo } from "react";
import { Tooltip } from "@ui/Tooltip";
import { useProfileEmails } from "api/profile";

export function TeamSSO({ team }: { team: Team }) {
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();
  const entitlements = useTeamEntitlements(team.id);
  const ssoOrganization = useGetSSO(team.id);
  const enableSSO = useEnableSSO(team.id);
  const updateSSODomain = useUpdateSSODomain(team.id);
  const disableSSO = useDisableSSO(team.id);
  const profileEmails = useProfileEmails();

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showDomainForm, setShowDomainForm] = useState(false);
  const [domain, setDomain] = useState("");
  const [domainError, setDomainError] = useState<React.ReactNode>(null);

  const ssoEnabled = entitlements?.ssoEnabled ?? false;
  const isSSOConfigured = !!ssoOrganization;
  const currentDomain = ssoOrganization?.domains?.[0]?.domain;

  // Extract verified domains from team member emails
  const verifiedDomains = useMemo(() => {
    if (!profileEmails) return new Set<string>();
    const domains = new Set<string>();
    profileEmails.forEach(({ email, isVerified }) => {
      const emailDomain = email.split("@")[1];
      if (emailDomain && isVerified) {
        domains.add(emailDomain.toLowerCase());
      }
    });
    return domains;
  }, [profileEmails]);

  const handleDomainFormSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedDomain = domain.trim().toLowerCase();

    if (!trimmedDomain) {
      setDomainError("Domain is required");
      return;
    }

    // Validate that domain matches a verified email domain
    if (!verifiedDomains.has(trimmedDomain)) {
      setDomainError(
        <div>
          The domain "{trimmedDomain}" does not match any verified email
          addresses on your account.{" "}
          <a
            href="/profile"
            target="_blank"
            rel="noopener noreferrer"
            className="text-content-link underline"
          >
            Add a verified email
          </a>{" "}
          with this domain before setting up SSO.
        </div>,
      );
      return;
    }

    setDomainError(null);
    setIsSubmitting(true);
    try {
      if (isSSOConfigured) {
        await updateSSODomain({ domain: trimmedDomain });
      } else {
        await enableSSO({ domain: trimmedDomain });
      }
      setShowDomainForm(false);
      setDomain("");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <>
      <h2>Single Sign-On (SSO)</h2>

      {!ssoEnabled && (
        <Callout variant="upsell">
          SSO is not available on your plan. Upgrade your plan to use SSO.
        </Callout>
      )}

      <Sheet>
        <h3 className="mb-2">Configuration</h3>
        <p className="mb-4 text-xs text-content-secondary">
          Configure Single Sign-On (SSO) for your team to enable secure
          authentication through your identity provider.
        </p>

        <Tooltip
          tip={
            !hasAdminPermissions
              ? "You do not have permission to change SSO settings."
              : !ssoEnabled
                ? "SSO is not available on your plan."
                : undefined
          }
        >
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={isSSOConfigured || showDomainForm}
              disabled={isSubmitting || !hasAdminPermissions || !ssoEnabled}
              onChange={async () => {
                if (isSSOConfigured) {
                  setIsSubmitting(true);
                  try {
                    await disableSSO();
                    setShowDomainForm(false);
                    setDomain("");
                  } finally {
                    setIsSubmitting(false);
                  }
                } else if (showDomainForm) {
                  setShowDomainForm(false);
                  setDomain("");
                } else {
                  setShowDomainForm(true);
                  setDomain("");
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

        {(showDomainForm || isSSOConfigured) && (
          <div className="mt-6 space-y-4">
            {isSSOConfigured && !showDomainForm && (
              <div className="flex flex-col gap-2">
                {currentDomain && (
                  <span>
                    Current domain:{" "}
                    <span className="font-semibold">{currentDomain}</span>
                  </span>
                )}
                <Button
                  variant="neutral"
                  className="w-fit"
                  size="sm"
                  onClick={() => {
                    setShowDomainForm(true);
                    setDomain(currentDomain || "");
                  }}
                  disabled={isSubmitting || !hasAdminPermissions}
                >
                  {currentDomain ? "Change SSO Domain" : "Set SSO Domain"}
                </Button>
              </div>
            )}

            {showDomainForm && (
              <form
                className="max-w-[30rem] space-y-4"
                onSubmit={handleDomainFormSubmit}
              >
                <TextInput
                  autoFocus
                  id="sso-domain"
                  label="Domain"
                  value={domain}
                  onChange={(e) => {
                    setDomain(e.target.value);
                    setDomainError(null);
                  }}
                  placeholder={currentDomain || "example.com"}
                  disabled={isSubmitting}
                  description="Enter the domain your team's members will use to login with SSO."
                  error={domainError}
                />
                <div className="flex gap-2">
                  <Button
                    variant="neutral"
                    onClick={() => {
                      setShowDomainForm(false);
                      setDomain("");
                      setDomainError(null);
                    }}
                    disabled={isSubmitting}
                  >
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    variant="primary"
                    disabled={!domain.trim() || isSubmitting}
                  >
                    Save
                  </Button>
                </div>
              </form>
            )}
          </div>
        )}
      </Sheet>
    </>
  );
}
