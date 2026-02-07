import React, { useMemo } from "react";
import { renderHook } from "@testing-library/react";
import {
  DeploymentInfoContext,
  DeploymentInfo,
} from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import { useIsCloudDeploymentInSelfHostedDashboard } from "./useIsCloudDeploymentInSelfHostedDashboard";

function createWrapper(contextOverrides: Partial<DeploymentInfo>) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    const value = useMemo(
      () => ({
        ...mockDeploymentInfo,
        ...contextOverrides,
      }),
      [],
    ) as DeploymentInfo; // using `as` because DeploymentInfo is a discriminated union
    return (
      <DeploymentInfoContext.Provider value={value}>
        {children}
      </DeploymentInfoContext.Provider>
    );
  };
}

describe("useIsCloudDeploymentInSelfHostedDashboard", () => {
  describe("when dashboard is not self-hosted", () => {
    it("returns false when isSelfHosted is false", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: false,
            ok: true,
            deploymentUrl: "https://fine-marmot-266.convex.cloud",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: false,
        deploymentName: undefined,
      });
    });
  });

  describe("when dashboard is self-hosted", () => {
    it("returns false when deploymentUrl is not loaded", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: false,
            errorCode: "TestError",
            errorMessage: "Test error message",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: false,
        deploymentName: undefined,
      });
    });
  });

  describe("when self-hosted with cloud deployment URL", () => {
    it("returns true with deployment name for valid cloud URL", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: true,
            deploymentUrl: "https://fine-marmot-266.convex.cloud",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: true,
        deploymentName: "fine-marmot-266",
      });
    });

    it("returns true with deployment name in non-default region", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: true,
            deploymentUrl: "https://basic-whale-224.eu-west-1.convex.cloud",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: true,
        deploymentName: "basic-whale-224",
      });
    });
  });

  describe("when self-hosted with non-cloud deployment URL", () => {
    it("returns false for custom domain", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: true,
            deploymentUrl: "https://api.sync.t3.chat",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: false,
        deploymentName: undefined,
      });
    });

    it("returns false for localhost URL", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: true,
            deploymentUrl: "http://localhost:3210",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: false,
        deploymentName: undefined,
      });
    });

    it("returns false for URL with wrong TLD", () => {
      const { result } = renderHook(
        () => useIsCloudDeploymentInSelfHostedDashboard(),
        {
          wrapper: createWrapper({
            isSelfHosted: true,
            ok: true,
            deploymentUrl: "https://fine-marmot-266.convex.site",
          }),
        },
      );

      expect(result.current).toEqual({
        isCloudDeploymentInSelfHostedDashboard: false,
        deploymentName: undefined,
      });
    });
  });
});
