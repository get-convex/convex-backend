// DNS provider credentials for the ACME dns-01 challenge.
//
// Tokens are sealed by the orchestrator before they hit Postgres and are
// never sent back, so this card can list and replace credentials but can
// never show one. Saving under an existing name rotates the token.

import { useState } from "react";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { TrashIcon } from "@radix-ui/react-icons";
import { useDnsCredentials } from "../hooks/useCustomDomains";

export function DnsCredentialsCard({ teamId }: { teamId: number | undefined }) {
  const { credentials, providers, error, isLoading, add, remove } =
    useDnsCredentials(teamId);

  const [name, setName] = useState("");
  const [provider, setProvider] = useState("");
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<number | null>(null);

  const selected = providers.find((p) => p.provider === provider);

  const onSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!name.trim() || !provider) return;
    setSubmitting(true);
    setFormError(null);
    try {
      await add(name.trim(), provider, secrets);
      setName("");
      setProvider("");
      setSecrets({});
    } catch (err) {
      setFormError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Sheet>
      <h3>DNS Provider Credentials</h3>
      <p className="mt-2 max-w-prose text-sm text-content-secondary">
        Needed only for the <code>dns-01</code> challenge — which is what lets
        you issue a wildcard certificate, or validate a domain when port 80
        isn&apos;t reachable. Tokens are encrypted before storage and never
        shown again. Saving under an existing name replaces the token.
      </p>

      <form onSubmit={onSubmit} className="mt-4 flex flex-col gap-3">
        <div className="flex flex-wrap items-end gap-2">
          <div className="min-w-40 grow">
            <TextInput
              id="dns-cred-name"
              label="Name"
              placeholder="cloudflare-prod"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-content-primary">Provider</span>
            <select
              aria-label="Provider"
              className="h-9 rounded-sm border bg-background-secondary px-2 text-sm"
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value);
                setSecrets({});
              }}
            >
              <option value="">Select…</option>
              {providers.map((p) => (
                <option key={p.provider} value={p.provider}>
                  {p.provider}
                </option>
              ))}
            </select>
          </label>
        </div>

        {selected?.fields.map((field) => (
          <TextInput
            key={field.key}
            id={`dns-cred-${field.key}`}
            label={field.label}
            type="password"
            autoComplete="off"
            description={field.help}
            value={secrets[field.key] ?? ""}
            onChange={(e) =>
              setSecrets((prev) => ({ ...prev, [field.key]: e.target.value }))
            }
          />
        ))}

        {formError && (
          <p className="text-xs text-content-error" role="alert">
            {formError}
          </p>
        )}

        <div>
          <Button
            type="submit"
            disabled={submitting || !name.trim() || !provider}
          >
            Save credential
          </Button>
        </div>
      </form>

      <div className="mt-4 flex flex-col gap-2">
        {isLoading && <Spinner className="size-4" />}
        {error && (
          <p className="text-sm text-content-error">
            Could not load credentials: {error.message}
          </p>
        )}
        {credentials?.length === 0 && (
          <p className="text-sm text-content-secondary">
            No DNS credentials yet.
          </p>
        )}
        {credentials?.map((c) => (
          <div
            key={c.id}
            className="flex items-center gap-2 rounded-sm border p-3"
          >
            <span className="grow truncate text-sm">{c.name}</span>
            <span className="text-xs text-content-secondary">{c.provider}</span>
            <Button
              size="xs"
              variant="danger"
              icon={<TrashIcon />}
              aria-label={`Remove ${c.name}`}
              onClick={() => setPendingDelete(c.id)}
            />
          </div>
        ))}
      </div>

      {pendingDelete !== null && (
        <ConfirmationDialog
          onClose={() => setPendingDelete(null)}
          onConfirm={async () => {
            await remove(pendingDelete);
            setPendingDelete(null);
          }}
          confirmText="Remove"
          variant="danger"
          dialogTitle="Remove DNS credential"
          dialogBody="Domains using this credential will fail to renew until another one is attached."
        />
      )}
    </Sheet>
  );
}
