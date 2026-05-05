import { TeamResponse } from "generatedApi";
import {
  CustomRoleResponse,
  RoleStatement,
  RoleStatementAction,
} from "@convex-dev/platform/managementApi";
import { Sheet } from "@ui/Sheet";
import { Callout } from "@ui/Callout";
import { Loading } from "@ui/Loading";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TextInput } from "@ui/TextInput";
import { Menu, MenuItem } from "@ui/Menu";
import { PlusIcon, DotsVerticalIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import { useRouter } from "next/router";
import { Link } from "@ui/Link";
import { cn } from "@ui/cn";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { useTeamEntitlements } from "api/teams";
import {
  useListCustomRoles,
  useCreateCustomRole,
  useUpdateCustomRole,
  useDeleteCustomRole,
  useIsCurrentMemberTeamAdmin,
} from "api/roles";
import Editor, { BeforeMount, OnMount } from "@monaco-editor/react";
import { editorOptions } from "@common/elements/ObjectEditor/ObjectEditor";
import { useCurrentTheme } from "@common/lib/useCurrentTheme";

const EMPTY_STATEMENTS = "[\n  \n]";

const STATEMENTS_EDITOR_PATH = "custom-role-statements.json";

function errorMessage(e: unknown, fallback: string): string {
  if (
    typeof e === "object" &&
    e !== null &&
    "message" in e &&
    typeof (e as { message: unknown }).message === "string"
  ) {
    return (e as { message: string }).message;
  }
  return fallback;
}

// Categories below match the (leaf, parent) pairs the server's
// `RoleStatement::validate` enforces in
// crates_private/big_brain_lib/src/roles/types.rs. Token actions split by
// owner so e.g. `team:*:token:*` only allows team-token actions. The server
// enforces this pairing too — this duplication just gives inline editor
// feedback.
type ActionCategory =
  | "team"
  | "project"
  | "deployment"
  | "member"
  | "teamToken"
  | "projectToken"
  | "deploymentToken"
  | "customRole";

const ACTIONS_BY_CATEGORY: Record<ActionCategory, RoleStatementAction[]> = {
  team: [
    "updateTeam",
    "deleteTeam",
    "updatePaymentMethod",
    "updateBillingContact",
    "updateBillingAddress",
    "createSubscription",
    "resumeSubscription",
    "cancelSubscription",
    "changeSubscriptionPlan",
    "setSpendingLimit",
    "viewBillingDetails",
    "viewInvoices",
    "createOAuthApplication",
    "updateOAuthApplication",
    "deleteOAuthApplication",
    "viewOAuthApplication",
    "generateOAuthClientSecret",
    "viewUsage",
    "applyReferralCode",
    "enableSSO",
    "disableSSO",
    "updateSSO",
    "viewSSO",
    "viewTeamIntegrations",
    "createTeamIntegrations",
    "updateTeamIntegrations",
    "deleteTeamIntegrations",
  ],
  project: [
    "createProject",
    "transferProject",
    "receiveProject",
    "updateProject",
    "deleteProject",
    "viewProject",
    "updateMemberProjectRole",
    "createProjectEnvironmentVariable",
    "updateProjectEnvironmentVariable",
    "deleteProjectEnvironmentVariable",
  ],
  deployment: [
    "createDeployment",
    "receiveDeployment",
    "transferDeployment",
    "deleteDeployment",
    "updateDeploymentReference",
    "updateDeploymentDashboardEditConfirmation",
    "updateDeploymentExpiresAt",
    "updateDeploymentSendLogsToClient",
    "updateDeploymentClass",
    "updateDeploymentIsDefault",
    "updateDeploymentType",
    "viewDeploymentIntegrations",
    "writeDeploymentIntegrations",
    "createCustomDomain",
    "deleteCustomDomain",
    "viewInsights",
    "startManualCloudBackup",
    "restoreFromCloudBackup",
    "configurePeriodicBackup",
    "disablePeriodicBackup",
    "deleteCloudBackup",
  ],
  member: [
    "inviteMember",
    "cancelMemberInvitation",
    "removeMember",
    "updateMemberRole",
  ],
  teamToken: [
    "createTeamAccessToken",
    "updateTeamAccessToken",
    "deleteTeamAccessToken",
    "viewTeamAccessToken",
  ],
  projectToken: [
    "createProjectAccessToken",
    "updateProjectAccessToken",
    "deleteProjectAccessToken",
    "viewProjectAccessToken",
  ],
  deploymentToken: [
    "createDeploymentAccessToken",
    "updateDeploymentAccessToken",
    "deleteDeploymentAccessToken",
    "viewDeploymentAccessToken",
  ],
  customRole: ["viewCustomRoles"],
};

const actionsForCategory = (category: ActionCategory) => ({
  if: { type: "array" },
  then: {
    minItems: 1,
    items: { type: "string", enum: ACTIONS_BY_CATEGORY[category] },
  },
  else: { const: "*" },
});

// Mirrors ResourceSpecifier::FromStr in
// crates_private/big_brain_lib/src/roles/parse.rs. Tokens nest directly under
// their owning resource: `team:*:token:*`, `project:*:token:*`, or
// `project:*:deployment:*:token:*`.
const SELECTOR_VAL = "[^,:]+";
const projectSel = `(\\*|id=${SELECTOR_VAL}|slug=${SELECTOR_VAL})`;
const deploymentSel = `(\\*|id=${SELECTOR_VAL}|type=${SELECTOR_VAL}|creator=${SELECTOR_VAL})`;
const memberSel = `(\\*|id=${SELECTOR_VAL})`;
const tokenSel = `(\\*|creator=${SELECTOR_VAL})`;
const csv = (sel: string) => `${sel}(,${sel})*`;
const tokenTail = `:token:${csv(tokenSel)}`;
const RESOURCE_PATTERN =
  `^(team:\\*(${tokenTail})?` +
  `|project:${csv(projectSel)}(${tokenTail}|:deployment:${csv(deploymentSel)}(${tokenTail})?)?` +
  `|member:${csv(memberSel)}` +
  `|customRole:\\*)$`;

const statementsSchema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  title: "Custom Role Statements",
  type: "array",
  items: { $ref: "#/definitions/statement" },
  definitions: {
    statement: {
      type: "object",
      required: ["effect", "actions", "resource"],
      additionalProperties: false,
      properties: {
        effect: { enum: ["allow", "deny"] },
        actions: { $ref: "#/definitions/actionPattern" },
        resource: { $ref: "#/definitions/resourceSpecifier" },
      },
      allOf: [
        {
          if: {
            required: ["resource"],
            properties: { resource: { pattern: "^team:[^:]+$" } },
          },
          then: { properties: { actions: actionsForCategory("team") } },
        },
        {
          if: {
            required: ["resource"],
            properties: { resource: { pattern: "^project:[^:]+$" } },
          },
          then: { properties: { actions: actionsForCategory("project") } },
        },
        {
          if: {
            required: ["resource"],
            properties: {
              resource: { pattern: "^project:[^:]+:deployment:[^:]+$" },
            },
          },
          then: {
            properties: { actions: actionsForCategory("deployment") },
          },
        },
        {
          if: {
            required: ["resource"],
            properties: { resource: { pattern: "^member:[^:]+$" } },
          },
          then: { properties: { actions: actionsForCategory("member") } },
        },
        {
          if: {
            required: ["resource"],
            properties: { resource: { pattern: "^team:[^:]+:token:[^:]+$" } },
          },
          then: { properties: { actions: actionsForCategory("teamToken") } },
        },
        {
          if: {
            required: ["resource"],
            properties: {
              resource: { pattern: "^project:[^:]+:token:[^:]+$" },
            },
          },
          then: {
            properties: { actions: actionsForCategory("projectToken") },
          },
        },
        {
          if: {
            required: ["resource"],
            properties: {
              resource: {
                pattern: "^project:[^:]+:deployment:[^:]+:token:[^:]+$",
              },
            },
          },
          then: {
            properties: { actions: actionsForCategory("deploymentToken") },
          },
        },
        {
          if: {
            required: ["resource"],
            properties: { resource: { pattern: "^customRole:[^:]+$" } },
          },
          then: { properties: { actions: actionsForCategory("customRole") } },
        },
      ],
    },
    // Structural shape only — the per-resource-kind narrowing in `statement`
    // owns the action enum check, so listing the enum here too would surface a
    // duplicate "value not allowed" tooltip for unknown actions.
    actionPattern: {
      if: { type: "array" },
      then: {
        minItems: 1,
        items: { type: "string" },
      },
      else: { const: "*" },
    },
    resourceSpecifier: {
      type: "string",
      minLength: 1,
      pattern: RESOURCE_PATTERN,
      patternErrorMessage: "Invalid resource specifier.",
      description:
        "Resource path like `team:*`, `project:*`, `project:slug=my-app`, `project:*:deployment:type=prod`, `customRole:*`, or with a `:token:*` suffix to scope token actions (e.g. `team:*:token:*`).",
    },
  },
};

function CustomRoleForm({
  teamId,
  existingRole,
  onClose,
  onSaved,
}: {
  teamId: number;
  existingRole?: CustomRoleResponse;
  onClose: () => void;
  onSaved: (roleId: number) => void;
}) {
  const [name, setName] = useState(existingRole?.name ?? "");
  const [description, setDescription] = useState(
    existingRole?.description ?? "",
  );
  const [statementsText, setStatementsText] = useState(
    existingRole
      ? JSON.stringify(existingRole.statements, null, 2)
      : EMPTY_STATEMENTS,
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string>();
  const [nameError, setNameError] = useState<string>();
  const [hasSchemaError, setHasSchemaError] = useState(false);
  // Tracks the id of a role created in this form instance so that a fast
  // second save after Create still routes through update, even if the SWR
  // cache for `list_custom_roles` hasn't yet repopulated `existingRole`.
  const [createdRoleId, setCreatedRoleId] = useState<number | undefined>();
  const savedRoleId = existingRole?.id ?? createdRoleId;
  const currentTheme = useCurrentTheme();
  const prefersDark = currentTheme === "dark";

  const createCustomRole = useCreateCustomRole(teamId);
  const updateCustomRole = useUpdateCustomRole(teamId);

  const handleEditorBeforeMount: BeforeMount = (monaco) => {
    monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
      validate: true,
      allowComments: false,
      schemas: [
        {
          uri: STATEMENTS_EDITOR_PATH,
          fileMatch: [STATEMENTS_EDITOR_PATH],
          schema: statementsSchema,
        },
      ],
    });
  };

  const handleEditorMount: OnMount = (editorInstance, monaco) => {
    const model = editorInstance.getModel();
    if (!model) return;
    const updateMarkers = () => {
      const markers = monaco.editor.getModelMarkers({ resource: model.uri });
      setHasSchemaError(
        markers.some((m) => m.severity === monaco.MarkerSeverity.Error),
      );
    };
    updateMarkers();
    const disposable = monaco.editor.onDidChangeMarkers((uris) => {
      if (uris.some((u) => u.toString() === model.uri.toString())) {
        updateMarkers();
      }
    });
    editorInstance.onDidDispose(() => disposable.dispose());
  };

  const handleSubmit = async () => {
    setError(undefined);
    setNameError(undefined);

    if (!name.trim()) {
      setNameError("Name is required.");
      return;
    }

    if (hasSchemaError) {
      setError(
        "Fix the highlighted schema errors in statements before saving.",
      );
      return;
    }

    let parsedStatements: RoleStatement[];
    try {
      const raw = JSON.parse(statementsText);
      if (!Array.isArray(raw)) {
        setError("Statements must be a JSON array.");
        return;
      }
      parsedStatements = raw as RoleStatement[];
    } catch {
      setError("Statements must be valid JSON.");
      return;
    }

    if (parsedStatements.length === 0) {
      setError("A custom role must have at least one statement.");
      return;
    }

    setIsSubmitting(true);
    try {
      if (savedRoleId !== undefined) {
        await updateCustomRole({
          id: savedRoleId,
          name: name.trim(),
          description: description.trim() || null,
          statements: parsedStatements,
        });
        onSaved(savedRoleId);
      } else {
        const created = await createCustomRole({
          name: name.trim(),
          description: description.trim() || null,
          statements: parsedStatements,
        });
        if (created) {
          setCreatedRoleId(created.id);
          onSaved(created.id);
        }
      }
    } catch (e: unknown) {
      setError(errorMessage(e, "An error occurred."));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Sheet>
      <div className="flex flex-col gap-4">
        <TextInput
          id="role-name"
          label="Name"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setNameError(undefined);
          }}
          placeholder="e.g. Viewer"
          error={nameError}
        />
        <div className="flex flex-col gap-1">
          <label
            className="text-left text-sm text-content-primary"
            htmlFor="role-description"
          >
            Description
          </label>
          <textarea
            id="role-description"
            className="min-h-[60px] w-full rounded border bg-background-secondary px-3 py-2 text-sm text-content-primary"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional description for this role"
          />
        </div>
        <div className="flex flex-col gap-1">
          <div className="text-left text-sm text-content-primary">
            Statements
          </div>
          <div className="h-[60vh] min-h-96 overflow-hidden rounded border">
            <Editor
              path={STATEMENTS_EDITOR_PATH}
              value={statementsText}
              language="json"
              theme={prefersDark ? "vs-dark" : "light"}
              onChange={(v) => setStatementsText(v ?? "")}
              beforeMount={handleEditorBeforeMount}
              onMount={handleEditorMount}
              options={{
                ...editorOptions,
                scrollBeyondLastLine: false,
                padding: { top: 8, bottom: 8 },
              }}
            />
          </div>
        </div>
        {error && (
          <p className="text-sm text-content-errorSecondary">{error}</p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="neutral" onClick={onClose} disabled={isSubmitting}>
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={isSubmitting || hasSchemaError}
            tip={
              hasSchemaError
                ? "Resolve the highlighted schema errors first."
                : undefined
            }
          >
            {savedRoleId !== undefined ? "Save" : "Create"}
          </Button>
        </div>
      </div>
    </Sheet>
  );
}

function CustomRoleListItem({
  role,
  teamId,
  onEdit,
  disabled,
  disabledReason,
}: {
  role: CustomRoleResponse;
  teamId: number;
  onEdit: () => void;
  disabled?: boolean;
  disabledReason?: string;
}) {
  const [showDelete, setShowDelete] = useState(false);
  const [deleteError, setDeleteError] = useState<string>();
  const deleteCustomRole = useDeleteCustomRole(teamId);

  return (
    <div className="flex w-full flex-col py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-col gap-0.5">
          <div>{role.name}</div>
          {role.description && (
            <div className="text-xs text-content-secondary">
              {role.description}
            </div>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex flex-col items-end">
            <div className="text-xs text-content-secondary">
              {role.statements.length} statement
              {role.statements.length !== 1 ? "s" : ""}
            </div>
            <TimestampDistance
              prefix="Created "
              date={new Date(role.createTime)}
            />
          </div>
          <Menu
            placement="bottom-end"
            buttonProps={{
              variant: "neutral",
              size: "xs",
              icon: <DotsVerticalIcon />,
              "aria-label": "Custom role options",
              disabled,
              tip: disabled ? disabledReason : undefined,
            }}
          >
            <MenuItem action={onEdit}>Edit</MenuItem>
            <MenuItem variant="danger" action={() => setShowDelete(true)}>
              Delete
            </MenuItem>
          </Menu>
        </div>
      </div>
      {showDelete && (
        <ConfirmationDialog
          dialogTitle="Delete Custom Role"
          dialogBody={`Are you sure you want to delete the custom role "${role.name}"? This cannot be undone.`}
          validationText={role.name}
          confirmText="Delete"
          error={deleteError}
          onClose={() => {
            setShowDelete(false);
            setDeleteError(undefined);
          }}
          onConfirm={async () => {
            try {
              await deleteCustomRole({ id: role.id });
              setShowDelete(false);
            } catch (e: unknown) {
              setDeleteError(errorMessage(e, "Failed to delete."));
            }
          }}
        />
      )}
    </div>
  );
}

export function CustomRoles({ team }: { team: TeamResponse }) {
  const router = useRouter();
  const entitlements = useTeamEntitlements(team.id);
  const entitlementsLoaded = entitlements !== undefined;
  const customRolesEnabled = entitlements?.customRolesEnabled ?? false;
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();
  const { data: customRolesData } = useListCustomRoles(
    customRolesEnabled ? team.id : undefined,
  );
  const roles = customRolesEnabled ? customRolesData?.items : [];

  const canManage = customRolesEnabled && hasAdminPermissions;
  const disabledReason = !entitlementsLoaded
    ? undefined
    : !customRolesEnabled
      ? "Custom roles are not enabled for this team."
      : !hasAdminPermissions
        ? "You do not have permission to manage custom roles."
        : undefined;

  const editingRoleId =
    typeof router.query.role === "string" ? router.query.role : undefined;
  const editingRole = editingRoleId
    ? roles?.find((r) => r.id.toString() === editingRoleId)
    : undefined;
  const isNewRole = router.query.new === "1";
  const showForm = canManage && (isNewRole || editingRoleId !== undefined);

  const listHref = {
    pathname: "/t/[team]/settings/custom-roles",
    query: { team: team.slug },
  } as const;

  const goToList = () => {
    void router.push(listHref, undefined, { shallow: true });
  };

  const goToNew = () => {
    void router.push(
      {
        pathname: "/t/[team]/settings/custom-roles",
        query: { team: team.slug, new: "1" },
      },
      undefined,
      { shallow: true },
    );
  };

  const goToEdit = (roleId: number) => {
    void router.push(
      {
        pathname: "/t/[team]/settings/custom-roles",
        query: { team: team.slug, role: roleId.toString() },
      },
      undefined,
      { shallow: true },
    );
  };

  const currentCrumb = isNewRole
    ? "New"
    : (editingRole?.name ?? (editingRoleId ? "Edit" : undefined));

  return (
    <div className="-mx-6 flex flex-col">
      <div className="sticky top-0 z-10 -mt-6 flex items-center gap-2 bg-background-primary p-6">
        {showForm ? (
          <Link href={listHref}>
            <h2>Custom Roles</h2>
          </Link>
        ) : (
          <h2>Custom Roles</h2>
        )}
        {showForm && currentCrumb && (
          <>
            <span className="text-content-secondary" role="separator">
              /
            </span>
            <h2 className="text-content-secondary">{currentCrumb}</h2>
          </>
        )}
      </div>
      <div className="relative flex overflow-x-hidden">
        <div
          className={cn(
            "flex w-full gap-6 transition-transform duration-500 motion-reduce:transition-none",
            showForm ? "-translate-x-[calc(100%+1.5rem)]" : "translate-x-0",
          )}
        >
          <div
            className={cn(
              "flex w-full shrink-0 flex-col gap-4 px-6",
              showForm ? "pointer-events-none select-none" : "",
            )}
            // @ts-expect-error https://github.com/facebook/react/issues/17157
            inert={showForm ? "inert" : undefined}
          >
            {entitlementsLoaded && !customRolesEnabled && (
              <Callout variant="upsell">
                Custom roles are available on the Business plan. Upgrade your
                team to create and manage custom roles.
              </Callout>
            )}
            <Sheet>
              <div className="mb-4 flex items-center justify-between">
                <p className="text-sm text-content-primary">
                  Custom roles let you define fine-grained permissions for your
                  team members.
                </p>
                <Button
                  onClick={goToNew}
                  icon={<PlusIcon />}
                  disabled={!canManage}
                  tip={disabledReason}
                >
                  Create Role
                </Button>
              </div>
              {roles === undefined ? (
                <Loading fullHeight={false} className="h-14 w-full" />
              ) : (
                <div className="flex w-full flex-col divide-y">
                  {roles.length > 0 ? (
                    roles.map((role) => (
                      <CustomRoleListItem
                        key={role.id}
                        role={role}
                        teamId={team.id}
                        onEdit={() => goToEdit(role.id)}
                        disabled={!canManage}
                        disabledReason={disabledReason}
                      />
                    ))
                  ) : (
                    <div className="my-6 flex w-full justify-center text-content-secondary">
                      No custom roles have been created yet.
                    </div>
                  )}
                </div>
              )}
            </Sheet>
          </div>
          <div
            className={cn(
              "flex w-full shrink-0 flex-col gap-4 px-6",
              !showForm ? "pointer-events-none select-none" : "",
            )}
            // @ts-expect-error https://github.com/facebook/react/issues/17157
            inert={!showForm ? "inert" : undefined}
          >
            {showForm &&
              (editingRoleId !== undefined && customRolesData === undefined ? (
                <Loading fullHeight={false} className="h-48 w-full" />
              ) : (
                // Stable key: the form is unmounted whenever `showForm` flips
                // to false (Cancel / breadcrumb), so a fresh mount picks up
                // the right `existingRole`. Keying by `editingRoleId` would
                // remount on the create→edit URL flip and wipe the editor.
                <CustomRoleForm
                  key="custom-role-form"
                  teamId={team.id}
                  existingRole={editingRole}
                  onClose={goToList}
                  onSaved={goToEdit}
                />
              ))}
          </div>
        </div>
      </div>
    </div>
  );
}
