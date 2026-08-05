import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked, screen, userEvent } from "storybook/test";
import { useTeams } from "api/teams";
import { TeamIndexPage } from "../../pages/t/[team]";

const meta = {
  component: TeamIndexPage,
  parameters: {
    layout: "fullscreen",
    a11y: {
      test: "todo",
    },
  },
} satisfies Meta<typeof TeamIndexPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

// The team the docs decorator mocks, plus enough others for the switcher to
// look like a list rather than a single row.
const teams = [
  {
    id: 2,
    creator: 1,
    slug: "acme",
    name: "Acme Corp",
    suspended: false,
    referralCode: "ACME01",
    referredBy: null,
  },
  {
    id: 3,
    creator: 1,
    slug: "acme-labs",
    name: "Acme Labs",
    suspended: false,
    referralCode: "ACME02",
    referredBy: null,
  },
  {
    id: 4,
    creator: 1,
    slug: "nicolas-ettlin",
    name: "Nicolas’s Team",
    suspended: false,
    referralCode: "NICO01",
    referredBy: null,
  },
];

/**
 * The team switcher the header's team name opens.
 */
export const TeamSwitcher: Story = {
  parameters: {
    screenshotSelector:
      '[aria-label="Switch team"], .command-palette--anchored',
    // The menu's list is capped at min(330px, 40vh): at the default 700px-tall
    // viewport the 40vh half of that clips the last team.
    screenshotViewport: { width: 1024, height: 1000 },
  },
  decorators: [
    (storyFn) => {
      mocked(useTeams).mockReturnValue({
        selectedTeamSlug: "acme",
        teams,
      });
      return storyFn();
    },
  ],
  play: async () => {
    // The header is rendered by the docs decorator and the palette portals to
    // document.body, so query the whole screen rather than the story canvas.
    await userEvent.click(await screen.findByLabelText("Switch team"));
    await screen.findByText("Switch Team");
  },
};
