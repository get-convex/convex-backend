import { Meta, StoryObj } from "@storybook/nextjs";
import { EmptyFunctions } from "./FunctionsView";

const meta = {
  component: EmptyFunctions,
  render: () => (
    <div className="h-screen">
      <EmptyFunctions />
    </div>
  ),
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof EmptyFunctions>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
