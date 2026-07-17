import type { Preview } from "@storybook/sveltekit";
import "../src/app.css";
import "@hackernoon/pixel-icon-library/fonts/iconfont.css";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: { color: /(background|color)$/i, date: /Date$/i },
    },
    backgrounds: {
      default: "Stone",
      values: [
        { name: "Stone", value: "#17161a" },
        { name: "Card", value: "#2a2825" },
      ],
    },
    a11y: { test: "todo" },
  },
};

export default preview;
