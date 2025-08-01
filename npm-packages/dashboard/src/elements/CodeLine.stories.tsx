import { Meta, StoryObj } from "@storybook/nextjs";
import { CodeLine } from "./CodeLine";

const meta = { component: CodeLine } satisfies Meta<typeof CodeLine>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    code: "const x = 1;",
  },
};
