import { Meta, StoryObj } from "@storybook/nextjs";
import { useEffect, useState } from "react";
import { ProgressBar } from "@ui/ProgressBar";

const meta = {
  component: ProgressBar,
  args: { ariaLabel: "Progress" },
  render: (args: Parameters<typeof ProgressBar>[0]) => (
    <ProgressBar {...args} />
  ),
} satisfies Meta<typeof ProgressBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Indeterminate: Story = {
  args: { fraction: undefined },
};

export const Empty: Story = {
  args: { fraction: 0 },
};

export const Half: Story = {
  args: { fraction: 0.5 },
};

export const Full: Story = {
  args: { fraction: 1 },
};

export const AnimatedValue: Story = {
  render: () => <Animation />,
};

export const Solid: Story = {
  args: { fraction: 0.5, variant: "solid" },
};

function Animation() {
  const [value, setValue] = useState(0);
  useEffect(() => {
    const interval = setInterval(() => {
      setValue((v) => (v + 20) % 120);
    }, 800);
    return () => clearInterval(interval);
  }, []);
  return <ProgressBar fraction={value / 100} ariaLabel="Progress" />;
}
