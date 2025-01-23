import { Team, InvitationResponse, TeamMemberResponse } from "generatedApi";

import { Sheet, LoadingTransition, TextInput } from "dashboard-common";
import { useState } from "react";
import { useProjects } from "api/projects";
import {
  useUpdateTeamMemberRole,
  useIsCurrentMemberTeamAdmin,
  useProjectRoles,
  useUpdateProjectRoles,
} from "api/roles";
import { useRemoveTeamMember } from "api/teams";
import { useProfile } from "api/profile";
import { useCancelInvite, useCreateInvite } from "api/invitations";
import sortBy from "lodash/sortBy";
import { TeamMemberInviteListItem } from "./TeamMemberInviteListItem";
import { TeamMemberListItem } from "./TeamMemberListItem";
import { TeamMemberListSkeleton } from "./TeamMemberListSkeleton";

type TeamMemberListProps = {
  team: Team;
  members?: TeamMemberResponse[];
  invites?: InvitationResponse[];
};

export function TeamMemberList({
  team,
  members,
  invites,
}: TeamMemberListProps) {
  const [memberSearch, setMemberSearch] = useState("");
  const [inviteSearch, setInviteSearch] = useState("");
  const filteredMembers = members?.filter(
    (member) =>
      member.name?.toLowerCase().includes(memberSearch.toLowerCase()) ||
      member.email.toLowerCase().includes(memberSearch.toLowerCase()),
  );
  const profile = useProfile();
  const me = filteredMembers?.find((member) => member.id === profile?.id);
  const filteredMembersWithoutMe = filteredMembers?.filter(
    (member) => member.id !== profile?.id,
  );

  const filteredInvites = invites?.filter((invite) =>
    invite.email.toLowerCase().includes(inviteSearch.toLowerCase()),
  );

  const onChangeRole = useUpdateTeamMemberRole(team.id);
  const onRemoveMember = useRemoveTeamMember(team.id);
  const onCreateInvite = useCreateInvite(team.id);
  const onCancelInvite = useCancelInvite(team.id);

  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();

  const { projectRoles } = useProjectRoles();
  const projects = useProjects(team.id);

  const updateProjectRoles = useUpdateProjectRoles(team.id);

  return (
    <>
      <Sheet className="min-w-[20rem]">
        <div className="mb-4 flex w-full items-center justify-between gap-2">
          <h3 className="grow">Team Members</h3>
          <div className="w-[12rem]">
            <TextInput
              type="search"
              id="memberSearch"
              value={memberSearch}
              onChange={(e) => setMemberSearch(e.target.value)}
              placeholder="Search members"
            />
          </div>
        </div>
        <LoadingTransition>
          {profile && members && projectRoles && projects && (
            <div className="flex w-full flex-col">
              {/* Always show self at the top */}
              {me && (
                <TeamMemberListItem
                  team={team}
                  projects={projects}
                  member={me}
                  members={members}
                  canChangeRole={false}
                  myProfile={profile}
                  onChangeRole={onChangeRole}
                  onRemoveMember={onRemoveMember}
                  hasAdminPermissions={hasAdminPermissions}
                  projectRoles={projectRoles?.filter(
                    (role) => role.memberId === me.id,
                  )}
                  onUpdateProjectRoles={updateProjectRoles}
                />
              )}
              {!filteredMembersWithoutMe ? (
                <TeamMemberListSkeleton />
              ) : filteredMembers && filteredMembers.length === 0 ? (
                <div className="my-4 flex w-full justify-center text-content-secondary">
                  No members match your search.
                </div>
              ) : (
                sortBy(filteredMembersWithoutMe, (member) =>
                  (member.name || member.email).toLocaleLowerCase(),
                ).map((member) => (
                  <TeamMemberListItem
                    key={`member${member.id}`}
                    team={team}
                    projects={projects}
                    member={member}
                    members={members}
                    canChangeRole
                    myProfile={profile}
                    onChangeRole={onChangeRole}
                    onRemoveMember={onRemoveMember}
                    hasAdminPermissions={hasAdminPermissions}
                    projectRoles={projectRoles?.filter(
                      (role) => role.memberId === member.id,
                    )}
                    onUpdateProjectRoles={updateProjectRoles}
                  />
                ))
              )}
            </div>
          )}
        </LoadingTransition>
      </Sheet>
      {invites && invites.length > 0 && (
        <Sheet>
          <div className="mb-4 flex w-full items-center justify-between gap-2">
            <h3 className="grow">Pending Invitations</h3>
            <div className="w-[12rem]">
              <TextInput
                type="search"
                id="inviteSearch"
                value={inviteSearch}
                onChange={(e) => setInviteSearch(e.target.value)}
                placeholder="Search invitations"
              />
            </div>
          </div>
          <div className="flex flex-col">
            {!filteredInvites ? (
              <TeamMemberListSkeleton />
            ) : filteredInvites.length === 0 ? (
              <div className="my-4 flex w-full justify-center text-content-secondary">
                No invites match your search.
              </div>
            ) : (
              filteredInvites.map((invite, idx) => (
                <TeamMemberInviteListItem
                  key={`invite${idx}`}
                  invite={invite}
                  hasAdminPermissions={hasAdminPermissions}
                  onCreateInvite={onCreateInvite}
                  onCancelInvite={onCancelInvite}
                />
              ))
            )}
          </div>
        </Sheet>
      )}
    </>
  );
}
