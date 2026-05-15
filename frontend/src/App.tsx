import { useEffect, useMemo, useState } from "react";
import { backupConfig, deleteKey, detectTools, listKeys, saveKey, switchKey } from "./api";
import { dictionaries, resolveLocale, type Locale } from "./i18n";
import type { KeyRecord, ToolStatus, ToolType } from "./types";

const emptyDraft: KeyRecord = {
  id: "",
  name: "",
  tool: "codex",
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
  const [draft, setDraft] = useState<KeyRecord>(emptyDraft);
  const [log, setLog] = useState(dictionaries[locale].ready);

  const t = dictionaries[locale];

  const activeByTool = useMemo(() => {
    const map: Record<ToolType, string> = {
      "claude-code": "-",
      codex: "-",
      "gemini-cli": "-"
    };
    for (const key of keys) {
      if (key.isActive) {
        map[key.tool] = key.name;
      }
    }
    return map;
  }, [keys]);

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
    if (log === dictionaries[locale === "zh-CN" ? "en-US" : "zh-CN"].ready) {
      setLog(t.ready);
    }
  }, [locale, log, t.ready]);

  const onSubmit = async () => {
    const now = new Date().toISOString();
    const payload: KeyRecord = {
      ...draft,
      id: draft.id || crypto.randomUUID(),
      createdAt: draft.createdAt || now,
      updatedAt: now
    };
    await saveKey(payload);
    setLog(`${t.savedKey}: ${payload.name}`);
    setDraft(emptyDraft);
    await reload();
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

      <div className="panel grid">
        {(["claude-code", "codex", "gemini-cli"] as ToolType[]).map((tool) => {
          const status = tools.find((x) => x.tool === tool);
          return (
            <div key={tool}>
              <strong>{tool}</strong>
              <div>{status?.installed ? t.installed : t.notInstalled}</div>
              <div>{t.activeKey}: {activeByTool[tool]}</div>
              <button
                className="secondary"
                onClick={async () => {
                  const result = await backupConfig(tool);
                  setLog(result.message);
                }}
              >
                {t.backupConfig}
              </button>
            </div>
          );
        })}
      </div>

      <div className="panel">
        <h2>{t.editKey}</h2>
        <div className="grid">
          <label>
            {t.keyName}
            <input value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
          </label>
          <label>
            {t.tool}
            <select value={draft.tool} onChange={(e) => setDraft({ ...draft, tool: e.target.value as ToolType })}>
              <option value="claude-code">claude-code</option>
              <option value="codex">codex</option>
              <option value="gemini-cli">gemini-cli</option>
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
            {t.note}
            <textarea value={draft.note || ""} onChange={(e) => setDraft({ ...draft, note: e.target.value })} />
          </label>
        </div>
        <div className="row">
          <button onClick={() => onSubmit().catch((err) => setLog(String(err)))}>{t.saveKey}</button>
          <button className="secondary" onClick={() => setDraft(emptyDraft)}>{t.clear}</button>
        </div>
      </div>

      <div className="panel">
        <h2>{t.keyList}</h2>
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
                onClick={async () => {
                  const result = await switchKey(key.id);
                  setLog(result.message);
                  await reload();
                }}
              >
                {t.switchKey}
              </button>
              <button className="secondary" onClick={() => setDraft(key)}>{t.edit}</button>
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

      <div className="panel">
        <h2>{t.log}</h2>
        <div className="log">{log}</div>
      </div>
    </div>
  );
}

export default App;
