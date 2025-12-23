import { useCallback, useEffect, useState } from "react";

export interface Session {
  id: string;
  name: string;
  deploymentUrl: string;
  adminKey: string;
  deploymentName: string;
  lastAccessed: number;
  createdAt: number;
}

interface SessionsData {
  sessions: Session[];
  activeSessionId: string | null;
}

const SESSIONS_STORAGE_KEY = "convex-dashboard-sessions";

function generateSessionId(): string {
  return `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function getSessionsFromStorage(): SessionsData {
  try {
    const stored = localStorage.getItem(SESSIONS_STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    console.error("Failed to parse sessions from localStorage:", e);
  }
  return { sessions: [], activeSessionId: null };
}

function saveSessionsToStorage(data: SessionsData): void {
  try {
    localStorage.setItem(SESSIONS_STORAGE_KEY, JSON.stringify(data));
  } catch (e) {
    console.error("Failed to save sessions to localStorage:", e);
  }
}

export function useMultiSession() {
  const [sessionsData, setSessionsData] = useState<SessionsData>(
    getSessionsFromStorage()
  );

  // Sync with localStorage on mount and when other tabs make changes
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === SESSIONS_STORAGE_KEY && e.newValue) {
        try {
          setSessionsData(JSON.parse(e.newValue));
        } catch (err) {
          console.error("Failed to sync sessions:", err);
        }
      }
    };

    window.addEventListener("storage", handleStorageChange);
    return () => window.removeEventListener("storage", handleStorageChange);
  }, []);

  const activeSession = sessionsData.sessions.find(
    (s) => s.id === sessionsData.activeSessionId
  );

  const addSession = useCallback(
    (
      deploymentUrl: string,
      adminKey: string,
      deploymentName: string,
      customName?: string
    ): Session => {
      const now = Date.now();
      
      // Check if session with same URL already exists
      const existingSession = sessionsData.sessions.find(
        (s) => s.deploymentUrl === deploymentUrl
      );

      if (existingSession) {
        // Update existing session
        const updatedSession: Session = {
          ...existingSession,
          adminKey,
          deploymentName,
          name: customName || existingSession.name,
          lastAccessed: now,
        };

        const newData: SessionsData = {
          sessions: sessionsData.sessions.map((s) =>
            s.id === existingSession.id ? updatedSession : s
          ),
          activeSessionId: existingSession.id,
        };

        setSessionsData(newData);
        saveSessionsToStorage(newData);
        return updatedSession;
      }

      // Create new session
      const newSession: Session = {
        id: generateSessionId(),
        name: customName || `Deployment ${sessionsData.sessions.length + 1}`,
        deploymentUrl,
        adminKey,
        deploymentName,
        lastAccessed: now,
        createdAt: now,
      };

      const newData: SessionsData = {
        sessions: [...sessionsData.sessions, newSession],
        activeSessionId: newSession.id,
      };

      setSessionsData(newData);
      saveSessionsToStorage(newData);
      return newSession;
    },
    [sessionsData]
  );

  const switchSession = useCallback(
    (sessionId: string): Session | null => {
      const session = sessionsData.sessions.find((s) => s.id === sessionId);
      if (!session) {
        return null;
      }

      const updatedSession: Session = {
        ...session,
        lastAccessed: Date.now(),
      };

      const newData: SessionsData = {
        sessions: sessionsData.sessions.map((s) =>
          s.id === sessionId ? updatedSession : s
        ),
        activeSessionId: sessionId,
      };

      setSessionsData(newData);
      saveSessionsToStorage(newData);
      return updatedSession;
    },
    [sessionsData]
  );

  const removeSession = useCallback(
    (sessionId: string): void => {
      const newSessions = sessionsData.sessions.filter(
        (s) => s.id !== sessionId
      );
      
      let newActiveSessionId = sessionsData.activeSessionId;
      
      // If we're removing the active session, switch to the most recent one
      if (sessionsData.activeSessionId === sessionId) {
        const sortedByRecent = [...newSessions].sort(
          (a, b) => b.lastAccessed - a.lastAccessed
        );
        newActiveSessionId = sortedByRecent[0]?.id || null;
      }

      const newData: SessionsData = {
        sessions: newSessions,
        activeSessionId: newActiveSessionId,
      };

      setSessionsData(newData);
      saveSessionsToStorage(newData);
    },
    [sessionsData]
  );

  const updateSessionName = useCallback(
    (sessionId: string, newName: string): void => {
      const newData: SessionsData = {
        ...sessionsData,
        sessions: sessionsData.sessions.map((s) =>
          s.id === sessionId ? { ...s, name: newName } : s
        ),
      };

      setSessionsData(newData);
      saveSessionsToStorage(newData);
    },
    [sessionsData]
  );

  const clearAllSessions = useCallback((): void => {
    const newData: SessionsData = {
      sessions: [],
      activeSessionId: null,
    };

    setSessionsData(newData);
    saveSessionsToStorage(newData);
  }, []);

  return {
    sessions: sessionsData.sessions,
    activeSession,
    activeSessionId: sessionsData.activeSessionId,
    addSession,
    switchSession,
    removeSession,
    updateSessionName,
    clearAllSessions,
  };
}
