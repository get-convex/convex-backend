import { useAuth0 } from "hooks/useAuth0";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";
import { TextInput } from "@ui/TextInput";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";

import { RadioGroup } from "@headlessui/react";
import classNames from "classnames";
import { CheckCircledIcon } from "@radix-ui/react-icons";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import Head from "next/head";
import {
  useDeleteAccount,
  useProfile,
  useProfileEmails,
  useUpdateProfileName,
} from "api/profile";
import { useState } from "react";
import { UserProfile } from "@auth0/nextjs-auth0/client";
import { Emails } from "components/profile/Emails";
import { DiscordAccounts } from "components/profile/DiscordAccounts";
import { MemberResponse } from "generatedApi";
import { LoadingTransition } from "@ui/Loading";
import { useTheme } from "next-themes";

export { getServerSideProps } from "lib/ssr";

function Profile() {
  const { user } = useAuth0();
  const profile = useProfile();
  const emails = useProfileEmails();

  const [showConfirmation, setShowConfirmation] = useState(false);
  const deleteAccount = useDeleteAccount();
  const [deleteAccountError, setDeleteAccountError] = useState<
    string | undefined
  >();
  const deleteAccountBody = (
    <p className="max-w-prose text-pretty text-sm">
      To delete your account, your account must match the following criteria:
      <ul className="mt-2 list-inside list-disc">
        <li>You must not be the only admin in teams with other members.</li>
        <li>
          You must delete all projects in teams where you are the only member.
        </li>
      </ul>
    </p>
  );

  return (
    <LoadingTransition
      loadingProps={{
        fullHeight: false,
        className: "h-full",
      }}
    >
      <Head>
        <title>Profile | Convex Dashboard</title>
      </Head>
      {emails && profile && user && (
        <div className="w-full overflow-auto scrollbar">
          <div className="mx-auto flex min-w-[22rem] max-w-prose flex-col justify-center gap-4 p-4">
            <Sheet className="flex w-full flex-col gap-4">
              <h3>Profile information</h3>
              <ProfileForm profile={profile} user={user} />
            </Sheet>

            <Emails emails={emails} />

            <ToggleDarkMode />
            <DiscordAccounts />

            <Sheet className="flex flex-col gap-4">
              <h3>Delete Account</h3>
              {deleteAccountBody}
              <Button
                variant="danger"
                className="w-fit"
                onClick={() => setShowConfirmation(true)}
              >
                Delete account
              </Button>
              {showConfirmation && (
                <ConfirmationDialog
                  onClose={() => setShowConfirmation(false)}
                  onConfirm={async () => {
                    try {
                      document.cookie = "";
                      window.localStorage.clear();
                      await deleteAccount();
                    } catch (e: any) {
                      setDeleteAccountError(e.message);
                      throw e;
                    }
                  }}
                  confirmText="Delete account"
                  dialogTitle="Delete Account"
                  error={deleteAccountError}
                  dialogBody={deleteAccountBody}
                  validationText="Delete my account"
                />
              )}
            </Sheet>
          </div>
        </div>
      )}
    </LoadingTransition>
  );
}

function ProfileForm({
  profile,
  user,
}: {
  profile: MemberResponse;
  user: UserProfile;
}) {
  const [name, setName] = useState(profile.name);
  const [isLoading, setIsLoading] = useState(false);
  const updateProfileName = useUpdateProfileName();

  return (
    <div className="flex flex-col gap-4">
      <form
        className="flex flex-col gap-1"
        onSubmit={async (e) => {
          e.preventDefault();
          if (!name) return;
          setIsLoading(true);
          try {
            await updateProfileName({ name });
          } finally {
            setIsLoading(false);
          }
        }}
      >
        <div className="flex items-end gap-2">
          <TextInput
            id="name"
            label="Name"
            value={name || ""}
            onChange={(e) => setName(e.target.value)}
            error={
              name
                ? name.length > 128
                  ? "Name must be at most 128 characters long."
                  : undefined
                : undefined
            }
          />
          <Button
            type="submit"
            disabled={
              name === profile.name || (name ? name.length > 128 : false)
            }
            icon={isLoading ? <Spinner /> : undefined}
          >
            Save
          </Button>
        </div>
      </form>

      <Tooltip tip="Changing your connected GitHub account is not currently supported. Contact support@convex.dev for help.">
        <TextInput
          id="github"
          label="GitHub Account"
          onChange={() => {}}
          value={user?.nickname || ""}
          disabled
        />
      </Tooltip>
    </div>
  );
}

export default withAuthenticatedPage(Profile);

const themes = [
  {
    title: "Light",
    value: "light",
  },
  { title: "Dark", value: "dark" },
  { title: "System", value: "system" },
];

function ToggleDarkMode() {
  const { theme: currentTheme, setTheme } = useTheme();

  return (
    <Sheet className="flex flex-col gap-4">
      <RadioGroup value={currentTheme} onChange={setTheme}>
        <RadioGroup.Label>
          <h3>Dashboard theme</h3>
        </RadioGroup.Label>

        <div className="mt-4 grid grid-cols-1 gap-y-6 sm:grid-cols-3 sm:gap-x-4">
          {themes.map((theme) => (
            <RadioGroup.Option
              key={theme.title}
              value={theme.value}
              className={({ checked, active }) =>
                classNames(
                  checked ? "border-transparent" : "",
                  active ? "" : "",
                  "relative block cursor-pointer rounded-lg border px-6 py-4 focus:outline-none sm:flex sm:justify-between",
                  "bg-background-tertiary border",
                )
              }
            >
              {({ checked, active }) => (
                <>
                  <span className="flex flex-1">
                    <span className="flex flex-col">
                      <RadioGroup.Label
                        as="span"
                        className="block text-sm font-medium text-content-primary"
                      >
                        {theme.title}
                      </RadioGroup.Label>
                    </span>
                  </span>
                  <CheckCircledIcon
                    className={classNames(!checked ? "invisible" : "", "mt-1")}
                    aria-hidden="true"
                  />
                  <span
                    className={classNames(
                      active ? "ring-2" : "border",
                      checked ? "border-border-selected" : "border-transparent",
                      "pointer-events-none absolute -inset-px rounded-lg",
                    )}
                    aria-hidden="true"
                  />
                </>
              )}
            </RadioGroup.Option>
          ))}
        </div>
      </RadioGroup>
    </Sheet>
  );
}
