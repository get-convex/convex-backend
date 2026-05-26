import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked, fn } from "storybook/test";
import {
  useDeleteAccount,
  useIdentities,
  useProfileEmails,
  useUnlinkIdentity,
  useUpdateProfileName,
} from "api/profile";
import {
  useCreatePersonalAccessToken,
  useDeletePersonalAccessToken,
  usePaginatedPersonalAccessTokens,
} from "api/personalAccessTokens";
import { useDiscordAccounts, useUnlinkDiscordAccount } from "api/discord";
import { Profile } from "../../pages/profile";

const now = Date.now();
const oneDay = 86_400_000;

const meta = {
  component: Profile,
  parameters: {
    layout: "fullscreen",
    a11y: {
      test: "todo",
    },
  },
  beforeEach: () => {
    mocked(useProfileEmails).mockReturnValue([
      {
        id: 1,
        email: "nicolas@acme.dev",
        isPrimary: true,
        isVerified: true,
        creationTime: now - 365 * oneDay,
      },
      {
        id: 2,
        email: "nicolas.personal@example.com",
        isPrimary: false,
        isVerified: true,
        creationTime: now - 30 * oneDay,
      },
    ]);
    mocked(useUpdateProfileName).mockReturnValue(fn());
    mocked(useDeleteAccount).mockReturnValue(fn());
    mocked(useIdentities).mockReturnValue([]);
    mocked(useUnlinkIdentity).mockReturnValue(fn());
    mocked(usePaginatedPersonalAccessTokens).mockReturnValue({
      data: {
        items: [
          {
            name: "ci-deploy",
            creationTime: now - 30 * oneDay,
            lastUsedTime: now - 2 * oneDay,
          },
          {
            name: "local-dev",
            creationTime: now - 7 * oneDay,
            lastUsedTime: null,
          },
        ],
        pagination: { hasMore: false },
      },
      isLoading: false,
    });
    mocked(useCreatePersonalAccessToken).mockReturnValue(fn());
    mocked(useDeletePersonalAccessToken).mockReturnValue(fn());
    mocked(useDiscordAccounts).mockReturnValue([]);
    mocked(useUnlinkDiscordAccount).mockReturnValue(fn());
  },
} satisfies Meta<typeof Profile>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
