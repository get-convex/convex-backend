import React from "react";
import { useWindowSize } from "@docusaurus/theme-common";
import DocSidebarDesktop from "@theme/DocSidebar/Desktop";
import DocSidebarMobile from "@theme/DocSidebar/Mobile";
export default function DocSidebar(props) {
  const windowSize = useWindowSize();
  // Desktop sidebar visible on hydration: need SSR rendering
  const shouldRenderSidebarDesktop =
    windowSize === "desktop" || windowSize === "ssr";
  // Mobile sidebar not visible on hydration: can avoid SSR rendering
  const shouldRenderSidebarMobile = windowSize === "mobile";
  return (
    <>
      {shouldRenderSidebarDesktop && <DocSidebarDesktop {...props} />}
      {shouldRenderSidebarMobile && <DocSidebarMobile {...props} />}
    </>
  );
}
