import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked, fn } from "storybook/test";
import {
  usePersonalAccessTokens,
  useCreatePersonalAccessToken,
  useDeletePersonalAccessToken,
} from "api/personalAccessTokens";
import { useTeams } from "api/teams";
import { PersonalAccessTokens } from "./PersonalAccessTokens";

const now = Date.now();
const oneDay = 86400000;

const meta = {
  component: PersonalAccessTokens,
  beforeEach: () => {
    mocked(useCreatePersonalAccessToken).mockReturnValue(fn());
    mocked(useDeletePersonalAccessToken).mockReturnValue(fn());
    mocked(useTeams).mockReturnValue({
      selectedTeamSlug: "my-team",
      teams: [
        { id: 1, name: "My Team", slug: "my-team" } as ReturnType<
          typeof useTeams
        >["teams"] extends (infer T)[] | undefined
          ? T
          : never,
      ],
    });
  },
} satisfies Meta<typeof PersonalAccessTokens>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  beforeEach: () => {
    mocked(usePersonalAccessTokens).mockReturnValue([]);
  },
};

export const WithTokens: Story = {
  beforeEach: () => {
    mocked(usePersonalAccessTokens).mockReturnValue([
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
    ]);
  },
};

export const WithSSOToken: Story = {
  beforeEach: () => {
    mocked(usePersonalAccessTokens).mockReturnValue([
      {
        name: "ci-deploy",
        creationTime: now - 30 * oneDay,
        lastUsedTime: now - 2 * oneDay,
        ssoTeamId: 1,
      },
      {
        name: "personal-token",
        creationTime: now - 14 * oneDay,
        lastUsedTime: now - oneDay,
      },
    ]);
  },
};

export const Loading: Story = {
  beforeEach: () => {
    mocked(usePersonalAccessTokens).mockReturnValue(
      undefined as unknown as ReturnType<typeof usePersonalAccessTokens>,
    );
  },
};
