import type { StorybookConfig } from "@storybook/react-vite";

const config: StorybookConfig = {
  stories: ["../src/v2/**/*.stories.@(ts|tsx)"],
  addons: [
    "@storybook/addon-docs",
    "@storybook/addon-a11y",
    "@storybook/addon-vitest",
  ],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  viteFinal(viteCfg) {
    return {
      ...viteCfg,
      server: {
        ...viteCfg.server,
        port: undefined,
        strictPort: false,
      },
      define: {
        ...viteCfg.define,
        "import.meta.env.VITE_MOCK_IPC": JSON.stringify("1"),
      },
    };
  },
};

export default config;
