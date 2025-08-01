import { Meta, StoryObj } from "@storybook/nextjs";
import { BreadcrumbLink } from "../BreadcrumbLink/BreadcrumbLink";
import { Breadcrumbs } from "./Breadcrumbs";

const meta = { component: Breadcrumbs } satisfies Meta<typeof Breadcrumbs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    children: [
      <BreadcrumbLink href="/">Home</BreadcrumbLink>,
      <BreadcrumbLink href="/clothing">Clothing</BreadcrumbLink>,
      <BreadcrumbLink href="/clothing/pants">Pants</BreadcrumbLink>,
    ],
  },
};

export const WithOtherChildren: Story = {
  args: {
    children: [
      <span className="p-3">span</span>,
      <h1>h1</h1>,
      <div className="w-56 bg-yellow-400">div</div>,
    ],
  },
};
