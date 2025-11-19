import React, { ReactNode } from "react";
import { DecoratorHelpers } from "@storybook/addon-themes";
import type { DecoratorFunction } from "storybook/internal/types";
import { ThemeProvider } from "next-themes";
import { ReactRenderer } from "@storybook/nextjs";
const { initializeThemeState, pluckThemeFromContext } = DecoratorHelpers;

const themeDecorator: (args: {
  themes: Record<string, string>;
  defaultTheme: string;
}) => DecoratorFunction<ReactRenderer> = ({ themes, defaultTheme }) => {
  initializeThemeState(Object.keys(themes), defaultTheme);

  return (storyFn, context) => {
    const selectedTheme = pluckThemeFromContext(context);
    const { themeOverride } = context.parameters.themes ?? {};

    const selected = themeOverride || selectedTheme || defaultTheme;

    return (
      <ThemeProvider attribute="class" forcedTheme={selected}>
        {storyFn() as ReactNode}
      </ThemeProvider>
    );
  };
};

export default themeDecorator;
