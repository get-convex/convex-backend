import { Button } from "@ui/Button";
import { Menu, MenuItem } from "@ui/Menu";
import { PlusIcon, TrashIcon, Pencil1Icon } from "@radix-ui/react-icons";
import { Session } from "lib/useMultiSession";
import { useState } from "react";
import { TextInput } from "@ui/TextInput";
import { Sheet } from "@ui/Sheet";

interface SessionSwitcherProps {
  sessions: Session[];
  activeSessionId: string | null;
  onSwitch: (sessionId: string) => void;
  onRemove: (sessionId: string) => void;
  onUpdateName: (sessionId: string, newName: string) => void;
  onAddNew: () => void;
}

function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor((Date.now() - timestamp) / 1000);
  
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function SessionItem({
  session,
  isActive,
  onSwitch,
  onRemove,
  onUpdateName,
}: {
  session: Session;
  isActive: boolean;
  onSwitch: () => void;
  onRemove: () => void;
  onUpdateName: (newName: string) => void;
}) {
  const [isEditing, setIsEditing] = useState(false);
  const [editedName, setEditedName] = useState(session.name);

  const handleSaveName = () => {
    if (editedName.trim() && editedName !== session.name) {
      onUpdateName(editedName.trim());
    }
    setIsEditing(false);
  };

  const truncateUrl = (url: string) => {
    try {
      const parsed = new URL(url);
      return parsed.hostname + (parsed.port ? `:${parsed.port}` : "");
    } catch {
      return url.length > 30 ? url.substring(0, 30) + "..." : url;
    }
  };

  if (isEditing) {
    return (
      <div className="flex items-center gap-2 px-3 py-2">
        <TextInput
          id={`session-name-${session.id}`}
          value={editedName}
          onChange={(e) => setEditedName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleSaveName();
            if (e.key === "Escape") {
              setEditedName(session.name);
              setIsEditing(false);
            }
          }}
          onBlur={handleSaveName}
          autoFocus
          className="text-sm"
        />
      </div>
    );
  }

  return (
    <div
      className={`group flex items-center justify-between gap-2 px-3 py-2 hover:bg-background-tertiary ${
        isActive ? "bg-background-tertiary" : ""
      }`}
    >
      <button
        onClick={onSwitch}
        className="flex min-w-0 flex-1 flex-col items-start text-left"
      >
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">
            {session.name}
            {isActive && (
              <span className="ml-2 text-xs text-content-secondary">
                (active)
              </span>
            )}
          </span>
        </div>
        <span className="truncate text-xs text-content-secondary">
          {truncateUrl(session.deploymentUrl)}
        </span>
        <span className="text-xs text-content-tertiary">
          {formatTimeAgo(session.lastAccessed)}
        </span>
      </button>
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100">
        <Button
          variant="unstyled"
          size="xs"
          aria-label="Rename session"
          onClick={(e) => {
            e.stopPropagation();
            setIsEditing(true);
          }}
          icon={<Pencil1Icon />}
          className="h-6 w-6 p-1 text-content-secondary hover:text-content-primary"
        />
        <Button
          variant="unstyled"
          size="xs"
          aria-label="Delete session"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
          icon={<TrashIcon />}
          className="h-6 w-6 p-1 text-content-secondary hover:text-red-500"
        />
      </div>
    </div>
  );
}

export function SessionSwitcher({
  sessions,
  activeSessionId,
  onSwitch,
  onRemove,
  onUpdateName,
  onAddNew,
}: SessionSwitcherProps) {
  // Sort sessions by last accessed (most recent first)
  const sortedSessions = [...sessions].sort(
    (a, b) => b.lastAccessed - a.lastAccessed
  );

  return (
    <Menu
      buttonProps={{
        children: (
          <div className="flex items-center gap-2">
            <span className="text-sm">Sessions</span>
            <span className="rounded bg-background-tertiary px-1.5 py-0.5 text-xs">
              {sessions.length}
            </span>
          </div>
        ),
        variant: "neutral",
        size: "xs",
        "aria-label": "Switch session",
      }}
      placement="bottom-end"
    >
      <div className="max-h-96 w-80 overflow-y-auto">
        {sortedSessions.length === 0 ? (
          <div className="px-3 py-4 text-center text-sm text-content-secondary">
            No saved sessions
          </div>
        ) : (
          <div className="divide-y">
            {sortedSessions.map((session) => (
              <SessionItem
                key={session.id}
                session={session}
                isActive={session.id === activeSessionId}
                onSwitch={() => onSwitch(session.id)}
                onRemove={() => onRemove(session.id)}
                onUpdateName={(newName) => onUpdateName(session.id, newName)}
              />
            ))}
          </div>
        )}
        <div className="border-t p-2">
          <Button
            variant="neutral"
            size="xs"
            icon={<PlusIcon />}
            onClick={onAddNew}
            className="w-full"
          >
            Add New Session
          </Button>
        </div>
      </div>
    </Menu>
  );
}
