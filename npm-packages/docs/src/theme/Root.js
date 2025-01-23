import { logEvent } from "convex-analytics";
import React, { useEffect, useState, createContext, useContext } from "react";

function Root({ children }) {
  useEffect(() => {
    logEvent("view doc load", { path: location.pathname });
  }, []);

  // Scroll the active sidebar item into view in case
  // it's below fold.
  useEffect(() => {
    document.querySelectorAll(".menu__link--active").forEach((activeLink) => {
      // scrollIntoViewIfNeeded works great so use
      // it by default (Chrome, Safari)
      if (activeLink.scrollIntoViewIfNeeded) {
        activeLink.scrollIntoViewIfNeeded?.();
      } else {
        // If we used block: "center" it would
        // shift the whole page after page load
        activeLink.scrollIntoView({
          behavior: "instant",
          block: "nearest",
        });
      }
    });
  }, []);

  const [lang, setLang] = useState("TS");

  return (
    <DialectContext.Provider value={{ lang, setLang }}>
      {children}
    </DialectContext.Provider>
  );
}

const DialectContext = createContext();

export function useSelectedDialect() {
  return useContext(DialectContext).lang;
}

export function useSetDialect() {
  return useContext(DialectContext).setLang;
}

export default Root;
