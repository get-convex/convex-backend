import { Meta, StoryObj } from "@storybook/react";
import { InvoiceResponse } from "generatedApi";
import { Invoices } from "./Invoices";

export default {
  component: Invoices,
} as Meta<typeof Invoices>;

export const NoInvoices: StoryObj<typeof Invoices> = {
  args: {
    invoices: [],
  },
};

function generateInvoices(num: number): InvoiceResponse[] {
  return Array.from({ length: num }, (_, i) => ({
    id: `${i}`,
    invoiceNumber: `INV-${i}`,
    total: `${Math.random() * 1000}`,
    invoiceDate: new Date().getTime(),
    status: ["synced", "paid", "issued", "draft"][
      i % 4
    ] as InvoiceResponse["status"],
    currency: "USD",
    hasFailedPayment: i % 2 === 0,
  }));
}

export const WithInvoices: StoryObj<typeof Invoices> = {
  args: {
    invoices: generateInvoices(5),
  },
};

export const WithPagesOfInvoices: StoryObj<typeof Invoices> = {
  args: {
    invoices: generateInvoices(55),
  },
};
