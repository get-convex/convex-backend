import ExecutionEnvironment from "@docusaurus/ExecutionEnvironment";
import { logEvent } from "convex-analytics";

export default (function () {
  if (!ExecutionEnvironment.canUseDOM) {
    return null;
  }

  const currUrl = new URL(location);
  const params = currUrl.searchParams;
  const cookieValue = params.get("t");
  if (cookieValue) {
    const isDev = currUrl.hostname.includes("localhost");
    const domain = isDev ? currUrl.hostname : "convex.dev";
    document.cookie =
      "cvx-t=" +
      cookieValue +
      ";path=/;domain=" +
      domain +
      // Max expiration date
      ";expires=Tue, 19 Jan 2038 04:14:07 GMT" +
      (isDev ? "" : ";secure");
  }

  let scrollLogged = false;

  window.addEventListener("scroll", () => {
    if (window.scrollY > 300 && !scrollLogged) {
      scrollLogged = true;
      logEvent("scroll doc", { path: location.pathname });
    }
  });

  return {
    onRouteUpdate({ location }) {
      // Per https://github.com/facebook/docusaurus/issues/3399#issuecomment-866642682
      // onRouteUpdate is called a bit early so we want to set a timeout to
      // ensure it has the right data
      window.setTimeout(() => {
        logEvent("view doc", { path: location.pathname });
        scrollLogged = false;
      }, 0);
    },
  };
})();
