import { useState, useEffect, useCallback } from "react";
import { api } from "../api/client";

interface UseSettingsReturn {
  settings: Record<string, string> | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  updateSettings: (data: Record<string, string>) => Promise<void>;
  resetToDefaults: () => Promise<void>;
}

export function useSettings(): UseSettingsReturn {
  const [settings, setSettings] = useState<Record<string, string> | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getSettings();
      setSettings(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Error loading settings");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const updateSettings = useCallback(async (data: Record<string, string>) => {
    setSaving(true);
    try {
      const result = await api.updateSettings(data);
      setSettings(result);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Error saving settings");
      throw e;
    } finally {
      setSaving(false);
    }
  }, []);

  const resetToDefaults = useCallback(async () => {
    await updateSettings({
      max_window_tokens: "10000",
      system_prompt: "",
    });
  }, [updateSettings]);

  return { settings, loading, saving, error, updateSettings, resetToDefaults };
}
