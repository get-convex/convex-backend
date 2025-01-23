import { StoryObj } from "@storybook/react";
import { Avatar } from "./Avatar";

export default { component: Avatar };

export const Initials: StoryObj<typeof Avatar> = {
  args: {
    name: "Zepp Williams",
  },
};

// export const Image = Template.bind({});
// Image.args = {
// name: "Zepp Williams",
// imageUrl: "https://i.pravatar.cc/200",
// };
