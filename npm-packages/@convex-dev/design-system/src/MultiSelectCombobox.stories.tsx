import { Meta, StoryObj } from "@storybook/nextjs";
import { ComponentProps, useState } from "react";
import { MultiSelectCombobox, MultiSelectValue } from "@ui/MultiSelectCombobox";
import { fn } from "storybook/test";

const meta = {
  component: MultiSelectCombobox,
  args: {
    options: [
      "Option 1",
      "Option 2",
      "Option 3",
      "Option 4",
      "Option 5",
      "Option 6",
      "Option 7",
      "Option 8",
      "Option 9",
      "Option 10",
    ],
    unit: "item",
    unitPlural: "items",
    label: "Select Items",
    selectedOptions: [],
    setSelectedOptions: fn(),
  },
  render: (args) => <Example {...args} />,
} satisfies Meta<typeof MultiSelectCombobox>;

export default meta;
type Story = StoryObj<typeof meta>;

function Example(args: ComponentProps<typeof MultiSelectCombobox>) {
  const [value, setValue] = useState<MultiSelectValue>(args.selectedOptions);
  return (
    <MultiSelectCombobox
      {...args}
      selectedOptions={value}
      setSelectedOptions={(newValue) => setValue(newValue)}
    />
  );
}

export const Default: Story = {};

export const WithInitialSelection: Story = {
  args: {
    selectedOptions: ["Option 1", "Option 2"],
  },
};

export const AllSelected: Story = {
  args: {
    selectedOptions: "all",
  },
};

export const WithSearchDisabled: Story = {
  args: {
    disableSearch: true,
  },
};

export const WithLabelHidden: Story = {
  args: {
    labelHidden: true,
  },
};

export const WithManyOptions: Story = {
  args: {
    options: Array.from({ length: 150 }, (_, i) => `Option ${i + 1}`),
    unit: "option",
    unitPlural: "options",
    label: "Select Options",
  },
};

export const WithCustomUnit: Story = {
  args: {
    options: ["Apple", "Banana", "Cherry", "Date", "Elderberry"],
    unit: "fruit",
    unitPlural: "fruits",
    label: "Select Fruits",
  },
};
