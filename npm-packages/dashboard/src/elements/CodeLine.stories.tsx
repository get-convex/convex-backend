import { StoryObj } from "@storybook/nextjs";
import { CodeLine } from "./CodeLine";

export default { component: CodeLine };

export const Primary: StoryObj<typeof CodeLine> = {
  args: {
    code: "const x = 1;",
  },
};
