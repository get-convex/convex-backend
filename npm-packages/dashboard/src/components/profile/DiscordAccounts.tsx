import Link from "next/link";

import { useDiscordAccounts, useUnlinkDiscordAccount } from "api/discord";
import { Button } from "dashboard-common/elements/Button";
import { Sheet } from "dashboard-common/elements/Sheet";
import { TrashIcon } from "@radix-ui/react-icons";
import { DiscordAccount, DiscordAccountDetails } from "generatedApi";

function DiscordAccountDetail({
  id,
  details,
}: {
  id: string;
  details: DiscordAccountDetails | null | undefined;
}) {
  if (details === null || details === undefined) {
    return <span className="italic">Account {id}</span>;
  }

  // https://discord.com/developers/docs/reference#image-formatting-cdn-endpoints
  const withoutDiscriminator = details.discriminator === "0";
  const avatarUrl = details.avatar
    ? `https://cdn.discordapp.com/avatars/${id}/${details.avatar}.png`
    : `https://cdn.discordapp.com/embed/avatars/${
        withoutDiscriminator
          ? (BigInt(id) >> BigInt(22)) % BigInt(6)
          : +details.discriminator % 5
      }.png`;

  const primaryText = withoutDiscriminator
    ? details.global_name
      ? details.global_name
      : details.username
    : details.username;
  const secondaryText = withoutDiscriminator
    ? details.global_name
      ? ` ${details.username}`
      : ""
    : `#${details.discriminator}`;

  return (
    <>
      <span className="inline-flex h-9 w-9 select-none items-center justify-center overflow-hidden rounded-full bg-background-tertiary">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img src={avatarUrl} alt="" />
      </span>

      <div>
        <span className="font-medium text-content-primary">
          <span>{primaryText}</span>
        </span>
        <span className="text-content-secondary">
          <span>{secondaryText}</span>
        </span>
      </div>
    </>
  );
}

function DiscordAccountListRow({ account }: { account: DiscordAccount }) {
  const unlinkAccount = useUnlinkDiscordAccount();

  return (
    <div key={account.id} className="flex items-center gap-4 py-4">
      <DiscordAccountDetail id={account.id} details={account.details} />

      <div className="ml-auto">
        <Button
          tip="Unlink"
          type="button"
          onClick={() => {
            void unlinkAccount({ discordId: account.id });
          }}
          variant="danger"
          inline
          icon={<TrashIcon />}
        />
      </div>
    </div>
  );
}

export function DiscordAccountsList({
  accounts,
}: {
  accounts: DiscordAccount[];
}) {
  return (
    <div className="flex flex-col divide-y divide-border-transparent">
      {accounts.map((account) => (
        <DiscordAccountListRow account={account} key={account.id} />
      ))}
    </div>
  );
}

export function DiscordAccounts() {
  const accounts = useDiscordAccounts();

  return (
    <Sheet className="flex flex-col gap-4">
      <h3>Discord</h3>

      <p className="max-w-lg text-sm">
        Join the{" "}
        <Link
          href="https://convex.dev/community"
          className="text-content-link"
          target="_blank"
        >
          Convex Discord server
        </Link>{" "}
        to get support, share ideas, and chat with other Convex users and team
        members.
      </p>

      {accounts && <DiscordAccountsList accounts={accounts} />}
    </Sheet>
  );
}
