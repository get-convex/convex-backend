import { createGlobalState } from "react-use";

export const useAccessToken = createGlobalState<string>();

export const useInitialData = createGlobalState<Record<string, any>>();
