import { Loading } from "dashboard-common/elements/Loading";
import { toast } from "dashboard-common/lib/utils";
import { AuditLog } from "components/teamSettings/AuditLog";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { TeamSettingsLayout } from "layouts/TeamSettingsLayout";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useRouter } from "next/router";

export { getServerSideProps } from "lib/ssr";

function AuditLogPage() {
  const team = useCurrentTeam();
  const auditLogsEnabled = useTeamEntitlements(team?.id)?.auditLogsEnabled;
  const router = useRouter();

  if (auditLogsEnabled === undefined) {
    return <Loading />;
  }
  if (!auditLogsEnabled) {
    toast("info", "The audit log is only available on paid plans.", "upsell");
    void router.push(`/t/${router.query.team}/settings/billing`);
    return null;
  }

  return (
    <TeamSettingsLayout
      page="audit-log"
      Component={AuditLog}
      title="Audit Log"
    />
  );
}

export default withAuthenticatedPage(AuditLogPage);
