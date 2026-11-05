import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Settings, Moon, Sun, Key, CheckCircle } from 'lucide-react';

interface SettingsViewProps {
  currentTheme: string;
  onThemeChange: (theme: string) => void;
  apiKey: string;
  onSaveApiKey: (key: string) => Promise<void>;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  currentTheme,
  onThemeChange,
  apiKey: initialKey,
  onSaveApiKey,
}) => {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState(initialKey);
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    await onSaveApiKey(apiKey.trim());
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  return (
    <div className="space-y-6 max-w-2xl animate-in fade-in duration-200">
      <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
        <Settings className="w-5 h-5 text-indigo-500" />
        <span>{t('settings.title')}</span>
      </h2>

      {/* Theme Setting */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-3 shadow-sm">
        <div>
          <h3 className="text-sm font-bold text-slate-900 dark:text-slate-200">
            {t('settings.theme_title')}
          </h3>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1">
            {t('settings.theme_desc')}
          </p>
        </div>

        <div className="flex items-center gap-3 pt-1">
          <button
            onClick={() => onThemeChange('dark')}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all ${
              currentTheme === 'dark'
                ? 'border-indigo-500 bg-indigo-500/20 text-indigo-400 shadow-md shadow-indigo-500/10'
                : 'border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Moon className="w-4 h-4 text-indigo-400" />
            <span>{t('settings.theme_dark')}</span>
          </button>

          <button
            onClick={() => onThemeChange('light')}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-xl border text-xs font-semibold transition-all ${
              currentTheme === 'light'
                ? 'border-indigo-500 bg-indigo-50 text-indigo-700 shadow-md shadow-indigo-500/10'
                : 'border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200'
            }`}
          >
            <Sun className="w-4 h-4 text-amber-500" />
            <span>{t('settings.theme_light')}</span>
          </button>
        </div>
      </div>

      {/* OMDb API Key */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-5 backdrop-blur-md space-y-4 shadow-sm">
        <div>
          <div className="flex items-center gap-2 text-indigo-600 dark:text-indigo-400 font-bold text-sm">
            <Key className="w-4 h-4" />
            <span>{t('settings.omdb_title')}</span>
          </div>
          <p className="text-xs text-slate-500 dark:text-slate-400 mt-1 leading-relaxed">
            {t('settings.omdb_desc')}
          </p>
        </div>

        <div>
          <label className="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">
            {t('settings.omdb_key_label')}
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t('settings.omdb_key_placeholder')}
              className="flex-1 rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono text-xs"
            />
            <button
              onClick={handleSave}
              className="rounded-xl bg-indigo-600 px-5 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 transition-all"
            >
              {t('settings.save')}
            </button>
          </div>
        </div>

        {saved && (
          <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400">
            <CheckCircle className="w-3.5 h-3.5" />
            <span>Настройки успешно сохранены</span>
          </div>
        )}
      </div>
    </div>
  );
};
