import type {
  components,
  paths as GeneratedManagementApiPaths,
} from "@convex-dev/platform/managementApi";

/**
 * Response shape of the paginated projects endpoint. Mirrors
 * `PlatformPaginatedProjectsResponse` in
 * `crates_private/big_brain/src/http/projects.rs`.
 */
export type PlatformPaginatedProjectsResponse = {
  items: components["schemas"]["PlatformProjectDetails"][];
  pagination: components["schemas"]["PaginationMetadata"];
};

/**
 * `GET /teams/{team_id}/projects` is served by big-brain's platform router but
 * intentionally kept out of the published management OpenAPI spec (registered
 * with `route` instead of the OpenAPI router in `platform_router`), so it is
 * absent from `@convex-dev/platform`'s generated types. We describe it here so
 * the dashboard's typed management-API hooks can call it without adding it to
 * the public SDK surface.
 *
 * Keep this in sync with `platform_get_projects_for_team` in
 * `crates_private/big_brain/src/http/projects.rs`.
 */
interface HiddenManagementApiPaths {
  "/teams/{team_id}/projects": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: {
      parameters: {
        query?: {
          /** Cursor for pagination (opaque string from a previous response). */
          cursor?: string;
          /** Maximum number of projects to return (1-100, defaults to 100). */
          limit?: number;
          /** Search query to filter projects by name or slug (case-insensitive). */
          q?: string;
        };
        header?: never;
        path: {
          /** Team ID */
          team_id: string;
        };
        cookie?: never;
      };
      requestBody?: never;
      responses: {
        200: {
          headers: {
            [name: string]: unknown;
          };
          content: {
            "application/json": PlatformPaginatedProjectsResponse;
          };
        };
      };
    };
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
}

/**
 * The published management API paths plus the dashboard-only hidden endpoints
 * the dashboard relies on. Use this as the source of truth for the dashboard's
 * management-API client and hooks.
 */
export type paths = GeneratedManagementApiPaths & HiddenManagementApiPaths;
