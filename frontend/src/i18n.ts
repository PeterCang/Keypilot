export type Locale = "zh-CN" | "en-US";

type Dict = {
  appTitle: string;
  language: string;
  zhCN: string;
  enUS: string;
  initFailed: string;
  ready: string;
  installed: string;
  notInstalled: string;
  activeKey: string;
  backupConfig: string;
  editKey: string;
  keyName: string;
  tool: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  note: string;
  saveKey: string;
  clear: string;
  keyList: string;
  active: string;
  noBaseUrl: string;
  switchKey: string;
  edit: string;
  delete: string;
  log: string;
  savedKey: string;
  deletedKey: string;
};

export const dictionaries: Record<Locale, Dict> = {
  "zh-CN": {
    appTitle: "Keypilot MVP 控制台",
    language: "语言",
    zhCN: "中文",
    enUS: "英文",
    initFailed: "初始化失败",
    ready: "就绪",
    installed: "已安装",
    notInstalled: "未安装",
    activeKey: "当前激活",
    backupConfig: "备份配置",
    editKey: "新增 / 编辑 Key",
    keyName: "名称",
    tool: "工具",
    apiKey: "API Key",
    baseUrl: "Base URL",
    model: "Model",
    note: "备注",
    saveKey: "保存 Key",
    clear: "清空",
    keyList: "Key 列表",
    active: "激活",
    noBaseUrl: "(无 Base URL)",
    switchKey: "切换",
    edit: "编辑",
    delete: "删除",
    log: "日志",
    savedKey: "已保存 Key",
    deletedKey: "已删除 Key"
  },
  "en-US": {
    appTitle: "Keypilot MVP Console",
    language: "Language",
    zhCN: "Chinese",
    enUS: "English",
    initFailed: "Init failed",
    ready: "Ready",
    installed: "Installed",
    notInstalled: "Not Installed",
    activeKey: "Active",
    backupConfig: "Backup Config",
    editKey: "Add / Edit Key",
    keyName: "Name",
    tool: "Tool",
    apiKey: "API Key",
    baseUrl: "Base URL",
    model: "Model",
    note: "Note",
    saveKey: "Save Key",
    clear: "Clear",
    keyList: "Key List",
    active: "active",
    noBaseUrl: "(No Base URL)",
    switchKey: "Switch",
    edit: "Edit",
    delete: "Delete",
    log: "Log",
    savedKey: "Saved key",
    deletedKey: "Deleted key"
  }
};

export const resolveLocale = (input?: string): Locale => {
  if (!input) return "zh-CN";
  if (input.toLowerCase().startsWith("en")) return "en-US";
  return "zh-CN";
};
