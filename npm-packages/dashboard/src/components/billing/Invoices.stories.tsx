import { Meta, StoryObj } from "@storybook/nextjs";
import { InvoiceResponse } from "generatedApi";
import { Invoices } from "./Invoices";

const meta = {
  component: Invoices,
} satisfies Meta<typeof Invoices>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NoInvoices: Story = {
  args: {
    invoices: [],
  },
};

function generateInvoices(num: number): InvoiceResponse[] {
  return Array.from({ length: num }, (_, i) => ({
    id: `${i}`,
    invoiceNumber: `INV-${i}`,
    amountDue: `${Math.random() * 1000}`,
    total: `${Math.random() * 1000}`,
    invoiceDate: new Date().getTime(),
    status: ["synced", "paid", "issued", "draft"][
      i % 4
    ] as InvoiceResponse["status"],
    currency: "USD",
    hasFailedPayment: i % 2 === 0,
  }));
}

export const WithInvoices: Story = {
  args: {
    invoices: generateInvoices(5),
  },
};

export const WithPagesOfInvoices: Story = {
  args: {
    invoices: generateInvoices(55),
  },
};
