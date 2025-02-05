import { Meta, StoryObj } from "@storybook/react";
import { ComponentProps, useState } from "react";
import { TextInput } from "@common/elements/TextInput";

export default {
  component: TextInput,
  render: (args) => <Example {...args} />,
} as Meta<typeof TextInput>;

function Example(args: ComponentProps<typeof TextInput>) {
  const [value, setValue] = useState("");
  return (
    <TextInput
      {...args}
      value={value}
      onChange={(e) => setValue(e.target.value)}
      placeholder="Enter text here"
    />
  );
}

export const Primary: StoryObj<typeof TextInput> = { args: {} };
