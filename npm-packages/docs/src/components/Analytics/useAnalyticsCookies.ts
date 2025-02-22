import useIsBrowser from "@docusaurus/useIsBrowser";
import { useCookies } from "react-cookie";

// We share a single cookie across the website, Docs, Stack, etc. to avoid
// having users reject/accept the cookie banner repeatedly. When making changes,
// ensure that those projects are updated as well.
const COOKIE_NAME = "allowsCookies";

export function useAnalyticsCookies() {
  const [cookies, setCookie] = useCookies([COOKIE_NAME]);
  const isBrowser = useIsBrowser();

  // An undefined value indicates that the cookie is not present, so the user
  // has not yet accepted or rejected the cookie banner.
  const allowsCookies = cookies[COOKIE_NAME];

  const setAllowsCookies = (value: boolean) => {
    // Return early if we're running on the server.
    if (!isBrowser) {
      return;
    }

    const hostname = window.location.hostname;
    const isConvex =
      hostname === "convex.dev" || hostname.endsWith(".convex.dev");

    setCookie(COOKIE_NAME, value, {
      domain: isConvex ? ".convex.dev" : undefined,
      path: "/",
      maxAge: 34560000,
      // Ensures cookie is only sent over HTTPS.
      secure: hostname !== "localhost",
    });
  };

  return { allowsCookies, setAllowsCookies };
}
