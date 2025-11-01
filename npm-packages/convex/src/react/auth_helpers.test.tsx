/**
 * @vitest-environment custom-vitest-environment.ts
 */
import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, test } from "vitest";
import { Authenticated, AuthLoading, Unauthenticated } from "./auth_helpers.js";
import { ConvexReactClient } from "./client.js";
import { ConvexProviderWithAuth } from "./index.js";

function mockUseAuth(isLoading = true, isAuthenticated = false) {
  return () => {
    return {
      isAuthenticated: isAuthenticated,
      isLoading: isLoading,
      fetchAccessToken: async () => null,
    };
  };
}

test("Helpers are valid children", () => {
  const _element = (
    <div>
      <Authenticated>Yay</Authenticated>
      <Unauthenticated>Nay</Unauthenticated>
      <AuthLoading>???</AuthLoading>
    </div>
  );
});

test("Helpers can take many children", () => {
  const _element = (
    <div>
      <Authenticated>
        <div>Yay</div>
        <div>Yay again</div>
      </Authenticated>
      <Unauthenticated>
        <div>Yay</div>
        <div>Yay again</div>
      </Unauthenticated>
      <AuthLoading>
        <div>Yay</div>
        <div>Yay again</div>
      </AuthLoading>
    </div>
  );
});

describe("<Unauthenticated />", () => {
  test("renders children when loading and prop loadingEqualsUnauthenticated is true", () => {
    const convex = new ConvexReactClient("https://127.0.0.1:3001");

    const { unmount } = render(
      <ConvexProviderWithAuth client={convex} useAuth={mockUseAuth(true)}>
        <Unauthenticated loadingEqualsUnauthenticated>
          <div>Yay</div>
        </Unauthenticated>
      </ConvexProviderWithAuth>,
    );

    expect(screen.getByText("Yay")).toBeTruthy();
    unmount();
  });

  test("shows no children when loading", () => {
    const convex = new ConvexReactClient("https://127.0.0.1:3001");

    const { unmount } = render(
      <ConvexProviderWithAuth client={convex} useAuth={mockUseAuth(true)}>
        <Unauthenticated>
          <div>Yay</div>
        </Unauthenticated>
      </ConvexProviderWithAuth>,
    );

    expect(screen.queryByText("Yay")).toBeNull();
    unmount();
  });
});
