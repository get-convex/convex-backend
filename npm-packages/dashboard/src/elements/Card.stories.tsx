import { StoryObj } from "@storybook/nextjs";
import { Card } from "./Card";

export default { component: Card };

type Story = StoryObj<typeof Card>;

export const Primary: Story = {
  args: { children: "Card content" },
};
