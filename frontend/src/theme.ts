import type { ThemeConfig } from "antd";

export const alfredTheme: ThemeConfig = {
  token: {
    colorPrimary: "#1677ff",
    borderRadius: 6,
    colorBgContainer: "#141414",
    colorBgLayout: "#000000",
    colorText: "#ffffff",
    colorBgElevated: "#1f1f1f",
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
  },
};
