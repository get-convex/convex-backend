import { Meta, StoryObj } from "@storybook/nextjs";
import { SlotMachine } from "./SlotMachine";
import { useEffect, useState } from "react";

const meta = {
  component: SlotMachine,
  render: (args: any) => <SlotMachine {...args} />,
  decorators: [
    (Story: any) => (
      <div className="flex min-h-[200px] items-center justify-center bg-background-primary p-8">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof SlotMachine>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Spins indefinitely without stopping */
export const Spinning: Story = {
  args: {},
};

export const WithDeploymentName: Story = {
  args: {
    deploymentName: "bold-falcon-42",
  },
};

export const AnotherDeploymentName: Story = {
  args: {
    deploymentName: "groovy-wildcat-999",
  },
};

/** Simulates a deployment name arriving after a 3-second delay */
function DelayedExample() {
  const [name, setName] = useState<string | undefined>(undefined);
  useEffect(() => {
    const timer = setTimeout(() => {
      setName("clever-penguin-456");
    }, 3000);
    return () => clearTimeout(timer);
  }, []);
  return (
    <div className="flex flex-col items-center gap-4">
      <SlotMachine deploymentName={name} />
      <p className="text-sm text-content-primary">
        {name
          ? `Deployment name assigned: ${name}`
          : "Waiting for deployment name..."}
      </p>
    </div>
  );
}

export const DelayedAssignment: Story = {
  render: () => <DelayedExample />,
};

/** Simulates a 6-second delay before assignment */
function LongDelayExample() {
  const [name, setName] = useState<string | undefined>(undefined);
  useEffect(() => {
    const timer = setTimeout(() => {
      setName("zealous-octopus-7");
    }, 6000);
    return () => clearTimeout(timer);
  }, []);
  return (
    <div className="flex flex-col items-center gap-4">
      <SlotMachine deploymentName={name} />
      <p className="text-sm text-content-primary">
        {name
          ? `Deployment name assigned: ${name}`
          : "Spinning for 6 seconds..."}
      </p>
    </div>
  );
}

export const LongDelay: Story = {
  render: () => <LongDelayExample />,
};

/** Text-only mode: no emojis, spinning */
export const TextOnlySpinning: Story = {
  args: {
    showEmoji: false,
  },
};

/** Text-only mode: stops at a deployment name */
export const TextOnlyWithName: Story = {
  args: {
    showEmoji: false,
    deploymentName: "bold-falcon-42",
  },
};

/** Reduced motion: loading state (no deployment name yet) */
export const ReducedMotionLoading: Story = {
  args: {
    forceReducedMotion: true,
  },
};

/** Reduced motion: resolved state with deployment name */
export const ReducedMotionResolved: Story = {
  args: {
    forceReducedMotion: true,
    deploymentName: "clever-penguin-456",
  },
};

/** Unknown adjective and animal: shows question mark emojis */
export const UnknownNames: Story = {
  args: {
    deploymentName: "xylophone-quokka-99",
  },
};
