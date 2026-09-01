import { CreditResponse } from "generatedApi";
import { formatUtcDate } from "@common/lib/format";
import { formatUsd, toast } from "@common/lib/utils";
import { HelpTooltip } from "@ui/HelpTooltip";
import { Tooltip } from "@ui/Tooltip";
import { Donut } from "@ui/Donut";
import { TextInput } from "@ui/TextInput";
import { Button } from "@ui/Button";
import { Loading } from "@ui/Loading";
import { type FormEvent, useState } from "react";

export function PrepaidCredits({
  credits,
  isLoading,
  teamId,
  onPromoRedeemed,
}: {
  credits: CreditResponse[];
  isLoading: boolean;
  teamId: number;
  onPromoRedeemed: () => Promise<void>;
}) {
  return (
    <>
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-1">
          <h4>Credits</h4>
          {credits.length > 0 && (
            <HelpTooltip tipSide="right">
              Credits are applied to your invoices before your payment method is
              charged. They're spent soonest-expiring first, and any balance
              left over when a credit expires is forfeited.
            </HelpTooltip>
          )}
        </div>
        {isLoading ? (
          <CreditSkeleton />
        ) : credits.length > 0 ? (
          <div className="flex flex-col gap-3">
            {credits.map((credit) => (
              <Credit key={credit.id} credit={credit} />
            ))}
          </div>
        ) : null}
        <PromoCodeForm teamId={teamId} onPromoRedeemed={onPromoRedeemed} />
      </div>
      <hr />
    </>
  );
}

function CreditSkeleton() {
  return (
    <Loading className="flex items-center gap-3" fullHeight={false}>
      <span className="sr-only">Loading credits</span>
      <div className="size-6 shrink-0 rounded-full bg-neutral-8/30 dark:bg-neutral-3/20" />
      <div className="flex min-w-0 grow flex-col gap-2 py-0.5">
        <div className="h-3 w-40 rounded-sm bg-neutral-8/30 dark:bg-neutral-3/20" />
        <div className="flex justify-between gap-4">
          <div className="h-3 w-32 rounded-sm bg-neutral-8/30 dark:bg-neutral-3/20" />
          <div className="h-3 w-24 rounded-sm bg-neutral-8/30 dark:bg-neutral-3/20" />
        </div>
      </div>
    </Loading>
  );
}

function PromoCodeForm({
  teamId,
  onPromoRedeemed,
}: {
  teamId: number;
  onPromoRedeemed: () => Promise<void>;
}) {
  const [code, setCode] = useState("");
  const [error, setError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const redeemPromo = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmedCode = code.trim();
    if (!trimmedCode) {
      setError("Enter a promo code.");
      return;
    }

    setError(undefined);
    setIsSubmitting(true);
    try {
      const response = await fetch("/api/redeem-promo", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code: trimmedCode, teamId }),
      });
      const result = (await response.json()) as {
        error?: string;
        credit_amount: number;
      };
      if (!response.ok) {
        setError(result.error ?? "Unable to redeem this promo code.");
        return;
      }

      setCode("");
      toast(
        "success",
        `${formatUsd(result.credit_amount)} in credits added to your team.`,
      );
      try {
        await onPromoRedeemed();
      } catch {
        toast(
          "error",
          "Credits were added, but the credit list could not be refreshed.",
        );
      }
    } catch {
      setError("Unable to redeem the promo code. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form className="flex max-w-md items-start gap-2" onSubmit={redeemPromo}>
      <TextInput
        id="promoCode"
        label="Promo code"
        labelHidden
        placeholder="Enter promo code"
        value={code}
        onChange={(event) => {
          setCode(event.target.value);
          setError(undefined);
        }}
        error={error}
        disabled={isSubmitting}
        autoComplete="off"
      />
      <Button type="submit" loading={isSubmitting} disabled={!code.trim()}>
        Redeem
      </Button>
    </form>
  );
}

function Credit({ credit }: { credit: CreditResponse }) {
  const { balance, initialBalance } = credit;
  const used = Math.max(0, initialBalance - balance);
  const usedLabel = `${formatUsd(used)} of ${formatUsd(initialBalance)} used`;
  const percentUsed = initialBalance > 0 ? (100 * used) / initialBalance : 0;

  return (
    <div className="flex items-center gap-3">
      <Tooltip
        side="bottom"
        tip={`You've used ${percentUsed.toFixed(1)}% of this credit.`}
        className="flex items-center"
      >
        <Donut current={used} max={initialBalance} />
      </Tooltip>
      <div className="flex min-w-0 grow flex-col gap-0.5">
        <CreditTitle credit={credit} />
        <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 text-sm">
          <span>{usedLabel}</span>
          <span className="text-content-secondary">
            <CreditStatus credit={credit} />
          </span>
        </div>
      </div>
    </div>
  );
}

// Credit from a plan allocation is named after what it was allocated for;
// credit granted by hand carries a note instead. A credit can have both, and a
// hand-granted one that wasn't annotated has neither.
function CreditTitle({ credit }: { credit: CreditResponse }) {
  const { itemName, description } = credit;
  if (!itemName && !description) {
    return null;
  }
  return (
    <div className="flex flex-wrap items-baseline gap-x-2 text-sm">
      {itemName && <span className="font-medium">{itemName}</span>}
      {description && (
        // Standing alone, the note is the credit's name and reads as one.
        <span className={itemName ? "text-content-secondary" : "font-medium"}>
          {description}
        </span>
      )}
    </div>
  );
}

function CreditStatus({ credit }: { credit: CreditResponse }) {
  const { expiryDate } = credit;

  if (expiryDate !== null && expiryDate !== undefined) {
    return <>Expires on {formatUtcDate(new Date(expiryDate))}</>;
  }
  return <>Never expires</>;
}
