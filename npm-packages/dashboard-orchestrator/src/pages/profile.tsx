// Personal profile page — BetterAuth-backed name/email view + sign-out.
// Cloud has an equivalent at dashboard/src/pages/profile.tsx with Discord,
// connected identities, and PATs; dashboard-orchestrator exposes only the
// fields BetterAuth's user table actually carries.

import Head from "next/head";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useTheme } from "next-themes";
import { RadioGroup } from "@headlessui/react";
import { CheckCircledIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { TextInput } from "@ui/TextInput";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { cn } from "@ui/cn";
import { authClient, signOut, useSession } from "../lib/auth-client";

export default function ProfilePage() {
  const router = useRouter();
  const session = useSession();
  const user = session?.data?.user;

  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedAt, setSavedAt] = useState<number | null>(null);

  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteError, setDeleteError] = useState<string | undefined>();

  useEffect(() => {
    if (user?.name && !name) setName(user.name);
    // intentionally only seed name from initial user — typing shouldn't get
    // clobbered by background session refreshes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.id]);

  if (!user) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background-primary">
        <p className="text-sm text-content-secondary">Loading…</p>
      </main>
    );
  }

  const onSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name || saving) return;
    setSaving(true);
    setError(null);
    try {
      const res = await authClient.updateUser({ name });
      if (res.error) throw new Error(res.error.message ?? "update failed");
      setSavedAt(Date.now());
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const onConfirmDelete = async () => {
    setDeleteError(undefined);
    try {
      const res = await authClient.deleteUser({});
      if (res.error) throw new Error(res.error.message ?? "delete failed");
      await signOut();
      void router.replace("/login");
    } catch (err) {
      setDeleteError((err as Error).message);
      throw err;
    }
  };

  return (
    <>
      <Head>
        <title>Profile | Convex Orchestrator</title>
      </Head>
      <main className="min-h-screen w-full overflow-auto bg-background-primary">
        <div className="mx-auto flex max-w-prose min-w-88 flex-col gap-4 p-6">
          <Sheet className="flex flex-col gap-4">
            <h3>Profile information</h3>
            <form className="flex flex-col gap-1" onSubmit={onSave}>
              <div className="flex items-end gap-2">
                <TextInput
                  id="name"
                  label="Name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  error={
                    name && name.length > 128
                      ? "Name must be at most 128 characters long."
                      : undefined
                  }
                />
                <Button
                  type="submit"
                  disabled={
                    !name || name === (user.name ?? "") || name.length > 128
                  }
                  loading={saving}
                >
                  Save
                </Button>
              </div>
            </form>
            {error && (
              <div className="text-xs text-content-error" role="alert">
                {error}
              </div>
            )}
            {savedAt && !error && (
              <div className="text-xs text-content-secondary">Saved.</div>
            )}
          </Sheet>

          <Sheet className="flex flex-col gap-2">
            <h3>Email</h3>
            <p className="text-sm text-content-secondary">
              Your email is managed by your authentication provider and cannot
              be changed from this dashboard.
            </p>
            <div className="font-mono text-sm text-content-primary">
              {user.email}
            </div>
          </Sheet>

          <ToggleDarkMode />

          <Sheet className="flex flex-col gap-3">
            <h3>Sign out</h3>
            <p className="text-sm text-content-secondary">
              End your session on this browser.
            </p>
            <Button
              variant="neutral"
              className="w-fit"
              onClick={async () => {
                await signOut();
                void router.replace("/login");
              }}
            >
              Sign out
            </Button>
          </Sheet>

          <Sheet className="flex flex-col gap-3">
            <h3>Delete Account</h3>
            <p className="max-w-prose text-sm text-content-secondary">
              Permanently delete your account from this orchestrator. Any teams
              you administer will need to transfer ownership before deletion can
              complete.
            </p>
            <Button
              variant="danger"
              className="w-fit"
              onClick={() => setDeleteOpen(true)}
            >
              Delete account
            </Button>
            {deleteOpen && (
              <ConfirmationDialog
                onClose={() => setDeleteOpen(false)}
                onConfirm={onConfirmDelete}
                confirmText="Delete account"
                dialogTitle="Delete Account"
                error={deleteError}
                validationText="Delete my account"
                dialogBody={
                  <p className="text-sm">
                    This permanently removes your user account and all
                    associated sessions. You will be signed out immediately.
                  </p>
                }
              />
            )}
          </Sheet>
        </div>
      </main>
    </>
  );
}

const themes = [
  { title: "Light", value: "light" },
  { title: "Dark", value: "dark" },
  { title: "System", value: "system" },
];

// Direct port of cloud's ToggleDarkMode (dashboard/src/pages/profile.tsx).
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
              className={({ checked }) =>
                cn(
                  "relative block cursor-pointer rounded-2xl border px-6 py-4 focus:outline-none sm:flex sm:justify-between",
                  checked
                    ? "[--theme-selector-border:var(--border-transparent)]"
                    : "[--theme-selector-border:transparent]",
                  "focus-visible:[--theme-selector-border:var(--border-selected)]",
                  "bg-background-primary/30 shadow-sm transition-colors hover:bg-background-primary/70",
                  checked
                    ? "bg-background-tertiary"
                    : "bg-background-secondary",
                )
              }
            >
              {({ checked }) => (
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
                    className={cn(!checked ? "invisible" : "", "mt-1")}
                    aria-hidden="true"
                  />
                  <span
                    className={cn(
                      "border border-(--theme-selector-border)",
                      "pointer-events-none absolute -inset-px rounded-2xl",
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
