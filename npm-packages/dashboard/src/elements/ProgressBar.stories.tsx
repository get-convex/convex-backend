import { StoryObj } from "@storybook/react";
import { useEffect, useState } from "react";
import { Sheet } from "dashboard-common";
import { ProgressBar } from "./ProgressBar";

export default {
  component: ProgressBar,
  args: { ariaLabel: "Progress" },
  render: (args: Parameters<typeof ProgressBar>[0]) => (
    <Sheet>
      <ProgressBar {...args} />
    </Sheet>
  ),
};

type Story = StoryObj<typeof ProgressBar>;

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

function Animation() {
  const [value, setValue] = useState(0);
  useEffect(() => {
    const interval = setInterval(() => {
      setValue((v) => (v + 20) % 120);
    }, 800);
    return () => clearInterval(interval);
  }, []);
  return (
    <Sheet>
      <ProgressBar fraction={value / 100} ariaLabel="Progress" />
    </Sheet>
  );
}
