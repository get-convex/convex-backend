import Link from "next/link";
import React from "react";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { Sheet } from "@common/elements/Sheet";
import { CopyTextButton } from "@common/elements/CopyTextButton";

export function AuthConfig() {
  const authProviders = useQuery(udfs.listAuthProviders.default);
  return (
    <Sheet>
      <div>
        <h3 className="mb-4">Authentication Configuration</h3>
        {!authProviders || authProviders.length === 0 ? (
          <p className="text-sm">
            This deployment has no configured authentication providers.{" "}
            <Link
              passHref
              href="https://docs.convex.dev/auth"
              className="text-content-link dark:underline"
              target="_blank"
            >
              Learn more
            </Link>
          </p>
        ) : (
          <>
            <p className="mt-4 text-sm">
              These are the authentication providers configured for this
              deployment.
            </p>
            <div className="flex max-w-3xl flex-col divide-y divide-border-transparent">
              {authProviders?.map((provider, i) => (
                <div key={i} className="flex flex-wrap gap-4 py-6">
                  <ProviderAttribute label="Domain" value={provider.domain} />
                  <ProviderAttribute
                    label="Application ID"
                    value={provider.applicationID}
                  />
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </Sheet>
  );
}

export function ProviderAttribute({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="flex flex-col gap-2 text-xs">
      <span className="font-semibold">{label}</span>
      <CopyTextButton text={value} className="text-sm font-normal" />
    </div>
  );
}
