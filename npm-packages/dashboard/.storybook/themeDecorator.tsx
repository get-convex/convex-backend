import React, { ReactNode } from "react";
import { DecoratorHelpers } from "@storybook/addon-themes";
import type { DecoratorFunction, Renderer } from "@storybook/types";
import { ThemeProvider } from "dashboard-common";
import { ReactRenderer } from "@storybook/react";
const { initializeThemeState, pluckThemeFromContext, useThemeParameters } =
  DecoratorHelpers;

const themeDecorator: <TRenderer extends Renderer = any>(args: {
  themes: Record<string, string>;
  defaultTheme: string;
}) => DecoratorFunction<TRenderer> = ({ themes, defaultTheme }) => {
  initializeThemeState(Object.keys(themes), defaultTheme);

  return (storyFn, context) => {
    const selectedTheme = pluckThemeFromContext(context);
    const { themeOverride } = useThemeParameters();

    const selected = themeOverride || selectedTheme || defaultTheme;

    return (
      <ThemeProvider attribute="class" forcedTheme={selected}>
        {storyFn() as ReactNode}
      </ThemeProvider>
    );
  };
};

export default themeDecorator;
