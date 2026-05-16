import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  deleteKey,
  detectTools,
  installTool,
  listKeys,
  restartTool,
  saveKey,
  startTool,
  switchKey,
  uninstallTool
} from "./api";
import { dictionaries, resolveLocale, type Locale } from "./i18n";
import type { KeyRecord, ToolStatus, ToolType } from "./types";

const emptyDraft: KeyRecord = {
  id: "",
  name: "",
  tool: "claude-code",
  apiKey: "",
  baseUrl: "",
  model: "",
  isActive: false,
  createdAt: "",
  note: ""
};

function App() {
  const [locale, setLocale] = useState<Locale>(resolveLocale(navigator.language));
  const [keys, setKeys] = useState<KeyRecord[]>([]);
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedTool, setSelectedTool] = useState<ToolType>("claude-code");
  const [launchArgs, setLaunchArgs] = useState("");
  const [showArgMenu, setShowArgMenu] = useState(false);
  const [draft, setDraft] = useState<KeyRecord>(emptyDraft);
  const [log, setLog] = useState(dictionaries[locale].ready);

  const t = dictionaries[locale];

  const reload = async () => {
    const [k, tStatus] = await Promise.all([listKeys(), detectTools()]);
    setKeys(k);
    setTools(tStatus);
  };

  useEffect(() => {
    reload().catch((err) => setLog(`${t.initFailed}: ${String(err)}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const unlistenPromise = listen<string>("key-switched", async (event) => {
      setLog(`${t.traySwitched}: ${event.payload}`);
      await reload();
    });
    const installLogUnlistenPromise = listen<string>("install-log", (event) => {
      setLog((prev) => `${prev}\n${event.payload}`);
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
      installLogUnlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locale]);

  useEffect(() => {
    if (log === dictionaries[locale === "zh-CN" ? "en-US" : "zh-CN"].ready) {
      setLog(t.ready);
    }
  }, [locale, log, t.ready]);

  useEffect(() => {
    setShowArgMenu(false);
  }, [selectedTool]);

  const onSubmit = async () => {
    const now = new Date().toISOString();
    const autoName = `${draft.tool}-${now.slice(0, 19).replace("T", " ")}`;
    const payload: KeyRecord = {
      ...draft,
      id: draft.id || crypto.randomUUID(),
      name: draft.name || autoName,
      createdAt: draft.createdAt || now,
      updatedAt: now
    };
    await saveKey(payload);
    setLog(`${t.savedKey}: ${payload.name}`);
    setDraft(emptyDraft);
    setShowAddModal(false);
    await reload();
  };

  const handleSwitch = async (key: KeyRecord, withRestart: boolean) => {
    const result = await switchKey(key.id);
    let nextLog = result.message;
    if (result.requiresRestart) {
      nextLog = `${nextLog} | ${t.restartRecommended}`;
      if (withRestart) {
        const restartMessage = await restartTool(key.tool);
        nextLog = `${nextLog} | ${t.restartedTool}: ${restartMessage}`;
      }
    }
    setLog(nextLog);
    await reload();
  };

  const selectedToolStatus = useMemo(() => {
    return tools.find((x) => x.tool === selectedTool) ?? { installed: false };
  }, [selectedTool, tools]);

  const toolOptions: Array<{ value: ToolType; label: string }> = useMemo(() => {
    const base: Array<{ value: ToolType; label: string }> = [
      { value: "claude-code", label: "Claude Code" },
      { value: "codex", label: "Codex CLI" },
      { value: "codex-app", label: "Codex App" },
      { value: "gemini-cli", label: "Gemini CLI" }
    ];
    return base.map((item) => {
      const version = tools.find((tItem) => tItem.tool === item.value)?.version;
      return {
        ...item,
        label: version ? `${item.label} (${version})` : item.label
      };
    });
  }, [tools]);

  const argPresetsByTool: Record<ToolType, string[]> = useMemo(
    () => ({
      "claude-code": [
        "--dangerously-skip-permissions",
        "--model sonnet",
        "--model opus",
        "--continue",
        "--resume latest",
        "--permission-mode default",
        "--permission-mode auto",
        "--permission-mode plan",
        "--allowed-tools \"Bash Edit Read\"",
        "--disallowed-tools \"Bash(rm *)\"",
        "--debug",
        "--ide",
        "--print"
      ],
      codex: [
        "--dangerously-bypass-approvals-and-sandbox",
        "--model gpt-5.5",
        "--profile default",
        "--ask-for-approval on-request",
        "--ask-for-approval never",
        "--sandbox workspace-write",
        "--sandbox read-only",
        "--search",
        "--oss"
      ],
      "codex-app": [
        "app .",
        "--dangerously-bypass-approvals-and-sandbox",
        "--model gpt-5.5",
        "--profile default",
        "--ask-for-approval on-request",
        "--ask-for-approval never",
        "--sandbox workspace-write",
        "--sandbox read-only",
        "--search",
        "--oss"
      ],
      "gemini-cli": [
        "--model pro",
        "--model flash",
        "--resume latest",
        "--approval-mode default",
        "--approval-mode auto_edit",
        "--approval-mode plan",
        "--approval-mode yolo",
        "--sandbox",
        "--skip-trust",
        "--debug"
      ]
    }),
    []
  );

  const appendLaunchArg = (arg: string) => {
    setLaunchArgs((prev) => {
      const trimmed = prev.trim();
      if (!trimmed) return arg;
      return `${trimmed} ${arg}`;
    });
    setShowArgMenu(false);
  };

  return (
    <div className="app">
      <div className="row head-row">
        <h1>{t.appTitle}</h1>
        <label className="locale-picker">
          {t.language}
          <select value={locale} onChange={(e) => setLocale(e.target.value as Locale)}>
            <option value="zh-CN">{t.zhCN}</option>
            <option value="en-US">{t.enUS}</option>
          </select>
        </label>
      </div>

      <div className="panel">
        <h2>{t.toolManager}</h2>
        <div className="tool-manager-inline">
          <div className="tool-select-label">
            <span className="tool-label-text">{t.tool}</span>
            <select value={selectedTool} onChange={(e) => setSelectedTool(e.target.value as ToolType)}>
              {toolOptions.map((item) => (
                <option key={item.value} value={item.value}>
                  {item.label}
                </option>
              ))}
            </select>
          </div>
          <div className="tool-install-status">{selectedToolStatus.installed ? t.installed : t.notInstalled}</div>
        </div>
        <div className="row tool-actions-row">
          {!selectedToolStatus.installed ? (
            <button
              onClick={async () => {
                setLog(`${t.installStarted}: ${selectedTool}`);
                const result = await installTool(selectedTool);
                setLog(result);
                await reload();
              }}
            >
              {t.installTool}
            </button>
          ) : (
            <>
              <label className="launch-args-label">
                <span>{t.launchArgs}</span>
                <div className="launch-args-input-wrap">
                  <input value={launchArgs} onChange={(e) => setLaunchArgs(e.target.value)} />
                  <div className="args-plus-wrap">
                    <button
                      type="button"
                      className="arg-plus-button"
                      aria-label={t.addArg}
                      onClick={() => setShowArgMenu((v) => !v)}
                    >
                      +
                    </button>
                    {showArgMenu ? (
                      <div className="arg-menu">
                        <div className="arg-menu-title">{t.argPresets}</div>
                        {argPresetsByTool[selectedTool].map((item) => (
                          <button
                            key={item}
                            type="button"
                            className="arg-menu-item"
                            onClick={() => appendLaunchArg(item)}
                          >
                            {item}
                          </button>
                        ))}
                      </div>
                    ) : null}
                  </div>
                </div>
              </label>
              <button
                onClick={async () => {
                  const result = await startTool(selectedTool, launchArgs);
                  setLog(result);
                }}
              >
                {t.startTool}
              </button>
              <button
                className="danger"
                onClick={async () => {
                  const result = await uninstallTool(selectedTool);
                  setLog(result);
                  await reload();
                }}
              >
                {t.uninstallTool}
              </button>
            </>
          )}
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>{t.keyList}</h2>
          <button
            className="plus-button"
            onClick={() => {
              setDraft(emptyDraft);
              setShowAddModal(true);
            }}
            aria-label={t.saveKey}
          >
            +
          </button>
        </div>
        {keys.map((key) => (
          <div key={key.id} className="list-item">
            <div className="row">
              <strong>{key.name}</strong>
              <span className="tag">{key.tool}</span>
              {key.isActive ? <span className="tag">{t.active}</span> : null}
            </div>
            <div>{key.baseUrl || t.noBaseUrl}</div>
            <div className="row">
              <button
                onClick={() => handleSwitch(key, false).catch((err) => setLog(String(err)))}
              >
                {t.switchKey}
              </button>
              <button
                className="secondary"
                onClick={() => handleSwitch(key, true).catch((err) => setLog(String(err)))}
              >
                {t.switchAndRestart}
              </button>
              <button
                className="danger"
                onClick={async () => {
                  await deleteKey(key.id);
                  setLog(`${t.deletedKey}: ${key.name}`);
                  await reload();
                }}
              >
                {t.delete}
              </button>
            </div>
          </div>
        ))}
      </div>

      {showAddModal ? (
        <div className="modal-overlay" onClick={() => setShowAddModal(false)}>
          <div className="modal-card" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>{t.addKey}</h2>
              <button className="plus-button" onClick={() => setShowAddModal(false)} aria-label="Close">x</button>
            </div>
            <div className="grid">
              <label>
                {t.tool}
                <select value={draft.tool} onChange={(e) => setDraft({ ...draft, tool: e.target.value as ToolType })}>
                  {toolOptions.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                {t.apiKey}
                <input value={draft.apiKey} onChange={(e) => setDraft({ ...draft, apiKey: e.target.value })} />
              </label>
              <label>
                {t.baseUrl}
                <input value={draft.baseUrl || ""} onChange={(e) => setDraft({ ...draft, baseUrl: e.target.value })} />
              </label>
              <label>
                {t.model}
                <input value={draft.model || ""} onChange={(e) => setDraft({ ...draft, model: e.target.value })} />
              </label>
              <label>
                {t.remark}
                <input value={draft.note || ""} onChange={(e) => setDraft({ ...draft, note: e.target.value })} />
              </label>
            </div>
            <div className="row">
              <button onClick={() => onSubmit().catch((err) => setLog(String(err)))}>{t.saveKey}</button>
            </div>
          </div>
        </div>
      ) : null}

      <div className="panel">
        <h2>{t.log}</h2>
        <div className="log">{log}</div>
      </div>
    </div>
  );
}

export default App;
