import { Meta, StoryFn, StoryObj } from "@storybook/nextjs";
import { Calendar } from "@common/elements/Calendar";

export const Single: Story = {
  args: {
    mode: "single",
    selected: new Date(Date.now() - 24 * 60 * 60 * 1000), // Yesterday
  },
};

export const Range: Story = {
  args: {
    mode: "range",
    selected: {
      from: new Date(Date.now() - 24 * 60 * 60 * 1000), // Yesterday
      to: new Date(Date.now() + 24 * 60 * 60 * 1000), // Tomorrow
    },
  },
};

export const RestrictedRange: Story = {
  args: {
    mode: "single",
    selected: new Date(Date.now() - 24 * 60 * 60 * 1000), // Yesterday
    fromDate: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000), // 3 days ago
    toDate: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000), // 3 days from now
  },
};

const meta = {
  component: Calendar,
  decorators: [
    (Story: StoryFn) => (
      // The calendar itself has a transparent background, but should generally be
      // placed over `background-secondary` to ensure proper contrast.
      <div className="w-min bg-background-secondary p-2">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof Calendar>;

export default meta;
type Story = StoryObj<typeof meta>;
