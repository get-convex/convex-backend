import {
  Button,
  Tooltip,
  Combobox,
  toast,
  Sheet,
  TextInput,
} from "dashboard-common";
import { useFormik } from "formik";
import { useCreateInvite } from "api/invitations";
import { Team, CreateInvitationArgs, TeamMemberResponse } from "generatedApi";
import * as Yup from "yup";
import Link from "next/link";
import { roleOptions } from "./TeamMemberListItem";

export type InviteMemberFormProps = {
  team: Team;
  members: TeamMemberResponse[];
  hasAdminPermissions: boolean;
};

export function InviteMemberForm({
  team,
  members,
  hasAdminPermissions,
}: InviteMemberFormProps) {
  const InviteSchema = Yup.object().shape({
    inviteEmail: Yup.string()
      .email("Must be a valid email.")
      .notOneOf(
        members.map((member) => member.email) || [],
        "Email is already a member of this team.",
      )
      .max(254, "Email must be at most 254 characters long."),
    role: Yup.string().oneOf(["developer", "admin"]),
  });
  const createInvite = useCreateInvite(team.id);
  const formState = useFormik<{
    inviteEmail: string;
    role: CreateInvitationArgs["role"];
  }>({
    initialValues: {
      // Define the form value as "inviteEmail" to avoid browser extensions thinking this is a login form.
      inviteEmail: "",
      role: "developer",
    },
    validationSchema: InviteSchema,
    onSubmit: async (values) => {
      if (!hasAdminPermissions) {
        return;
      }
      const validation = await formState.validateForm();
      if (Object.keys(validation).length) {
        toast("success", "Invalid email", "email");
        return;
      }
      await createInvite({
        email: values.inviteEmail,
        role: values.role,
      });
      await formState.setFieldValue("inviteEmail", "");
    },
  });

  return (
    <Sheet className="min-w-fit text-sm">
      <h3 className="mb-4">Invite Member</h3>
      <form onSubmit={formState.handleSubmit} aria-label="Invite team member">
        <div className="mb-6 flex w-full grow flex-wrap gap-4 sm:flex-nowrap">
          <Tooltip
            tip={
              !hasAdminPermissions
                ? "You do not have permission to invite team members"
                : undefined
            }
            className="w-full"
          >
            <TextInput
              label="Email address"
              labelHidden
              placeholder="Email address"
              type="email"
              onChange={formState.handleChange}
              value={formState.values.inviteEmail}
              error={
                formState.touched ? formState.errors.inviteEmail : undefined
              }
              onBlur={formState.handleBlur}
              id="inviteEmail"
              aria-label="Email address"
              disabled={!hasAdminPermissions}
            />
          </Tooltip>
          {hasAdminPermissions && (
            <Combobox
              buttonClasses="w-fit"
              disableSearch
              label="Role"
              buttonProps={{
                tip: (
                  <span>
                    Select a{" "}
                    <Link
                      href="https://docs.convex.dev/dashboard/teams#roles-and-permissions"
                      className="underline"
                    >
                      team role
                    </Link>{" "}
                    for the new member.
                  </span>
                ),
                tipSide: "top",
              }}
              options={roleOptions}
              selectedOption={formState.values.role}
              setSelectedOption={async (role) => {
                if (!role) {
                  return;
                }
                await formState.setFieldValue("role", role);
              }}
            />
          )}
          <Button
            disabled={
              !formState.dirty || formState.isSubmitting || !formState.isValid
            }
            type="submit"
            aria-label="submit"
          >
            Send Invite
          </Button>
        </div>
      </form>
    </Sheet>
  );
}
