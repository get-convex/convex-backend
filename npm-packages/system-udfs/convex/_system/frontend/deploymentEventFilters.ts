export type DeploymentEventPostFilters = {
  authorMemberIds?: readonly bigint[];
  actions?: readonly string[];
};

export function hasDeploymentEventPostFilters(
  filters: DeploymentEventPostFilters,
) {
  return filters.authorMemberIds !== undefined || filters.actions !== undefined;
}
