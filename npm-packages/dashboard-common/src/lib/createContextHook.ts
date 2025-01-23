import React, { useContext } from "react";

type ReturnType<T> = [React.Context<T>, () => T];

/**
 * Creates a new context + hook to use it easily.
 * @returns
 */
export function createContextHook<T>({
  name,
  defaultValue,
}: {
  name: string;
  defaultValue?: any;
}): ReturnType<T> {
  const NewContext = React.createContext<T>(defaultValue);

  function useContextHook() {
    const context = useContext<T>(NewContext);

    if (!context) {
      throw new Error(`${name} cannot be used outside it's Provider.`);
    }

    return context;
  }

  return [NewContext, useContextHook];
}
