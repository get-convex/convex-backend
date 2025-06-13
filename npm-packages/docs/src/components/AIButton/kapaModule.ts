import { useEffect } from "react";
import { useAnalyticsCookies } from "../Analytics/useAnalyticsCookies";

const SCRIPT_ID = "kapa-widget-script";

function createKapaWidgetScript() {
  if (document.getElementById(SCRIPT_ID)) {
    return;
  }

  const script = document.createElement("script");
  script.id = SCRIPT_ID;
  script.src = "https://widget.kapa.ai/kapa-widget.bundle.js";
  script.async = true;
  script.setAttribute(
    "data-website-id",
    "a20c0988-f33e-452b-9174-5045a58b965d",
  );
  script.setAttribute("data-project-name", "Convex");
  script.setAttribute("data-project-color", "#141414");
  script.setAttribute(
    "data-project-logo",
    "https://img.stackshare.io/service/41143/default_f1d33b63d360437ba28c8ac981dd68d7d2478b22.png",
  );
  script.setAttribute("data-button-hide", "true");
  script.setAttribute("data-modal-override-open-class", "js-launch-kapa-ai");
  script.setAttribute("data-user-analytics-fingerprint-enabled", "true");
  script.setAttribute("data-render-on-load", "true");
  script.setAttribute("data-user-analytics-cookie-enabled", "false");

  document.body.appendChild(script);
}

export function useKapaWidget() {
  const { allowsCookies } = useAnalyticsCookies();

  // Default to false if the cookie is not set.
  const analyticsEnabled = allowsCookies ?? false;

  // Create the Kapa widget script on mount.
  useEffect(() => {
    createKapaWidgetScript();
  }, []);

  // Update the widget's data attribude if cookie consent changes. A page
  // refresh may still be necessary in order for Kapa to pick up the change.
  // See: https://convexdev.slack.com/archives/C06P55VN8Q4/p1749147459001719
  useEffect(() => {
    const script = document.getElementById(SCRIPT_ID);
    if (script) {
      script.setAttribute(
        "data-user-analytics-cookie-enabled",
        analyticsEnabled.toString(),
      );
    }
  }, [analyticsEnabled]);
}
