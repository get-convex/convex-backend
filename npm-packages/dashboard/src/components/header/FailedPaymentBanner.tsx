import classNames from "classnames";
import { useCurrentTeam } from "api/teams";
import { useHasFailedPayment } from "api/billing";
import { Link } from "@ui/Link";

export function FailedPaymentBanner() {
  const team = useCurrentTeam();
  return (
    <div
      className={classNames(
        "flex flex-wrap items-center px-2 py-1 gap-1 border-b h-24 sm:h-12 overflow-x-hidden",
        "bg-background-error text-content-error text-xs",
      )}
    >
      Your latest subscription payment has failed.{" "}
      <Link href={`/t/${team?.slug}/settings/billing#paymentMethod`} passHref>
        Update your payment method
      </Link>{" "}
      to avoid a service interruption.
    </div>
  );
}

export function useShowFailedPaymentBanner() {
  const team = useCurrentTeam();
  const { hasFailedPayment } = useHasFailedPayment(
    team?.managedBy === "vercel" ? undefined : team?.id,
  );
  return hasFailedPayment;
}
