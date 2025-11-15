import classNames from "classnames";
import { useCurrentTeam } from "api/teams";
import { useListInvoices } from "api/billing";
import Link from "next/link";

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
      <Link
        href={`/t/${team?.slug}/settings/billing#paymentMethod`}
        passHref
        className="text-content-link hover:underline"
      >
        Update your payment method
      </Link>{" "}
      to avoid a service interruption.
    </div>
  );
}

export function useShowFailedPaymentBanner() {
  const team = useCurrentTeam();
  const { invoices } = useListInvoices(team?.managedBy ? undefined : team?.id);
  const failedInvoice = invoices
    ? invoices.find(
        (invoice) => invoice.status === "issued" && invoice.hasFailedPayment,
      )
    : undefined;

  return failedInvoice !== undefined;
}
