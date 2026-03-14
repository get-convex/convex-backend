import { Meta, StoryObj } from "@storybook/nextjs";
import { mocked, fn } from "storybook/test";
import { TeamAccessTokenResponse } from "generatedApi";
import { ProjectSettingsPage } from "../../../pages/t/[team]/[project]/settings";
import { useUpdateProject } from "api/projects";
import {
  useCurrentProjectRoles,
  useProjectRoles,
  useUpdateProjectRoles,
} from "api/roles";
import {
  useCreateTeamAccessToken,
  useDeleteAccessToken,
  useDeleteAppAccessTokenByName,
  useProjectAccessTokens,
  useProjectAppAccessTokens,
} from "api/accessTokens";
import {
  useProjectEnvironmentVariables,
  useUpdateProjectEnvVars,
} from "api/environmentVariables";
import { useBBMutation } from "api/api";

const mockProjectRoles = [
  {
    memberId: 1,
    projectId: 7,
    role: "admin",
  },
];

const mockToken: TeamAccessTokenResponse = {
  accessToken: "tok_abc123",
  serializedAccessToken: "preview_key_serialized",
  name: "storybook-preview-key",
  creator: 1,
  creationTime: Date.now() - 1000 * 60 * 60,
  lastUsedTime: Date.now() - 1000 * 60 * 30,
};

const mockAppToken: NonNullable<
  ReturnType<typeof useProjectAppAccessTokens>
>[number] = {
  appClientId: "storybook-app-client-id",
  appName: "Storybook OAuth App",
  name: "storybook-preview-key",
  creationTime: Date.now() - 1000 * 60 * 60,
  lastUsedTime: Date.now() - 1000 * 60 * 30,
};

function applyProjectSettingsMocks() {
  mocked(useProjectRoles).mockReturnValue({
    isLoading: false,
    projectRoles: mockProjectRoles as any,
  });
  mocked(useCurrentProjectRoles).mockReturnValue(mockProjectRoles as any);
  mocked(useUpdateProjectRoles).mockReturnValue(fn());
  mocked(useUpdateProject).mockReturnValue(fn());
  mocked(useProjectEnvironmentVariables).mockReturnValue({
    configs: [],
  });
  mocked(useUpdateProjectEnvVars).mockReturnValue(fn());
  mocked(useCreateTeamAccessToken).mockReturnValue(fn());
  mocked(useProjectAccessTokens).mockReturnValue([mockToken]);
  mocked(useProjectAppAccessTokens).mockReturnValue([mockAppToken]);
  mocked(useDeleteAccessToken).mockReturnValue(fn());
  mocked(useDeleteAppAccessTokenByName).mockReturnValue(fn());
  mocked(useBBMutation).mockReturnValue(fn());
}

const meta = {
  component: ProjectSettingsPage,
  parameters: {
    layout: "fullscreen",
    nextjs: {
      router: {
        pathname: "/t/[team]/[project]/settings",
        route: "/t/[team]/[project]/settings",
        basePath: `/t/acme/my-amazing-app/settings`,
        asPath: `/t/acme/my-amazing-app/settings`,
        query: {
          team: "acme",
          project: "my-amazing-app",
        },
      },
    },
  },
  beforeEach: () => {
    applyProjectSettingsMocks();
  },
} satisfies Meta<typeof ProjectSettingsPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
