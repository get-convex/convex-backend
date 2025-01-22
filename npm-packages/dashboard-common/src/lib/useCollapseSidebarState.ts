import { useCallback } from "react";
import { useWindowSize } from "react-use";
import { useGlobalLocalStorage } from "./useGlobalLocalStorage";

export function useCollapseSidebarState() {
  const { width } = useWindowSize();
  const [collapsedBig, setCollapsedBig] = useGlobalLocalStorage(
    "collapseSidebar",
    false,
  );
  const [collapsedSmall, setCollapsedSmall] = useGlobalLocalStorage(
    "collapseSidebarSmallWidth",
    true,
  );

  const setBig = useCallback(
    (collapsed: boolean) => {
      setCollapsedBig(collapsed);
      if (collapsed && !collapsedSmall) {
        setCollapsedSmall(true);
      }
    },
    [collapsedSmall, setCollapsedBig, setCollapsedSmall],
  );

  const setSmall = useCallback(
    (collapsed: boolean) => {
      setCollapsedSmall(collapsed);
      if (!collapsed && collapsedBig) {
        setCollapsedBig(false);
      }
    },
    [collapsedBig, setCollapsedBig, setCollapsedSmall],
  );

  return width < 1024
    ? ([collapsedSmall, setSmall] as const)
    : ([collapsedBig, setBig] as const);
}
