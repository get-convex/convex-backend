import startCase from "lodash/startCase";
import type { TeamMemberCustomRole } from "generatedApi";

type RoleDisplayProps = {
  role: string;
  customRoles?: TeamMemberCustomRole[] | null;
};

/**
 * Renders a team member's role: built-in roles (Admin/Developer) as plain
 * text and `custom` as a row of tag chips with each attached role's name.
 * Used by both the team-members list and the pending-invites list so they
 * stay visually consistent.
 */
export function RoleDisplay({ role, customRoles }: RoleDisplayProps) {
  if (role !== "custom") {
    return (
      <div className="text-sm text-content-primary">{startCase(role)}</div>
    );
  }
  const list = customRoles ?? [];
  return (
    <div className="flex flex-wrap items-center gap-1">
      {list.length === 0 ? (
        <div className="text-sm text-content-primary">Custom</div>
      ) : (
        list.map(({ id, name }) => (
          <span
            key={id}
            className="rounded-sm border px-1.5 py-1 text-xs text-content-primary"
          >
            {name}
          </span>
        ))
      )}
    </div>
  );
}
