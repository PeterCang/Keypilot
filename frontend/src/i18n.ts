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
  installTool: string;
  uninstallTool: string;
  launchArgs: string;
  startTool: string;
  toolManager: string;
  addKey: string;
  unsupportedToolHint: string;
  editKey: string;
  keyName: string;
  tool: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  note: string;
  remark: string;
  saveKey: string;
  clear: string;
  keyList: string;
  active: string;
  noBaseUrl: string;
  switchKey: string;
  switchAndRestart: string;
  edit: string;
  delete: string;
  log: string;
  savedKey: string;
  deletedKey: string;
  restartRecommended: string;
  restartedTool: string;
  traySwitched: string;
  installStarted: string;
  addArg: string;
  argPresets: string;
};

export const dictionaries: Record<Locale, Dict> = {
  "zh-CN": {
    appTitle: "Keypilot控制台",
    language: "语言",
    zhCN: "中文",
    enUS: "英文",
    initFailed: "初始化失败",
    ready: "就绪",
    installed: "已安装",
    notInstalled: "未安装",
    activeKey: "当前激活",
    backupConfig: "备份配置",
    installTool: "安装工具",
    uninstallTool: "卸载工具",
    launchArgs: "启动参数",
    startTool: "启动工具",
    toolManager: "工具管理",
    addKey: "新增 Key",
    unsupportedToolHint: "Codex App 当前版本仅展示入口，安装/卸载/启动暂未接入",
    editKey: "新增 / 编辑 Key",
    keyName: "名称",
    tool: "工具",
    apiKey: "API Key",
    baseUrl: "Base URL",
    model: "Model",
    note: "备注",
    remark: "Remark",
    saveKey: "保存 Key",
    clear: "清空",
    keyList: "Key 列表",
    active: "激活",
    noBaseUrl: "(无 Base URL)",
    switchKey: "切换",
    switchAndRestart: "切换并重启",
    edit: "编辑",
    delete: "删除",
    log: "日志",
    savedKey: "已保存 Key",
    deletedKey: "已删除 Key",
    restartRecommended: "检测到工具正在运行，建议重启",
    restartedTool: "已执行重启指令",
    traySwitched: "托盘已切换 Key"
    ,
    installStarted: "开始安装",
    addArg: "添加参数",
    argPresets: "常用参数"
  },
  "en-US": {
    appTitle: "Keypilot Console",
    language: "Language",
    zhCN: "Chinese",
    enUS: "English",
    initFailed: "Init failed",
    ready: "Ready",
    installed: "Installed",
    notInstalled: "Not Installed",
    activeKey: "Active",
    backupConfig: "Backup Config",
    installTool: "Install Tool",
    uninstallTool: "Uninstall Tool",
    launchArgs: "Launch Args",
    startTool: "Start Tool",
    toolManager: "Tool Manager",
    addKey: "Add Key",
    unsupportedToolHint: "Codex App is listed, but install/uninstall/start is not wired yet",
    editKey: "Add / Edit Key",
    keyName: "Name",
    tool: "Tool",
    apiKey: "API Key",
    baseUrl: "Base URL",
    model: "Model",
    note: "Note",
    remark: "Remark",
    saveKey: "Save Key",
    clear: "Clear",
    keyList: "Key List",
    active: "active",
    noBaseUrl: "(No Base URL)",
    switchKey: "Switch",
    switchAndRestart: "Switch + Restart",
    edit: "Edit",
    delete: "Delete",
    log: "Log",
    savedKey: "Saved key",
    deletedKey: "Deleted key",
    restartRecommended: "Tool is running, restart is recommended",
    restartedTool: "Restart command has been executed",
    traySwitched: "Key switched from tray",
    installStarted: "Install started",
    addArg: "Add Arg",
    argPresets: "Common Args"
  }
};

export const resolveLocale = (input?: string): Locale => {
  if (!input) return "zh-CN";
  if (input.toLowerCase().startsWith("en")) return "en-US";
  return "zh-CN";
};
