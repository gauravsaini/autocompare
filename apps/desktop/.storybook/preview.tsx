import type { Preview } from "@storybook/react-vite";
import React, { useEffect } from "react";
import "../src/v2/v2.css";
import "../src/theme/tokens.css";
import "./preview.css";
import { registerBuiltinComparators } from "../src/v2/plugins/builtins";

registerBuiltinComparators();

function StoryThemeWrapper({
  theme,
  layout,
  fullscreen,
  children,
}: {
  readonly theme: "light" | "dark";
  readonly layout: "standard" | "compact";
  readonly fullscreen: boolean;
  readonly children: React.ReactNode;
}): React.JSX.Element {
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.layout = layout;
    document.documentElement.style.setProperty("font-family", "var(--ac-font-ui)");
  }, [theme, layout]);

  return (
    <div
      data-theme={theme}
      data-layout={layout}
      className={
        fullscreen
          ? "h-screen w-screen overflow-hidden bg-[var(--v2-bg)] text-[var(--v2-fg)]"
          : "min-h-full w-full bg-[var(--v2-bg)] text-[var(--v2-fg)]"
      }
    >
      {children}
    </div>
  );
}

const preview: Preview = {
  globalTypes: {
    theme: {
      name: "Theme",
      description: "AutoCompare color theme",
      defaultValue: "light",
      toolbar: {
        icon: "circlehollow",
        items: [
          { value: "light", title: "Light" },
          { value: "dark", title: "Dark" },
        ],
        dynamicTitle: true,
      },
    },
    layout: {
      name: "Density",
      description: "UI layout density",
      defaultValue: "standard",
      toolbar: {
        icon: "sidebar",
        items: [
          { value: "standard", title: "Standard" },
          { value: "compact", title: "Compact" },
        ],
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: {
    theme: "light",
    layout: "standard",
  },
  decorators: [
    (Story, context) => {
      const theme = context.globals.theme === "dark" ? "dark" : "light";
      const layout = context.globals.layout === "compact" ? "compact" : "standard";
      const fullscreen = context.parameters.layout === "fullscreen";
      return (
        <StoryThemeWrapper theme={theme} layout={layout} fullscreen={fullscreen}>
          <Story />
        </StoryThemeWrapper>
      );
    },
  ],
  parameters: {
    layout: "padded",
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    a11y: {
      test: "error",
    },
  },
};

export default preview;
