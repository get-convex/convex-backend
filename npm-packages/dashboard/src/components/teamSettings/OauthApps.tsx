import React, { useState } from "react";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { OauthAppResponse } from "generatedApi";
import {
  PlusIcon,
  EyeNoneIcon,
  EyeOpenIcon,
  InfoCircledIcon,
  DotsVerticalIcon,
  QuestionMarkCircledIcon,
  ExternalLinkIcon,
} from "@radix-ui/react-icons";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import {
  useTeamOauthApps,
  useUpdateOauthApp,
  useRegisterOauthApp,
  useDeleteOauthApp,
} from "api/oauth";
import { Modal } from "@ui/Modal";
import { Formik } from "formik";
import * as Yup from "yup";
import { LoadingTransition } from "@ui/Loading";
import { CopyTextButton } from "dashboard-common/src/elements/CopyTextButton";
import { Tooltip } from "@ui/Tooltip";
import { Menu, MenuItem } from "@ui/Menu";
import { cn } from "@ui/cn";
import { toast } from "@common/lib/utils";

import { captureException, captureMessage } from "@sentry/nextjs";
import { useAuth0 } from "hooks/useAuth0";
import { useProfile } from "api/profile";
import Link from "next/link";

// Utility function to validate URLs without side effects
function isValidOauthRedirectUri(uri: string): boolean {
  try {
    const url = new URL(uri);
    // Only allow http and https
    if (!["http:", "https:"].includes(url.protocol)) return false;
    // Disallow fragments
    if (url.hash && url.hash !== "") return false;
    // Must have a hostname
    if (!url.hostname) return false;
    // Allow localhost and private IPs (no extra check needed)
    return true;
  } catch {
    return false;
  }
}

// Shared validation schema for both create and edit
const OAUTH_APP_SCHEMA = Yup.object({
  appName: Yup.string()
    .min(3, "App name must be at least 3 characters")
    .max(128, "App name must be at most 128 characters")
    .required("App name is required"),
  redirectUris: Yup.string()
    .required("At least one redirect URI is required")
    .test(
      "uris",
      "Must provide between 1 and 20 comma-delimited URIs",
      (value) => {
        if (!value) return false;
        const uris = value
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        return uris.length >= 1 && uris.length <= 20;
      },
    )
    .test("valid-urls", function validateUrls(value: string | undefined) {
      if (!value)
        return this.createError({
          message: "At least one redirect URI is required",
        });
      const uris = value
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      for (const uri of uris) {
        if (!isValidOauthRedirectUri(uri)) {
          return this.createError({
            message: `Redirect URI is not valid: ${uri}`,
          });
        }
      }
      return true;
    }),
});

// Validation schema for verification request form
const VERIFICATION_REQUEST_SCHEMA = Yup.object({
  description: Yup.string()
    .min(10, "Description must be at least 10 characters")
    .max(2000, "Description must be at most 2000 characters")
    .required("Description is required"),
});

// Handler for verification request submission
async function handleVerificationRequest(
  description: string,
  app: OauthAppResponse,
  teamId: number,
  setVerificationError: (error: string) => void,
  setVerificationLoading: (loading: boolean) => void,
  setVerificationModalOpen: (open: boolean) => void,
): Promise<void> {
  setVerificationError("");
  setVerificationLoading(true);

  try {
    const body = JSON.stringify({
      subject: `OAuth App Verification Request: ${app.appName}`,
      message: `OAuth App Verification Request

Team ID: ${teamId}
Client ID: ${app.clientId}
App Name: ${app.appName}

Description:
${description}

Please review this OAuth application for verification.`,
      teamId,
    });

    const resp = await fetch("/api/contact-form", {
      method: "POST",
      body,
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (!resp.ok) {
      try {
        if (resp.status < 500 || resp.status >= 400) {
          const { error } = await resp.json();
          captureMessage(error);
        }
      } catch (e) {
        captureException(e);
      }

      toast(
        "error",
        "Failed to send verification request. Please try again or email us at support@convex.dev",
      );
      return;
    }

    setVerificationModalOpen(false);
    toast("success", "Verification request sent!");
  } catch (err: any) {
    setVerificationError(err?.message || "Failed to send verification request");
  } finally {
    setVerificationLoading(false);
  }
}

// Move OauthAppForm to the top of the file so it is in scope for all usages
function OauthAppForm({
  initialValues,
  validationSchema,
  onSubmit,
  submitLabel,
  loading,
  error,
  isVerified,
}: {
  initialValues: { appName: string; redirectUris: string };
  validationSchema: any;
  onSubmit: (
    values: { appName: string; redirectUris: string },
    helpers: {
      setSubmitting: (isSubmitting: boolean) => void;
      resetForm?: () => void;
    },
  ) => Promise<void>;
  submitLabel: string;
  loading: boolean;
  error?: string;
  isVerified: boolean;
}) {
  return (
    <Formik
      initialValues={initialValues}
      validationSchema={validationSchema}
      onSubmit={onSubmit}
      enableReinitialize
    >
      {({
        values,
        errors,
        touched,
        handleChange,
        handleSubmit,
        isSubmitting,
      }) => (
        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <TextInput
            id="app-name"
            name="appName"
            label="Application Name"
            disabled={isVerified}
            placeholder="My OAuth App"
            value={values.appName}
            onChange={handleChange}
            error={
              touched.appName && typeof errors.appName === "string"
                ? errors.appName
                : undefined
            }
            required
          />
          <label
            htmlFor="redirect-uris"
            className="flex flex-col gap-1 text-sm text-content-primary"
          >
            Redirect URIs (seperated by commas)
            <textarea
              id="redirect-uris"
              name="redirectUris"
              className="h-24 resize-y rounded-sm border bg-background-secondary px-4 py-2 text-content-primary placeholder:text-content-tertiary focus:border-border-selected focus:outline-hidden"
              value={values.redirectUris}
              onChange={handleChange}
              placeholder="https://example.com/callback, http://localhost:1337/callback"
              required
            />
            {touched.redirectUris &&
              typeof errors.redirectUris === "string" && (
                <p className="text-xs text-content-errorSecondary" role="alert">
                  {errors.redirectUris}
                </p>
              )}
          </label>
          <div className="mt-2 flex items-center justify-between">
            {error && (
              <span className="mr-2 text-xs text-content-errorSecondary">
                {error}
              </span>
            )}
            <Button
              type="submit"
              className="ml-auto"
              loading={isSubmitting || loading}
            >
              {submitLabel}
            </Button>
          </div>
        </form>
      )}
    </Formik>
  );
}

// Verification request form component
function VerificationRequestForm({
  app,
  onSubmit,
  loading,
  error,
}: {
  app: OauthAppResponse;
  onSubmit: (description: string) => Promise<void>;
  loading: boolean;
  error?: string;
}) {
  const { user } = useAuth0();
  const profile = useProfile();
  const userEmail = profile?.email || user?.email;

  return (
    <Formik
      initialValues={{ description: "" }}
      validationSchema={VERIFICATION_REQUEST_SCHEMA}
      onSubmit={async (values, { setSubmitting }) => {
        await onSubmit(values.description);
        setSubmitting(false);
      }}
    >
      {({
        values,
        errors,
        touched,
        handleChange,
        handleSubmit,
        isSubmitting,
      }) => (
        <form className="flex flex-col gap-2" onSubmit={handleSubmit}>
          <div className="space-y-2 text-sm">
            <div>
              <span>Application Name:</span>
              <span className="ml-1 font-semibold text-content-primary">
                {app.appName}
              </span>
              <span className="ml-2 text-content-tertiary">
                (cannot be changed after verification)
              </span>
            </div>
          </div>

          <label
            htmlFor="description"
            className="flex flex-col gap-1 text-sm text-content-primary"
          >
            Application Description
            <textarea
              id="description"
              name="description"
              className="h-48 resize-y rounded-sm border bg-background-secondary px-4 py-2 text-content-primary placeholder:text-content-tertiary focus:border-border-selected focus:outline-hidden"
              value={values.description}
              onChange={handleChange}
              placeholder="Tell us a bit about your application."
              required
            />
            {touched.description && typeof errors.description === "string" && (
              <p className="text-xs text-content-errorSecondary" role="alert">
                {errors.description}
              </p>
            )}
          </label>

          <div className="rounded border bg-blue-50 p-3 dark:bg-blue-900/20">
            <p className="text-sm text-content-secondary">
              <strong>Note:</strong> The Convex team will review your request
              and respond via to{" "}
              <span className="font-mono text-content-primary">
                {userEmail || "your email"}
              </span>
              .
            </p>
          </div>

          <div className="mt-2 flex items-center justify-between">
            {error && (
              <span className="mr-2 text-xs text-content-errorSecondary">
                {error}
              </span>
            )}
            <Button
              type="submit"
              className="ml-auto"
              loading={isSubmitting || loading}
            >
              Send Request
            </Button>
          </div>
        </form>
      )}
    </Formik>
  );
}

export function OauthApps({ teamId }: { teamId: number }) {
  const { data: oauthApps, isLoading } = useTeamOauthApps(teamId);
  const isAdmin = useIsCurrentMemberTeamAdmin();
  const registerOauthApp = useRegisterOauthApp(teamId);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [registerError, setRegisterError] = useState("");

  return (
    <Sheet className="max-w-fit">
      {createModalOpen && (
        <Modal
          onClose={() => setCreateModalOpen(false)}
          title="Register a new OAuth App"
        >
          <OauthAppForm
            initialValues={{ appName: "", redirectUris: "" }}
            validationSchema={OAUTH_APP_SCHEMA}
            isVerified={false}
            onSubmit={async (
              values: { appName: string; redirectUris: string },
              {
                setSubmitting,
                resetForm,
              }: {
                setSubmitting: (isSubmitting: boolean) => void;
                resetForm?: () => void;
              },
            ) => {
              setRegisterError("");
              setSubmitting(true);
              try {
                const redirectUris = values.redirectUris
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean);
                await registerOauthApp({
                  appName: values.appName,
                  redirectUris,
                });
                if (resetForm) resetForm();
                setCreateModalOpen(false);
              } catch (err: any) {
                setRegisterError(err?.message || "Failed to register app");
              } finally {
                setSubmitting(false);
              }
            }}
            submitLabel="Save"
            loading={false}
            error={registerError}
          />
        </Modal>
      )}
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-14 w-full" }}
      >
        {isLoading ? null : oauthApps && oauthApps.length ? (
          <div className="flex flex-col gap-4">
            <div className="mb-4 flex flex-wrap items-center justify-between gap-4">
              <p className="text-sm">
                These are the OAuth applications registered by your team.
              </p>
              <Button
                size="xs"
                icon={<PlusIcon />}
                onClick={() => setCreateModalOpen(true)}
              >
                Create Application
              </Button>
            </div>
            <div className="flex w-full flex-col gap-4">
              {oauthApps.map((app: OauthAppResponse) => (
                <OauthAppListItem
                  key={app.clientId}
                  app={app}
                  isAdmin={isAdmin}
                  oauthAppSchema={OAUTH_APP_SCHEMA}
                  teamId={teamId}
                />
              ))}
            </div>
          </div>
        ) : (
          <div className="flex w-full flex-col items-center gap-4">
            <div className="text-center">
              <h3 className="mb-2 font-semibold text-content-primary">
                No OAuth Applications
              </h3>
              <p className="mb-4 max-w-md text-sm text-content-secondary">
                OAuth applications allow third-party developers to create and
                connect to Convex deployments owned by other teams.
              </p>
              <p className="mb-4 max-w-md text-sm text-content-secondary">
                This page is for developers who want to create Convex
                integrations.
              </p>
              <div className="mb-2 flex w-full flex-col items-center space-y-2 text-sm text-content-tertiary">
                <p>
                  <Link
                    href="https://docs.convex.dev/auth/oauth"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex w-fit items-center gap-1 text-content-link hover:underline"
                  >
                    <ExternalLinkIcon />
                    Learn more about OAuth applications
                  </Link>
                </p>
                <p>
                  <Link
                    href="https://docs.convex.dev/auth/oauth#verification"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex w-fit items-center gap-1 text-content-link hover:underline"
                  >
                    <ExternalLinkIcon />
                    OAuth app verification requirements
                  </Link>
                </p>
              </div>
            </div>
            <Button
              size="sm"
              icon={<PlusIcon />}
              onClick={() => setCreateModalOpen(true)}
            >
              Create Your First Application
            </Button>
          </div>
        )}
      </LoadingTransition>
    </Sheet>
  );
}

function OauthAppListItem({
  app,
  isAdmin,
  oauthAppSchema,
  teamId,
}: {
  app: OauthAppResponse;
  isAdmin: boolean;
  oauthAppSchema: any;
  teamId: number;
}) {
  // Local state for edit modal
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editName, setEditName] = useState(app.appName);
  const [editUris, setEditUris] = useState(app.redirectUris.join(", "));
  const [editError, setEditError] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const updateOauthApp = useUpdateOauthApp(teamId, app.clientId);
  const deleteOauthApp = useDeleteOauthApp(teamId, app.clientId);
  // Local state for delete confirmation
  const [showDelete, setShowDelete] = useState(false);
  const [deleteError, setDeleteError] = useState("");

  const [secretVisible, setSecretVisible] = useState(false);

  // Local state for verification request modal
  const [verificationModalOpen, setVerificationModalOpen] = useState(false);
  const [verificationError, setVerificationError] = useState("");
  const [verificationLoading, setVerificationLoading] = useState(false);

  return (
    <div className="scrollbar flex w-full flex-col gap-2 overflow-x-auto rounded border bg-background-secondary p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-3">
          <h4>{app.appName}</h4>
          <Tooltip
            tip={
              !app.verified
                ? "This app is not verified yet. You may test this app by authorizing it to the team that registered it. To allow this app to work for other teams, you must request verification."
                : undefined
            }
            side="right"
          >
            <div
              className={cn(
                "flex items-center gap-1 rounded-sm border p-1 text-xs",
                !app.verified &&
                  "border-yellow-700 bg-yellow-100/50 text-yellow-700 dark:border-yellow-200 dark:bg-yellow-900/50 dark:text-yellow-200",
              )}
            >
              {app.verified ? "Verified" : "Unverified"}
              {!app.verified && <QuestionMarkCircledIcon />}
            </div>
          </Tooltip>
        </div>
        <Menu
          placement="bottom-start"
          buttonProps={{
            variant: "neutral",
            icon: <DotsVerticalIcon />,
            "aria-label": "App options",
            size: "xs",
          }}
        >
          {!app.verified ? (
            <MenuItem action={() => setVerificationModalOpen(true)}>
              Request Verification
            </MenuItem>
          ) : null}
          <MenuItem
            action={() => setEditModalOpen(true)}
            disabled={!isAdmin}
            tip={!isAdmin ? "Only team admins can edit OAuth apps." : undefined}
            tipSide="right"
          >
            Edit Application
          </MenuItem>
          <MenuItem
            action={() => setShowDelete(true)}
            variant="danger"
            disabled={!isAdmin}
            tip={
              !isAdmin ? "Only team admins can delete OAuth apps." : undefined
            }
            tipSide="right"
          >
            Delete Application
          </MenuItem>
        </Menu>
      </div>
      <div className="flex flex-wrap gap-2">
        <div className="text-xs break-all">
          <div className="leading-6 font-semibold">Client ID</div>
          <CopyTextButton text={app.clientId} className="font-mono text-xs" />
        </div>
        {app.clientSecret ? (
          <div className="text-xs break-all">
            <div className="flex items-center gap-1 leading-6 font-semibold">
              Client Secret
              <Button
                type="button"
                aria-label={
                  secretVisible ? "Hide client secret" : "Show client secret"
                }
                inline
                size="xs"
                variant="neutral"
                onClick={() => setSecretVisible((v) => !v)}
              >
                {secretVisible ? <EyeNoneIcon /> : <EyeOpenIcon />}
              </Button>
            </div>
            <CopyTextButton
              text={app.clientSecret}
              className="font-mono text-xs"
              textHidden={!secretVisible}
            />
          </div>
        ) : (
          <div className="text-xs break-all">
            <div className="flex items-center gap-1 leading-6 font-semibold">
              Client Secret
              <Tooltip tip="Only team admins can see the client secret.">
                <InfoCircledIcon />
              </Tooltip>
            </div>
            <span className="text-xs leading-6.5">
              •••••••••••••••••••••••••••••••••
            </span>
          </div>
        )}
      </div>
      <div className="text-xs">
        <span className="leading-6 font-semibold">Redirect URIs</span>
        <ul className="list-inside list-disc">
          {app.redirectUris.map((uri: string, i: number) => (
            <li key={i} className="max-w-prose break-all">
              {uri}
            </li>
          ))}
        </ul>
      </div>
      {editModalOpen && (
        <Modal onClose={() => setEditModalOpen(false)} title="Edit OAuth App">
          <OauthAppForm
            initialValues={{ appName: editName, redirectUris: editUris }}
            isVerified={app.verified}
            validationSchema={oauthAppSchema}
            onSubmit={async (
              values: { appName: string; redirectUris: string },
              {
                setSubmitting,
              }: {
                setSubmitting: (isSubmitting: boolean) => void;
                resetForm?: () => void;
              },
            ) => {
              setEditError("");
              setEditLoading(true);
              setSubmitting(true);
              try {
                const redirectUris = values.redirectUris
                  .split(",")
                  .map((s: string) => s.trim())
                  .filter(Boolean);
                await updateOauthApp({
                  appName:
                    values.appName === app.appName ? undefined : values.appName,
                  redirectUris,
                });
                setEditModalOpen(false);
                setEditName(values.appName);
                setEditUris(values.redirectUris);
              } catch (err: any) {
                setEditError(err?.message || "Failed to update app");
              } finally {
                setEditLoading(false);
                setSubmitting(false);
              }
            }}
            submitLabel="Save"
            loading={editLoading}
            error={editError}
          />
        </Modal>
      )}
      {showDelete && (
        <ConfirmationDialog
          dialogTitle="Delete OAuth App"
          dialogBody="Are you sure you want to delete this OAuth app? This cannot be undone."
          validationText={app.appName}
          confirmText="Delete"
          error={deleteError}
          onClose={() => setShowDelete(false)}
          onConfirm={async () => {
            try {
              await deleteOauthApp();
              setShowDelete(false);
            } catch (err: any) {
              setDeleteError(err?.message || "Failed to delete app");
            }
          }}
        />
      )}
      {verificationModalOpen && (
        <Modal
          onClose={() => setVerificationModalOpen(false)}
          title="Request OAuth App Verification"
        >
          <VerificationRequestForm
            app={app}
            onSubmit={(description) =>
              handleVerificationRequest(
                description,
                app,
                teamId,
                setVerificationError,
                setVerificationLoading,
                setVerificationModalOpen,
              )
            }
            loading={verificationLoading}
            error={verificationError}
          />
        </Modal>
      )}
    </div>
  );
}
