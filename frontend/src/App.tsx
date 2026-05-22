import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { message, open } from "@tauri-apps/plugin-dialog";
import {
  deleteKey,
  detectTools,
  ensureInitialKeyForTool,
  installTool,
  listKeys,
  restartTool,
  saveKey,
  startTool,
  switchKey,
  syncActiveKeyForTool,
  uninstallTool
} from "./api";
import { dictionaries, resolveLocale, type Locale } from "./i18n";
import type { KeyRecord, ToolStatus, ToolType } from "./types";

const BASE_URL_HISTORY_STORAGE_KEY = "keypilot.base_url_history";
const MAX_BASE_URL_HISTORY = 10;

const toolConfigGroup = (tool: ToolType) => (tool === "codex-app" ? "codex" : tool);

const toolsShareConfig = (left: ToolType, right: ToolType) => toolConfigGroup(left) === toolConfigGroup(right);

const formatApiKeyPreview = (apiKey: string) => {
  if (apiKey.length <= 28) return apiKey;
  return `${apiKey.slice(0, 16)}...${apiKey.slice(-12)}`;
};

const shouldShowModel = (tool: ToolType) => toolConfigGroup(tool) === "codex";

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
  const [isKeyListLoading, setIsKeyListLoading] = useState(false);
  const [projectDir, setProjectDir] = useState("");
  const [launchArgs, setLaunchArgs] = useState("");
  const [showArgMenu, setShowArgMenu] = useState(false);
  const argMenuWrapRef = useRef<HTMLDivElement | null>(null);
  const [draft, setDraft] = useState<KeyRecord>(emptyDraft);
  const [baseUrlHistory, setBaseUrlHistory] = useState<Partial<Record<ToolType, string[]>>>({});
  const [deletingKeyId, setDeletingKeyId] = useState<string | null>(null);
  const [switchingKeyId, setSwitchingKeyId] = useState<string | null>(null);
  const [installingTool, setInstallingTool] = useState<ToolType | null>(null);
  const [geminiCustomInstallCmd, setGeminiCustomInstallCmd] = useState("");
  const [geminiCustomUninstallCmd, setGeminiCustomUninstallCmd] = useState("");
  const [log, setLog] = useState(dictionaries[locale].ready);

  const t = dictionaries[locale];

  const persistBaseUrlHistory = (next: Partial<Record<ToolType, string[]>>) => {
    setBaseUrlHistory(next);
    localStorage.setItem(BASE_URL_HISTORY_STORAGE_KEY, JSON.stringify(next));
  };

  const saveBaseUrlToHistory = (tool: ToolType, baseUrl?: string) => {
    const normalized = (baseUrl ?? "").trim();
    if (!normalized) return;
    const historyKey = toolConfigGroup(tool);
    const previous = baseUrlHistory[historyKey] ?? [];
    const deduped = [normalized, ...previous.filter((item) => item !== normalized)].slice(0, MAX_BASE_URL_HISTORY);
    persistBaseUrlHistory({
      ...baseUrlHistory,
      [historyKey]: deduped
    });
  };

  const reloadKeys = async (tool?: ToolType) => {
    const targetTool = tool ?? selectedTool;
    const { keys: allKeys, effectiveSnapshot } = await syncActiveKeyForTool(targetTool);
    const filtered = allKeys.filter((item) => toolsShareConfig(item.tool, targetTool));
    setKeys(filtered);
    const active = filtered.find((k) => k.isActive);
    if (active) {
      const keyDisplay = effectiveSnapshot?.apiKey
        ? formatApiKeyPreview(effectiveSnapshot.apiKey)
        : formatApiKeyPreview(active.apiKey);
      const sourcePart = effectiveSnapshot ? ` | ${t.currentConfigFromTool}: ${effectiveSnapshot.source}` : "";
      setLog(`[${targetTool}] ${t.activeKeyLog}: ${active.note || active.name} | ${keyDisplay}${sourcePart}`);
    } else {
      setLog(`[${targetTool}] ${t.noActiveKeyLog}`);
    }
  };

  const reloadTools = async () => {
    const tStatus = await detectTools();
    setTools(tStatus);
  };

  const reloadAll = async (tool?: ToolType) => {
    await Promise.all([reloadKeys(tool), reloadTools()]);
  };

  useEffect(() => {
    try {
      const raw = localStorage.getItem(BASE_URL_HISTORY_STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Partial<Record<ToolType, string[]>>;
      setBaseUrlHistory(parsed);
    } catch {
      localStorage.removeItem(BASE_URL_HISTORY_STORAGE_KEY);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setIsKeyListLoading(true);
      try {
        await reloadAll(selectedTool);
      } catch (err) {
        setLog(`${t.initFailed}: ${String(err)}`);
      } finally {
        if (!cancelled) {
          setIsKeyListLoading(false);
        }
      }
    };
    run().catch(() => undefined);
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedTool]);

  useEffect(() => {
    const unlistenPromise = listen<string>("key-switched", async (event) => {
      setLog(`${t.traySwitched}: ${event.payload}`);
      await reloadAll(selectedTool);
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

  useEffect(() => {
    if (!showArgMenu) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node | null;
      if (!target) return;
      if (argMenuWrapRef.current?.contains(target)) return;
      setShowArgMenu(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
    };
  }, [showArgMenu]);

  const onSubmit = async () => {
    const normalizedApiKey = draft.apiKey.trim();
    const allKeys = await listKeys();
    const duplicate = allKeys.find(
      (item) => toolsShareConfig(item.tool, selectedTool) && item.apiKey.trim() === normalizedApiKey && item.id !== draft.id
    );
    if (duplicate) {
      await message(t.duplicateApiKey, {
        title: t.addKey,
        kind: "warning"
      });
      setLog(t.duplicateApiKey);
      return;
    }

    const now = new Date().toISOString();
    const autoName = `${selectedTool}-${now.slice(0, 19).replace("T", " ")}`;
    const payload: KeyRecord = {
      ...draft,
      tool: selectedTool,
      apiKey: normalizedApiKey,
      id: draft.id || crypto.randomUUID(),
      name: draft.name || autoName,
      createdAt: draft.createdAt || now,
      updatedAt: now
    };
    await saveKey(payload);
    saveBaseUrlToHistory(selectedTool, payload.baseUrl);
    setLog(`${t.savedKey}: ${payload.name}`);
    setDraft({ ...emptyDraft, tool: selectedTool });
    setShowAddModal(false);
    await reloadAll(selectedTool);
  };

  const handleSwitch = async (key: KeyRecord, withRestart: boolean) => {
    try {
      setSwitchingKeyId(key.id);
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
      await reloadAll(selectedTool);
    } finally {
      setSwitchingKeyId(null);
    }
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

  const baseUrlSuggestions = useMemo(() => {
    return baseUrlHistory[toolConfigGroup(selectedTool)] ?? [];
  }, [baseUrlHistory, selectedTool]);

  const openEditModal = (key: KeyRecord) => {
    setDraft(key);
    setShowAddModal(true);
  };

  const isSwitching = switchingKeyId !== null;
  const isInstalling = installingTool !== null;

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
            <>
              {selectedTool === "gemini-cli" && (
                <label className="launch-args-label">
                  <span>{t.customInstallCmd}</span>
                  <input
                    value={geminiCustomInstallCmd}
                    onChange={(e) => setGeminiCustomInstallCmd(e.target.value)}
                    placeholder="e.g. npm install -g @your-provider/gemini-cli"
                  />
                </label>
              )}
              <button
                disabled={isSwitching || isInstalling}
                onClick={async () => {
                  try {
                    setInstallingTool(selectedTool);
                    setLog(`${t.installStarted}: ${selectedTool}`);
                    const result = await installTool(
                      selectedTool,
                      selectedTool === "gemini-cli" ? geminiCustomInstallCmd : undefined
                    );
                    setLog((prev) => `${prev}\n${result}`);
                    await reloadAll(selectedTool);
                  } catch (err) {
                    setLog((prev) => `${prev}\n${String(err)}`);
                  } finally {
                    setInstallingTool(null);
                  }
                }}
              >
                {installingTool === selectedTool ? t.installStarted : t.installTool}
              </button>
            </>
          ) : (
            <>
              <label className="project-dir-label">
                <span>{t.projectDir}</span>
                <div className="project-dir-input-wrap">
                  <input value={projectDir} onChange={(e) => setProjectDir(e.target.value)} />
                  <button
                    type="button"
                    className="secondary"
                    disabled={isSwitching}
                    onClick={async () => {
                      const selected = await open({ directory: true, multiple: false });
                      if (typeof selected === "string") {
                        setProjectDir(selected);
                      }
                    }}
                  >
                    {t.selectDir}
                  </button>
                </div>
              </label>
              <label className="launch-args-label">
                <span>{t.launchArgs}</span>
                <div className="launch-args-input-wrap">
                  <input value={launchArgs} onChange={(e) => setLaunchArgs(e.target.value)} />
                  <div className="args-plus-wrap" ref={argMenuWrapRef}>
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
              {selectedTool === "gemini-cli" && (
                <label className="launch-args-label">
                  <span>{t.customUninstallCmd}</span>
                  <input
                    value={geminiCustomUninstallCmd}
                    onChange={(e) => setGeminiCustomUninstallCmd(e.target.value)}
                    placeholder="e.g. npm uninstall -g @your-provider/gemini-cli"
                  />
                </label>
              )}
              <button
                disabled={isSwitching}
                onClick={async () => {
                  if (!projectDir.trim()) {
                    await message(t.projectDirRequired, {
                      title: t.projectDir,
                      kind: "warning"
                    });
                    return;
                  }
                  const result = await startTool(selectedTool, launchArgs, projectDir || undefined);
                  setLog(result);
                }}
              >
                {t.startTool}
              </button>
              <button
                className="danger"
                disabled={isSwitching}
                onClick={async () => {
                  const result = await uninstallTool(
                    selectedTool,
                    selectedTool === "gemini-cli" ? geminiCustomUninstallCmd : undefined
                  );
                  setLog(result);
                  await reloadAll(selectedTool);
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
            disabled={isSwitching}
            onClick={() => {
              setDraft({ ...emptyDraft, tool: selectedTool, baseUrl: "" });
              setShowAddModal(true);
            }}
            aria-label={t.saveKey}
          >
            +
          </button>
        </div>
        {isKeyListLoading ? <div className="list-item">{t.loading}</div> : (
          <>
            {keys.length === 0 ? <div className="list-item muted">{t.noKeys}</div> : null}
            {keys.map((key) => (
              <div key={key.id} className="list-item key-item">
                <div className="key-item-main">
                  <div className="row key-title-row">
                    <strong>{t.remark}: {key.note || "-"}</strong>
                    <span className="tag">{key.tool}</span>
                    {key.isActive ? <span className="tag active-tag">{t.activeKey}</span> : null}
                  </div>
                  <div className="key-field">{t.apiKey}: {formatApiKeyPreview(key.apiKey)}</div>
                  <div className="key-field">{t.baseUrl}: {key.baseUrl || "-"}</div>
                  {shouldShowModel(key.tool) ? <div className="key-field">{t.model}: {key.model || "-"}</div> : null}
                </div>
                {!key.isActive ? (
                  <div className="key-item-actions">
                    <button
                      disabled={deletingKeyId === key.id || isSwitching}
                      onClick={() => handleSwitch(key, false).catch((err) => setLog(String(err)))}
                    >
                      {switchingKeyId === key.id ? `${dictionaries["en-US"].switching}（${dictionaries["zh-CN"].switching}）` : t.switchKey}
                    </button>
                    <button
                      className="secondary"
                      disabled={deletingKeyId === key.id || isSwitching}
                      onClick={() => openEditModal(key)}
                    >
                      {t.edit}
                    </button>
                    <button
                      className="danger"
                      disabled={deletingKeyId === key.id || isSwitching}
                      onClick={async () => {
                        try {
                          setDeletingKeyId(key.id);
                          await deleteKey(key.id);
                          setLog(`${t.deletedKey}: ${key.name}`);
                          await reloadAll(selectedTool);
                        } finally {
                          setDeletingKeyId(null);
                        }
                      }}
                    >
                      {deletingKeyId === key.id ? `${dictionaries["en-US"].deleting}（${dictionaries["zh-CN"].deleting}）` : t.delete}
                    </button>
                  </div>
                ) : null}
              </div>
            ))}
          </>
        )}
      </div>

      {showAddModal ? (
        <div className="modal-overlay" onClick={() => setShowAddModal(false)}>
          <div className="modal-card" onClick={(e) => e.stopPropagation()}>
            <div className="panel-head">
              <h2>{`${draft.id ? t.editKey : t.addKey} · ${toolOptions.find((item) => item.value === selectedTool)?.label ?? selectedTool}`}</h2>
              <button className="plus-button" onClick={() => setShowAddModal(false)} aria-label="Close">x</button>
            </div>
            <div className="grid add-key-form">
              <label>
                {t.remark}
                <input value={draft.note || ""} onChange={(e) => setDraft({ ...draft, note: e.target.value })} />
              </label>
              <label>
                {t.apiKey}
                <input value={draft.apiKey} onChange={(e) => setDraft({ ...draft, apiKey: e.target.value })} />
              </label>
              <label>
                {t.baseUrl}
                <input
                  list={`base-url-history-${selectedTool}`}
                  value={draft.baseUrl || ""}
                  onChange={(e) => setDraft({ ...draft, baseUrl: e.target.value })}
                  onBlur={() => saveBaseUrlToHistory(selectedTool, draft.baseUrl)}
                />
                <datalist id={`base-url-history-${selectedTool}`}>
                  {baseUrlSuggestions.map((url) => (
                    <option key={url} value={url} />
                  ))}
                </datalist>
              </label>
              <label>
                {`${t.model} (${t.optional})`}
                <input value={draft.model || ""} onChange={(e) => setDraft({ ...draft, model: e.target.value })} />
              </label>
            </div>
            <div className="row">
              <button disabled={isSwitching} onClick={() => onSubmit().catch((err) => setLog(String(err)))}>{t.saveKey}</button>
            </div>
          </div>
        </div>
      ) : null}

      <div className="panel">
        <h2>{t.log}</h2>
        <div className="log">{log}</div>
      </div>
      {isSwitching ? (
        <div className="busy-overlay" aria-live="polite">
          <div className="busy-card">{`${dictionaries["en-US"].switching}（${dictionaries["zh-CN"].switching}）...`}</div>
        </div>
      ) : null}
    </div>
  );
}

export default App;
