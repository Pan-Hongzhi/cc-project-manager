import type { GlobalThemeOverrides } from "naive-ui";

/**
 * 「终端工作台」主题：与应用图标 H 同一套材质——近黑的终端底色、锌灰窗框、
 * 珊瑚橙的像素小人作为品牌主色、绿色光标块表示「运行中」。
 */
export const palette = {
  ink: "#0C0C0F", // 终端底色（图标前窗）
  panel: "#131316",
  panelRaised: "#18181B",
  chrome: "#27272A", // 图标标题栏
  chromeRaised: "#3F3F46", // 图标后窗
  edge: "#52525B", // 图标窗框描边
  muted: "#71717A", // 图标标题栏圆点
  text: "#E4E4E7",
  textStrong: "#FAFAFA",
  coral: "#E07A5F", // 像素小人
  coralHover: "#E98B72",
  coralPressed: "#C9674F",
  green: "#22C55E", // 光标块 / 运行中
  amber: "#F5A524",
  red: "#F87171",
  blue: "#7DA2FF",
} as const;

export const fonts = {
  ui: '"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", system-ui, sans-serif',
  mono: '"Cascadia Code", "Cascadia Mono", Consolas, "Courier New", monospace',
} as const;

export const themeOverrides: GlobalThemeOverrides = {
  common: {
    fontFamily: fonts.ui,
    fontFamilyMono: fonts.mono,
    fontSize: "13px",
    fontSizeSmall: "12px",
    borderRadius: "6px",
    borderRadiusSmall: "4px",
    primaryColor: palette.coral,
    primaryColorHover: palette.coralHover,
    primaryColorPressed: palette.coralPressed,
    primaryColorSuppl: palette.coralHover,
    successColor: palette.green,
    successColorHover: "#34D06C",
    successColorPressed: "#1DAA51",
    warningColor: palette.amber,
    warningColorHover: "#F8B546",
    warningColorPressed: "#D9901A",
    errorColor: palette.red,
    errorColorHover: "#FA8A8A",
    errorColorPressed: "#E05555",
    infoColor: palette.blue,
    infoColorHover: "#95B4FF",
    infoColorPressed: "#6690F0",
    bodyColor: palette.ink,
    cardColor: palette.panel,
    modalColor: palette.panelRaised,
    popoverColor: palette.panelRaised,
    tableColor: palette.panel,
    tableHeaderColor: palette.panelRaised,
    inputColor: palette.panelRaised,
    actionColor: palette.panelRaised,
    hoverColor: "rgba(224, 122, 95, 0.08)",
    borderColor: palette.chrome,
    dividerColor: palette.chrome,
    textColorBase: palette.text,
    textColor1: palette.textStrong,
    textColor2: palette.text,
    textColor3: palette.muted,
    placeholderColor: palette.muted,
    scrollbarColor: palette.chromeRaised,
    scrollbarColorHover: palette.edge,
  },
  DataTable: {
    thColor: palette.panelRaised,
    tdColor: palette.panel,
    tdColorHover: "rgba(224, 122, 95, 0.07)",
    borderColor: palette.chrome,
    thTextColor: palette.muted,
    thFontWeight: "600",
    tdTextColor: palette.text,
  },
  Tabs: {
    tabTextColorLine: palette.muted,
    tabTextColorHoverLine: palette.text,
    tabTextColorActiveLine: palette.textStrong,
    barColor: palette.coral,
    tabFontWeightActive: "600",
  },
  Tag: {
    borderRadius: "4px",
    colorBordered: "transparent",
  },
  Descriptions: {
    thColor: palette.panelRaised,
    tdColor: palette.panel,
    borderColor: palette.chrome,
    thTextColor: palette.muted,
  },
  Table: {
    thColor: palette.panelRaised,
    tdColor: palette.panel,
    borderColor: palette.chrome,
    thTextColor: palette.muted,
  },
  Alert: {
    color: palette.panelRaised,
    border: `1px solid ${palette.chrome}`,
  },
  Divider: {
    color: palette.chrome,
    textColor: palette.muted,
  },
  Statistic: {
    labelTextColor: palette.muted,
    valueTextColor: palette.textStrong,
    valueFontFamily: fonts.mono,
  },
  Progress: {
    railColor: palette.chrome,
  },
};
