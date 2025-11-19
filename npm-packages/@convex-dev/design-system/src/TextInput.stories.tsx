import { Meta, StoryObj } from "@storybook/nextjs";
import { ComponentProps, useState } from "react";
import { TextInput } from "@ui/TextInput";
import { fn } from "storybook/test";

const meta = {
  component: TextInput,
  render: (args) => <Example {...args} />,
  args: {
    value: "",
    onChange: fn(),
    id: "text-input",
  },
} satisfies Meta<typeof TextInput>;

export default meta;
type Story = StoryObj<typeof meta>;

function Example(args: ComponentProps<typeof TextInput>) {
  const [value, setValue] = useState(args.value);
  return (
    <TextInput
      {...args}
      value={value}
      onChange={(e) => setValue(e.target.value)}
      placeholder="Enter text here"
    />
  );
}

export const Primary: Story = {};
