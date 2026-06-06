import { processDeploymentEvents } from "@common/lib/useDeploymentAuditLog";
import type { Doc } from "system-udfs/convex/_generated/dataModel";

function deploymentEvent(
  action: string,
  metadata: Record<string, unknown>,
  memberId: bigint | null = BigInt(7),
): Doc<"_deployment_audit_log"> {
  return {
    _id: "" as Doc<"_deployment_audit_log">["_id"],
    _creationTime: Date.parse("2026-06-07T12:00:00Z"),
    action,
    member_id: memberId,
    metadata,
  } as Doc<"_deployment_audit_log">;
}

describe("processDeploymentEvents", () => {
  test("keeps self-hosted admin key and periodic backup audit events", () => {
    const events = [
      deploymentEvent("admin_key_created", { id: "key1", name: "CI" }),
      deploymentEvent("admin_key_adopted", { id: "key2", name: "Bootstrap" }),
      deploymentEvent("admin_key_revoked", { id: "key1" }),
      deploymentEvent("admin_key_renamed", { id: "key2", new_name: "Ops" }),
      deploymentEvent("periodic_backup_configured", {
        cronspec: "0 3 * * *",
        include_storage: true,
      }),
      deploymentEvent("periodic_backup_disabled", {}),
      deploymentEvent("periodic_backup_triggered", { export_id: "export1" }),
    ];

    const processed = processDeploymentEvents(events, [
      { id: 7, email: "admin@example.com" },
    ]);

    expect(processed.map((event) => event.action)).toEqual(
      events.map((event) => event.action),
    );
    expect(
      processed.every((event) => event.memberName === "admin@example.com"),
    ).toBe(true);
  });

  test("labels system-authored self-hosted audit events as Convex", () => {
    const [processed] = processDeploymentEvents(
      [deploymentEvent("periodic_backup_disabled", {}, null)],
      [],
    );

    expect(processed.memberName).toBe("Convex");
  });
});
