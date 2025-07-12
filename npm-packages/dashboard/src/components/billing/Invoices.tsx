import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { cn } from "@ui/cn";
import { InvoiceResponse } from "generatedApi";

const headerClass = "text-left text-xs text-content-secondary font-normal py-2";

export function Invoices({ invoices }: { invoices: InvoiceResponse[] }) {
  return (
    <Sheet className="flex w-full flex-col gap-4">
      <h3>Invoices</h3>
      <span className="text-sm">
        Preview or download your upcoming and past invoices.
      </span>
      {invoices.length > 0 ? (
        <InvoicesTable invoices={invoices} />
      ) : (
        <div className="my-24 flex flex-col items-center gap-2 text-content-secondary">
          No invoices yet.
        </div>
      )}
      <p className="flex flex-col gap-1 text-xs text-content-secondary">
        Looking for older invoices? Invoices from before May 2024 were emailed
        to the team creator.
      </p>
    </Sheet>
  );
}

function InvoicesTable({ invoices }: { invoices: InvoiceResponse[] }) {
  return (
    <div className="scrollbar max-h-[30rem] overflow-y-auto rounded-sm border">
      <table className="w-full">
        <thead className="sticky top-0 z-10 border-b bg-background-secondary">
          <tr>
            <th className={cn(headerClass, "pl-2")}>Invoice</th>
            <th className={headerClass}>Issue Date</th>
            <th className={headerClass}>Status</th>
            <th className={headerClass}>Amount</th>
            <th className={cn(headerClass, "pr-2")}>Receipt</th>
          </tr>
        </thead>
        <tbody>
          {invoices.map((invoice) => (
            <tr key={invoice.id} className="hover:bg-background-tertiary">
              <td className="p-2 text-sm">
                {invoice.status === "draft" ? (
                  <span className="text-content-secondary">Upcoming</span>
                ) : (
                  invoice.invoiceNumber
                )}
              </td>
              <td className="py-2 text-sm">
                {new Date(invoice.invoiceDate).toLocaleDateString()}
              </td>
              <td className="py-2 text-sm">
                <StatusPill
                  status={invoice.hasFailedPayment ? "failed" : invoice.status}
                />
              </td>
              <td className="py-2 text-sm">
                {parseFloat(invoice.total).toLocaleString("en-US", {
                  currency: invoice.currency,
                  style: "currency",
                  currencyDisplay: "symbol",
                })}
              </td>

              <td className="py-2 pr-2 text-sm">
                <Button
                  tip={
                    !invoice.hostedInvoiceUrl &&
                    "Could not generate link to this invoice."
                  }
                  tipSide="right"
                  size="xs"
                  inline
                  disabled={!invoice.hostedInvoiceUrl}
                  href={invoice.hostedInvoiceUrl || undefined}
                  target="_blank"
                >
                  {invoice.status === "draft"
                    ? "Preview Upcoming Invoice"
                    : "View Invoice"}
                  <ExternalLinkIcon />
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const statusColors: Record<InvoiceResponse["status"], string> = {
  paid: "bg-background-success",
  issued: "bg-background-warning",
  draft: "bg-blue-200 dark:bg-blue-800",
  void: "bg-background-error",
  failed: "bg-background-error",
  synced: "bg-background-tertiary",
};
function StatusPill({ status }: { status: InvoiceResponse["status"] }) {
  const color = statusColors[status] || "bg-blue-200";
  return (
    <span className={cn("rounded-sm p-1 text-xs", color)}>
      {status === "draft"
        ? "Preview"
        : status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
}
