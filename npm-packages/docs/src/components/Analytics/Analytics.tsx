import React from "react";
import CookieBanner from "./CookieBanner";
import PostHog from "./PostHog";

export default function Analytics() {
  return (
    <>
      <PostHog />
      <CookieBanner />
    </>
  );
}
