import type { AdminKey } from "../../hooks/useAdminKeys";
import { RenameAdminKeyButton } from "./RenameAdminKeyButton";
import { RevokeAdminKeyButton } from "./RevokeAdminKeyModal";

export function AdminKeysList({
  keys,
  onRevoke,
  onRename,
}: {
  keys: AdminKey[];
  onRevoke: (id: string, isCurrent: boolean) => Promise<void>;
  onRename: (id: string, name: string) => Promise<void>;
}) {
  return (
    <table className="w-full text-sm">
      <thead>
        <tr className="text-left text-content-secondary">
          <th className="py-2">Name</th>
          <th className="py-2">Created</th>
          <th className="py-2">Status</th>
          <th className="py-2 text-right">Actions</th>
        </tr>
      </thead>
      <tbody>
        {keys.map((k) => (
          <tr key={k.id} className="border-t">
            <td className="py-2">
              {k.name}
              {k.isCurrent && (
                <span className="ml-2 rounded bg-background-tertiary px-1.5 py-0.5 text-xs">
                  This key
                </span>
              )}
            </td>
            <td className="py-2">
              {new Date(k.creationTime).toLocaleDateString()}
            </td>
            <td className="py-2">{k.revokedTime ? "Revoked" : "Active"}</td>
            <td className="py-2 text-right">
              {!k.revokedTime && (
                <div className="flex justify-end gap-1">
                  <RenameAdminKeyButton
                    id={k.id}
                    name={k.name}
                    onRename={onRename}
                  />
                  <RevokeAdminKeyButton
                    id={k.id}
                    isCurrent={k.isCurrent}
                    onRevoke={onRevoke}
                  />
                </div>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
