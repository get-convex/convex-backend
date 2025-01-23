import { version } from "convex";

interface Event {
  event: string;
  args?: object | null;
}

// Log an event to Big Brain.
export function logEvent(eventName: string, props: object | null = null) {
  const data: Event = {
    event: eventName,
  };
  if (props) {
    data.args = props;
  }

  const isDev = window.location.hostname.includes("localhost");

  let apiHost = "https://api.convex.dev";
  if (isDev) {
    apiHost = "http://127.0.0.1:8050";
  }

  const endpoint = `${apiHost}/api/dashboard/event`;
  logEventInner(data, endpoint);
}

// Log an instance-specific event. The event is sent to the instance backend, rather than to Big Brain.
export function logDeploymentEvent(
  eventName: string,
  instanceUrl: string,
  authHeader?: string,
  props: object | null = null,
) {
  const data: Event = {
    event: eventName,
  };
  if (props) {
    data.args = props;
  }
  const isDev = window.location.hostname.includes("localhost");
  let endpoint = `${instanceUrl}/api/event`;
  if (isDev && !instanceUrl.includes("127.0.0.1")) {
    endpoint = `http://127.0.0.1:8000/api/event`;
  }
  logEventInner(data, endpoint, authHeader);
}

function logEventInner(data: Event, endpoint: string, authHeader?: string) {
  fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Convex-Client": `npm-${version}`,
      ...(authHeader ? { Authorization: authHeader } : {}),
    },
    body: JSON.stringify(data),
  })
    .then((response) => {
      if (!response.ok) {
        console.warn("Analytics request failed with response:", response.body);
      }
    })
    .catch((error) => {
      console.warn("Analytics response failed with error:", error);
    });
}
