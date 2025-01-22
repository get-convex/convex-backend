import { StoryObj } from "@storybook/react";
import { BreadcrumbLink } from "../BreadcrumbLink/BreadcrumbLink";
import { Breadcrumbs } from "./Breadcrumbs";

export default { component: Breadcrumbs };

export const Primary: StoryObj<typeof Breadcrumbs> = {
  args: {
    children: [
      <BreadcrumbLink href="/">Home</BreadcrumbLink>,
      <BreadcrumbLink href="/clothing">Clothing</BreadcrumbLink>,
      <BreadcrumbLink href="/clothing/pants">Pants</BreadcrumbLink>,
    ],
  },
};

export const WithOtherChildren: StoryObj<typeof Breadcrumbs> = {
  args: {
    children: [
      <span className="p-3">span</span>,
      <h1>h1</h1>,
      <div className="w-56 bg-yellow-400">div</div>,
    ],
  },
};
