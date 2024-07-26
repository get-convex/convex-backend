/**
 * @vitest-environment jsdom
 */
import { test } from "vitest";
import React from "react";
import { Authenticated, AuthLoading, Unauthenticated } from "./auth_helpers.js";

test.skip("Helpers are valid children", () => {
  const _element = (
    <div>
      <Authenticated>Yay</Authenticated>
      <Unauthenticated>Nay</Unauthenticated>
      <AuthLoading>???</AuthLoading>
    </div>
  );
});

test.skip("Helpers can take many children", () => {
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
