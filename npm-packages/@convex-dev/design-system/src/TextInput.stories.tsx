import { Meta, StoryObj } from "@storybook/nextjs";
import { ComponentProps, useState } from "react";
import { TextInput } from "@ui/TextInput";

const meta = {
  component: TextInput,
  render: (args) => <Example {...args} />,
} satisfies Meta<typeof TextInput>;

export default meta;
type Story = StoryObj<typeof meta>;

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

export const Primary: Story = { args: {} };
