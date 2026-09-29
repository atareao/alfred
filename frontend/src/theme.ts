import { theme } from "antd";
import type { ThemeConfig } from "antd";

const { darkAlgorithm } = theme;

export const valetTheme: ThemeConfig = {
  algorithm: darkAlgorithm,
  token: {
    colorPrimary: "#1677ff",
    borderRadius: 6,
    colorBgContainer: "#141414",
    colorBgLayout: "#000000",
    colorText: "#ffffff",
    colorBgElevated: "#1f1f1f",
    colorBorder: "rgba(255,255,255,0.15)",
  },
  components: {
    Layout: {
      siderBg: "#001529",
      headerBg: "#141414",
      bodyBg: "#000000",
    },
    Menu: {
      darkItemBg: "#001529",
      darkItemSelectedBg: "#1677ff",
    },
    Table: {
      headerBg: "#1a1a2e",
      headerColor: "#ffffff",
      headerSortActiveBg: "#1f1f2e",
      headerSortHoverBg: "#1f1f2e",
      bodySortBg: "#1a1a1a",
      rowHoverBg: "rgba(255,255,255,0.06)",
      rowSelectedBg: "#1a1a2e",
      rowSelectedHoverBg: "#1f1f2e",
      borderColor: "rgba(255,255,255,0.1)",
      footerBg: "#141414",
      footerColor: "#ffffff",
      cellFontSize: 12,
    },
    Card: {
      colorBgContainer: "#1a1a1a",
    },
  },
};
