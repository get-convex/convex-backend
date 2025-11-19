import { Meta, StoryObj } from "@storybook/nextjs";
import { ComponentProps, useState } from "react";
import { Combobox } from "@ui/Combobox";
import { fn } from "storybook/test";

const meta = {
  component: Combobox,
  args: {
    label: "My combobox",
    options: [
      { label: "Option 1", value: "1" },
      { label: "Option 2", value: "2" },
      { label: "Option 3", value: "3" },
      { label: "Option 4", value: "4" },
      { label: "Option 5", value: "5" },
      { label: "Option 6", value: "6" },
      { label: "Option 7", value: "7" },
      { label: "Option 8", value: "8" },
      { label: "Option 9", value: "9" },
      { label: "Option 10", value: "10" },
      { label: "Option 11", value: "11" },
      { label: "Option 12", value: "12" },
      { label: "Option 13", value: "13" },
      { label: "Option 14", value: "14" },
      { label: "Option 15", value: "15" },
      { label: "Option 16", value: "16" },
      { label: "Option 17", value: "17" },
      { label: "Option 18", value: "18" },
      { label: "Option 19", value: "19" },
      { label: "Option 20", value: "20" },
    ],
    selectedOption: "1",
    setSelectedOption: fn(),
  },
  render: (args) => <Example {...args} />,
} satisfies Meta<typeof Combobox>;

export default meta;
type Story = StoryObj<typeof meta>;

function Example<T>(args: Omit<ComponentProps<typeof Combobox<T>>, "Option">) {
  const [selectedOption, setSelectedOption] = useState<T | null | undefined>(
    args.selectedOption,
  );
  return (
    <Combobox
      {...args}
      selectedOption={selectedOption}
      setSelectedOption={(opt: T | null) => opt && setSelectedOption(opt)}
    />
  );
}

export const Default: Story = {};
export const WithTooltip: Story = {
  args: {
    buttonProps: {
      tip: "Switch between components installed in this deployment.",
      tipSide: "right",
    },
  },
};
