import { usePostHog as usePostHogOriginal } from "posthog-js/react";

// Map of event names to their properties (use `never` if no properties).
export type PostHogEventMap = {
  created_project: never;
  deleted_projects: never;
  created_table: never;
  add_documents: {
    count: number;
  };
  added_environment_variables: {
    count: number;
  };
  uploaded_files: {
    count: number;
  };
  ran_custom_query: never;
  copied_query_result: never;
  upgraded_to_pro: never;
  added_integration: {
    kind: string;
  };
  paused_deployment: never;
  generated_deploy_key: {
    type: string;
  };
  deleted_deploy_key: {
    type: string;
  };
  created_backup: {
    includedStorage: boolean;
  };
};

export type PostHogEvent = keyof PostHogEventMap;

// Type-safe wrapper around PostHog's capture function, only allows capturing
// events with predefined event names and their specific properties.
export function usePostHog() {
  const posthog = usePostHogOriginal();

  // Captures a custom event by name, with properties as required.
  function capture<E extends PostHogEvent>(
    event: E,
    ...args: PostHogEventMap[E] extends never
      ? []
      : [properties: PostHogEventMap[E]]
  ) {
    posthog?.capture(event, args[0]);
  }

  return {
    capture,
    posthog, // Expose the original PostHog instance for advanced use cases.
  };
}
