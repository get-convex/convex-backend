import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import { useCookies } from "react-cookie";

// Ensure this is the same for the website, dashboard, Stack, etc. as they all
// share the same cookie. This allows a user to consent once across all of the
// subdomains, rather than seeing the banner repeatedly.
const COOKIE_NAME = "allowsCookies";

export function useAnalyticsCookies() {
  const [cookies, setCookie] = useCookies([COOKIE_NAME]);
  const { siteConfig } = useDocusaurusContext();

  const isProduction = siteConfig.customFields.NODE_ENV === "production";

  // An undefined value indicates that the cookie is not present, so the user
  // has not yet accepted or rejected the cookie banner.
  const allowsCookies = cookies[COOKIE_NAME];

  const setAllowsCookies = (value: boolean) => {
    setCookie(COOKIE_NAME, value, {
      domain: isProduction ? ".convex.dev" : undefined,
      path: "/",
      maxAge: 34560000,
    });
  };

  return { allowsCookies, setAllowsCookies };
}
