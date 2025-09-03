import { useRouter } from "next/router";
import { LDMultiKindContext } from "launchdarkly-js-sdk-common";
import { useMemo } from "react";
import { createGlobalState } from "react-use";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { useProfile } from "api/profile";
import { useCurrentTeam } from "api/teams";
import { useProjects } from "api/projects";
import { useCurrentDeployment } from "api/deployments";

export const useGlobalLDContext = createGlobalState<
  LDMultiKindContext | undefined
>();

export const useLDContext = () => {
  const router = useRouter();
  const team = useCurrentTeam();
  const profile = useProfile();

  const projects = useProjects(team?.id);

  const project =
    projects && projects.find((p) => p.slug === router.query.project);
  return useMemo(() => {
    if (
      !router.isReady ||
      !profile ||
      (router.query.team && !team) ||
      (router.query.project && !project)
    ) {
      return undefined;
    }
    const ctx: LDMultiKindContext = {
      kind: "multi",
      user: {
        key: profile.id.toString(),
        email: profile.email,
        id: profile.id,
        _meta: {
          privateAttributes: ["email"],
        },
      },
    };
    if (team) {
      ctx.team = {
        key: team.id.toString(),
        name: team.name,
        slug: team.slug,
      };
    }
    if (project) {
      ctx.project = {
        key: project.id.toString(),
        name: project.name,
        slug: project.slug,
        createTime: project.createTime,
      };
    }

    return ctx;
  }, [
    router.isReady,
    router.query.team,
    router.query.project,
    profile,
    team,
    project,
  ]);
};

export const useLDContextWithDeployment = () => {
  const ctx = useLDContext();
  const serverVersion = useQuery(udfs.getVersion.default);
  const deployment = useCurrentDeployment();

  if (!ctx || serverVersion === undefined || !deployment) {
    return undefined;
  }

  ctx.deployment = {
    // The deployment name is unique.
    key: deployment.name,
    type: deployment.deploymentType,
    createTime: deployment.createTime,
    serverVersion,
    // Same as serverVersion, but renaming to npmPackageVersion.
    // Keeping both here for now to avoid breaking changes to existing
    // flag configs.
    npmPackageVersion: serverVersion,
  };
  return ctx;
};
