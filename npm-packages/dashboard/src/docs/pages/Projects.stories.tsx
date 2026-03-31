import { Meta, StoryObj } from "@storybook/nextjs";
import { TeamIndexPage } from "../../pages/t/[team]";

const meta = {
  component: TeamIndexPage,
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta<typeof TeamIndexPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
