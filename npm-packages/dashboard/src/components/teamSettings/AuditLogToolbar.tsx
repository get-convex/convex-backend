import { Combobox, Option } from "dashboard-common/elements/Combobox";
import { DateRangePicker } from "dashboard-common/elements/DateRangePicker";
import startCase from "lodash/startCase";
import { endOfToday, startOfDay } from "date-fns";
import sortBy from "lodash/sortBy";
import type { DateRange } from "react-day-picker";
import { AuditLogAction, MemberResponse } from "generatedApi";

// TODO: Replace with a generated type once we can generate enum
// values
const AUDIT_LOG_ACTIONS = [
  "joinTeam",
  "createTeam",
  "updateTeam",
  "deleteTeam",
  "createProject",
  "updateProject",
  "deleteProject",
  "createProjectEnvironmentVariable",
  "updateProjectEnvironmentVariable",
  "deleteProjectEnvironmentVariable",
  "createDeployment",
  "deleteDeployment",
  "inviteMember",
  "cancelMemberInvitation",
  "updateMemberRole",
  "updateMemberProjectRole",
  "removeMember",
  "updatePaymentMethod",
  "updateBillingContact",
  "updateBillingAddress",
  "createSubscription",
  "cancelSubscription",
  "resumeSubscription",
  "createCustomDomain",
  "deleteCustomDomain",
  "createTeamAccessToken",
  "updateTeamAccessToken",
  "deleteTeamAccessToken",
  "startManualCloudBackup",
  "deleteCloudBackup",
  "restoreFromCloudBackup",
  "configurePeriodicBackup",
  "disablePeriodicBackup",
  "disableTeamExceedingSpendingLimits",
  "setSpendingLimit",
] as const;

const actionOptions: Option<AuditLogAction | "all_actions">[] = [
  { label: "All actions", value: "all_actions" },
  ...AUDIT_LOG_ACTIONS.map((action) => ({
    label: startCase(action),
    value: action,
  })),
];

export function AuditLogToolbar({
  selectedMember,
  setSelectedMember,
  selectedAction,
  setSelectedAction,
  selectedStartDay,
  selectedEndDay,
  members,
  setDate,
}: {
  selectedMember: string;
  setSelectedMember: (member: string) => void;
  selectedAction: AuditLogAction | "all_actions";
  setSelectedAction: (action: AuditLogAction | "all_actions") => void;
  selectedStartDay: Date;
  selectedEndDay: Date;
  members: MemberResponse[];
  setDate: (date: DateRange) => void;
}) {
  const minStartDate = startOfDay(new Date("2024-06-05"));
  const maxEndDate = endOfToday();

  const startDate =
    selectedStartDay > minStartDate ? selectedStartDay : minStartDate;

  return (
    <div className="flex flex-wrap gap-2">
      <DateRangePicker
        minDate={minStartDate}
        maxDate={maxEndDate}
        date={{ from: startDate, to: selectedEndDay }}
        setDate={setDate}
      />
      <Combobox
        options={[
          { label: "All members", value: "all_members" },
          ...sortBy(
            members.map((m) => ({
              label: m.name ? `${m.name} (${m.email})` : m.email,
              value: m.id.toString(),
            })),
            [(option) => option.label.toLowerCase()],
          ),
        ]}
        allowCustomValue
        selectedOption={selectedMember}
        setSelectedOption={(o) =>
          setSelectedMember(o === null ? "all_members" : o)
        }
        label="Members"
      />
      <Combobox
        options={actionOptions}
        selectedOption={selectedAction}
        setSelectedOption={(o) =>
          setSelectedAction(o === null ? "all_actions" : o)
        }
        label="Actions"
      />
    </div>
  );
}
