import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { ExportSummary, ImportSummary } from '../types';
import { Archive, Download, FileSpreadsheet, FileText, CheckCircle, AlertCircle } from 'lucide-react';

interface BackupViewProps {
  onRefresh: () => void;
}

export const BackupView: React.FC<BackupViewProps> = ({ onRefresh }) => {
  const { t } = useTranslation();
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleExportZip = async () => {
    try {
      const filePath = await save({
        filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
        defaultPath: 'myfilmlibrary-backup.zip',
      });
      if (!filePath) return;

      setLoading(true);
      setStatusMessage(null);
      setErrorMessage(null);

      const res = await invoke<ExportSummary>('export_backup_zip', { targetPath: filePath });
      setStatusMessage(`Экспорт завершён: ${res.films} фильмов, ${res.posters} постеров сохранены в архив.`);
    } catch (err: any) {
      setErrorMessage(err?.toString() || 'Ошибка экспорта');
    } finally {
      setLoading(false);
    }
  };

  const handleImportZip = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
      });
      if (!selected || typeof selected !== 'string') return;

      setLoading(true);
      setStatusMessage(null);
      setErrorMessage(null);

      const res = await invoke<ImportSummary>('import_backup_zip', { sourcePath: selected });
      setStatusMessage(
        `Импорт успешен: добавлено ${res.added} фильмов, пропущено ${res.skipped} дубликатов, восстановлено ${res.posters_restored} постеров.`
      );
      onRefresh();
    } catch (err: any) {
      setErrorMessage(err?.toString() || 'Ошибка импорта');
    } finally {
      setLoading(false);
    }
  };

  const handleExportCsv = async () => {
    try {
      const filePath = await save({
        filters: [{ name: 'CSV File', extensions: ['csv'] }],
        defaultPath: 'myfilmlibrary.csv',
      });
      if (!filePath) return;

      setLoading(true);
      setStatusMessage(null);
      setErrorMessage(null);

      await invoke('export_films_csv', { targetPath: filePath });
      setStatusMessage('Список фильмов успешно экспортирован в CSV!');
    } catch (err: any) {
      setErrorMessage(err?.toString() || 'Ошибка экспорта CSV');
    } finally {
      setLoading(false);
    }
  };

  const handleExportMarkdown = async () => {
    try {
      const filePath = await save({
        filters: [{ name: 'Markdown File', extensions: ['md'] }],
        defaultPath: 'myfilmlibrary.md',
      });
      if (!filePath) return;

      setLoading(true);
      setStatusMessage(null);
      setErrorMessage(null);

      await invoke('export_films_markdown', { targetPath: filePath });
      setStatusMessage('Каталог успешно экспортирован в Markdown файл!');
    } catch (err: any) {
      setErrorMessage(err?.toString() || 'Ошибка экспорта Markdown');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6 max-w-4xl animate-in fade-in duration-200">
      <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
        <Archive className="w-5 h-5 text-indigo-400" />
        <span>{t('backup.title')}</span>
      </h2>

      {statusMessage && (
        <div className="flex items-center gap-2 p-4 text-xs font-semibold bg-emerald-500/10 border border-emerald-500/30 rounded-2xl text-emerald-300">
          <CheckCircle className="w-4 h-4 shrink-0 text-emerald-400" />
          <span>{statusMessage}</span>
        </div>
      )}

      {errorMessage && (
        <div className="flex items-center gap-2 p-4 text-xs font-semibold bg-rose-500/10 border border-rose-500/30 rounded-2xl text-rose-300">
          <AlertCircle className="w-4 h-4 shrink-0 text-rose-400" />
          <span>{errorMessage}</span>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Full ZIP Backup */}
        <div className="flex flex-col justify-between rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <div>
            <div className="flex items-center gap-2 text-indigo-400 font-bold text-sm mb-2">
              <Download className="w-4 h-4" />
              <span>{t('backup.export_zip_title')}</span>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed mb-4">
              {t('backup.export_zip_desc')}
            </p>
          </div>
          <button
            onClick={handleExportZip}
            disabled={loading}
            className="w-full rounded-xl bg-indigo-600 px-4 py-2.5 text-xs font-semibold text-white shadow-lg shadow-indigo-600/20 hover:bg-indigo-500 disabled:opacity-50 transition-all"
          >
            {t('backup.export_zip_btn')}
          </button>
        </div>

        {/* Restore ZIP Backup */}
        <div className="flex flex-col justify-between rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <div>
            <div className="flex items-center gap-2 text-emerald-400 font-bold text-sm mb-2">
              <Archive className="w-4 h-4" />
              <span>{t('backup.import_zip_title')}</span>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed mb-4">
              {t('backup.import_zip_desc')}
            </p>
          </div>
          <button
            onClick={handleImportZip}
            disabled={loading}
            className="w-full rounded-xl border border-slate-700 bg-slate-800 px-4 py-2.5 text-xs font-semibold text-slate-200 hover:bg-slate-700 disabled:opacity-50 transition-all"
          >
            {t('backup.import_zip_btn')}
          </button>
        </div>

        {/* Export CSV */}
        <div className="flex flex-col justify-between rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <div>
            <div className="flex items-center gap-2 text-amber-400 font-bold text-sm mb-2">
              <FileSpreadsheet className="w-4 h-4" />
              <span>{t('backup.export_csv_title')}</span>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed mb-4">
              {t('backup.export_csv_desc')}
            </p>
          </div>
          <button
            onClick={handleExportCsv}
            disabled={loading}
            className="w-full rounded-xl border border-slate-700 bg-slate-800 px-4 py-2.5 text-xs font-semibold text-slate-200 hover:bg-slate-700 disabled:opacity-50 transition-all"
          >
            {t('backup.export_csv_btn')}
          </button>
        </div>

        {/* Export Markdown */}
        <div className="flex flex-col justify-between rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <div>
            <div className="flex items-center gap-2 text-blue-400 font-bold text-sm mb-2">
              <FileText className="w-4 h-4" />
              <span>{t('backup.export_md_title')}</span>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed mb-4">
              {t('backup.export_md_desc')}
            </p>
          </div>
          <button
            onClick={handleExportMarkdown}
            disabled={loading}
            className="w-full rounded-xl border border-slate-700 bg-slate-800 px-4 py-2.5 text-xs font-semibold text-slate-200 hover:bg-slate-700 disabled:opacity-50 transition-all"
          >
            {t('backup.export_md_btn')}
          </button>
        </div>
      </div>
    </div>
  );
};

