import { useEffect, useState } from "react";
import { isOnline } from "./hooks/usePresence";

const UPDATE_MS = 1000;

export default ({ othersPresence }) => {
  const [, setNow] = useState(Date.now());
  useEffect(() => {
    const intervalId = setInterval(() => setNow(Date.now()), UPDATE_MS);
    return () => clearInterval(intervalId);
  }, [setNow]);
  return (
    <div className="facepile">
      {othersPresence
        .slice(0, 5)
        .map((presence) => ({
          ...presence,
          online: isOnline(presence),
        }))
        .sort((presence1, presence2) =>
          presence1.online === presence2.online
            ? presence1.created - presence2.created
            : Number(presence1.online) - Number(presence2.online),
        )
        .map((presence, i) => (
          <span
            className={"face" + (presence.online ? "" : " grayscale")}
            key={presence.created}
            style={{ marginLeft: i ? -8 : 0 }}
            title={
              presence.data.name +
              (presence.online
                ? ": Online"
                : ": Last seen " + new Date(presence.updated).toDateString())
            }
          >
            {presence.data.emoji}
          </span>
        ))}
    </div>
  );
};
