import { useCookies } from "react-cookie";
import useIsBrowser from "@docusaurus/useIsBrowser";

// Ensure this is the same for the website, dashboard, Stack, etc. as they all
// share the same cookie. This allows a user to consent once across all of the
// subdomains, rather than seeing the banner repeatedly.
const COOKIE_NAME = "allowsCookies";

export function useAnalyticsCookies() {
  const [cookies, setCookie] = useCookies([COOKIE_NAME]);
  const isBrowser = useIsBrowser();

  // An undefined value indicates that the cookie is not present, so the user
  // has not yet accepted or rejected the cookie banner.
  const allowsCookies = cookies[COOKIE_NAME];

  const setAllowsCookies = (value: boolean) => {
    if (isBrowser) {
      setCookie(COOKIE_NAME, value, {
        domain: `.${window.location.hostname}`,
        path: "/",
        maxAge: 34560000,
      });
    }
  };

  return { allowsCookies, setAllowsCookies };
}
