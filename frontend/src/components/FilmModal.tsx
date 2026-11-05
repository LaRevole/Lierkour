import React, { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { Film, NewFilm, MovieMetadata } from '../types';
import { isSerialCategory } from '../utils';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { X, Sparkles, Image as ImageIcon, ClipboardPaste } from 'lucide-react';

interface FilmModalProps {
  isOpen: boolean;
  filmToEdit?: Film | null;
  parentFilmForSeason?: { parent: Film; seasonNumber: number } | null;
  onClose: () => void;
  onSave: (film: NewFilm) => Promise<void>;
  categories: string[];
}

export const FilmModal: React.FC<FilmModalProps> = ({
  isOpen,
  filmToEdit,
  parentFilmForSeason,
  onClose,
  onSave,
  categories,
}) => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [fetchingOmdb, setFetchingOmdb] = useState(false);
  const [omdbError, setOmdbError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const [formData, setFormData] = useState<NewFilm>({
    title: '',
    original_title: '',
    category: categories[0] || 'Сериалы',
    director: '',
    year: new Date().getFullYear(),
    genres: '',
    country: '',
    runtime_minutes: null,
    imdb_id: '',
    tags: '',
    status: 'want_to_watch',
    rating: null,
    description: '',
    thoughts: '',
    poster_path: null,
    seasons_total: 0,
    watched_seasons: [],
    quotes: [],
    parent_id: null,
  });

  const [quotesText, setQuotesText] = useState('');

  useEffect(() => {
    if (filmToEdit) {
      setFormData({
        title: filmToEdit.title || '',
        original_title: filmToEdit.original_title || '',
        category: filmToEdit.category || 'Фильмы',
        director: filmToEdit.director || '',
        year: filmToEdit.year || new Date().getFullYear(),
        genres: filmToEdit.genres || '',
        country: filmToEdit.country || '',
        runtime_minutes: filmToEdit.runtime_minutes ?? null,
        imdb_id: filmToEdit.imdb_id || '',
        tags: filmToEdit.tags || '',
        status: filmToEdit.status || 'want_to_watch',
        rating: filmToEdit.rating ?? null,
        description: filmToEdit.description || '',
        thoughts: filmToEdit.thoughts || '',
        poster_path: filmToEdit.poster_path || null,
        seasons_total: filmToEdit.seasons_total || 0,
        watched_seasons: filmToEdit.watched_seasons || [],
        quotes: filmToEdit.quotes || [],
        parent_id: filmToEdit.parent_id || null,
      });
      setQuotesText((filmToEdit.quotes || []).join('\n'));
    } else if (parentFilmForSeason && parentFilmForSeason.parent) {
      // Предзаполнение для добавления сезона
      const parent = parentFilmForSeason.parent;
      const sNum = parentFilmForSeason.seasonNumber || 1;
      setFormData({
        title: `${parent.title || 'Сериал'} - Сезон ${sNum}`,
        original_title: parent.original_title ? `${parent.original_title} Season ${sNum}` : '',
        category: parent.category || 'Сериалы',
        director: parent.director || '',
        year: parent.year || new Date().getFullYear(),
        genres: parent.genres || '',
        country: parent.country || '',
        runtime_minutes: null,
        imdb_id: '',
        tags: parent.tags || '',
        status: 'want_to_watch',
        rating: null,
        description: '',
        thoughts: '',
        poster_path: parent.poster_path || null,
        seasons_total: 1,
        watched_seasons: [],
        quotes: [],
        parent_id: parent.id,
      });
      setQuotesText('');
    } else {
      setFormData({
        title: '',
        original_title: '',
        category: categories[0] || 'Фильмы',
        director: '',
        year: new Date().getFullYear(),
        genres: '',
        country: '',
        runtime_minutes: null,
        imdb_id: '',
        tags: '',
        status: 'want_to_watch',
        rating: null,
        description: '',
        thoughts: '',
        poster_path: null,
        seasons_total: 0,
        watched_seasons: [],
        quotes: [],
        parent_id: null,
      });
      setQuotesText('');
    }
    setOmdbError(null);
    setSaveError(null);
  }, [filmToEdit, parentFilmForSeason, isOpen, categories]);

  const importBase64Poster = async (base64Data: string) => {
    try {
      const stored = await invoke<string>('import_poster_from_base64', { base64Data });
      setFormData((prev) => ({ ...prev, poster_path: stored }));
    } catch (err: any) {
      console.error('Failed to import poster from base64:', err);
      alert('Ошибка при импорте изображения: ' + (err?.toString() || 'Неизвестная ошибка'));
    }
  };

  const handlePickPoster = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
      });
      if (selected && typeof selected === 'string') {
        const stored = await invoke<string>('import_poster_file', { sourcePath: selected });
        setFormData((prev) => ({ ...prev, poster_path: stored }));
      }
    } catch (err: any) {
      console.error('Failed to pick poster:', err);
    }
  };

  const handlePasteFromClipboard = async () => {
    try {
      if (navigator.clipboard && navigator.clipboard.read) {
        const items = await navigator.clipboard.read();
        for (const item of items) {
          const imageType = item.types.find((t) => t.startsWith('image/'));
          if (imageType) {
            const blob = await item.getType(imageType);
            const reader = new FileReader();
            reader.onload = async (ev) => {
              const b64 = ev.target?.result as string;
              if (b64) await importBase64Poster(b64);
            };
            reader.readAsDataURL(blob);
            return;
          }
        }
      }

      if (navigator.clipboard && navigator.clipboard.readText) {
        const text = await navigator.clipboard.readText();
        if (text.startsWith('data:image/')) {
          await importBase64Poster(text);
          return;
        }
      }

      alert('В буфере обмена нет изображения. Скопируйте изображение (например, Правый клик -> Скопировать картинку) и повторите.');
    } catch (err) {
      console.warn('Clipboard read failed:', err);
      alert('Не удалось прочитать буфер обмена напрямую. Нажмите Ctrl+V на этой странице.');
    }
  };

  // Paste event listener for Ctrl+V
  useEffect(() => {
    if (!isOpen) return;

    const handlePaste = (e: ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;

      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        if (item.type.startsWith('image/')) {
          e.preventDefault();
          const file = item.getAsFile();
          if (file) {
            const reader = new FileReader();
            reader.onload = async (ev) => {
              const b64 = ev.target?.result as string;
              if (b64) await importBase64Poster(b64);
            };
            reader.readAsDataURL(file);
            return;
          }
        }
      }
    };

    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
  }, [isOpen]);

  const handleFetchOmdb = async () => {
    const query = formData.original_title.trim() || formData.title.trim();
    if (!query) return;

    setFetchingOmdb(true);
    setOmdbError(null);
    try {
      const meta = await invoke<MovieMetadata>('fetch_omdb_metadata', {
        title: query,
        year: formData.year,
      });

      setFormData((prev) => ({
        ...prev,
        title: prev.title.trim() ? prev.title : meta.title,
        original_title: prev.original_title.trim() ? prev.original_title : meta.title,
        director: meta.director || prev.director,
        year: meta.year ?? prev.year,
        genres: meta.genres || prev.genres,
        country: meta.country || prev.country,
        runtime_minutes: meta.runtime_minutes ?? prev.runtime_minutes,
        description: meta.description || prev.description,
        imdb_id: meta.imdb_id || prev.imdb_id,
        seasons_total: meta.total_seasons || prev.seasons_total,
      }));
    } catch (err: any) {
      setOmdbError(err?.toString() || 'Failed to fetch OMDb metadata');
    } finally {
      setFetchingOmdb(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!(formData.title || '').trim()) return;

    setLoading(true);
    setSaveError(null);
    try {
      const quotes = quotesText
        .split('\n')
        .map((q) => q.trim())
        .filter((q) => q.length > 0);

      await onSave({
        ...formData,
        quotes,
      });
      onClose();
    } catch (err: any) {
      console.error('Save film error:', err);
      setSaveError(err?.toString() || 'Ошибка сохранения фильма');
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 overflow-y-auto">
      <div className="w-full max-w-2xl rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-2xl animate-in fade-in zoom-in-95 duration-200 my-auto text-slate-900 dark:text-slate-100">
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 pb-4">
          <div>
            <h2 className="text-lg font-bold">
              {filmToEdit
                ? t('modal.edit_title')
                : parentFilmForSeason
                ? `Новый сезон (${parentFilmForSeason.parent?.title || ''})`
                : t('modal.add_title')}
            </h2>
            {parentFilmForSeason && (
              <p className="text-xs text-indigo-500 font-semibold">
                Привязывается к колоде сериала
              </p>
            )}
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1.5 text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 hover:text-slate-700 dark:hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {omdbError && (
          <div className="mt-3 p-3 text-xs bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-600 dark:text-rose-300">
            {omdbError}
          </div>
        )}

        {saveError && (
          <div className="mt-3 p-3 text-xs font-semibold bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-600 dark:text-rose-300">
            ⚠️ Ошибка: {saveError}
          </div>
        )}

        <form onSubmit={handleSubmit} className="mt-4 space-y-4 max-h-[75vh] overflow-y-auto pr-1">
          {/* Titles & OMDb button */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.title_label')}
              </label>
              <input
                type="text"
                required
                value={formData.title || ''}
                onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                placeholder={t('modal.title_placeholder')}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                  {t('modal.original_title_label')}
                </label>
                <button
                  type="button"
                  onClick={handleFetchOmdb}
                  disabled={fetchingOmdb || (!(formData.original_title || '').trim() && !(formData.title || '').trim())}
                  className="flex items-center gap-1 text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:underline disabled:opacity-40"
                >
                  <Sparkles className="w-3 h-3" />
                  <span>{fetchingOmdb ? t('modal.fetching') : t('modal.fetch_omdb')}</span>
                </button>
              </div>
              <input
                type="text"
                value={formData.original_title || ''}
                onChange={(e) => setFormData({ ...formData, original_title: e.target.value })}
                placeholder="Original title for OMDb..."
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none"
              />
            </div>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.category_label')}
              </label>
              <select
                value={formData.category}
                onChange={(e) => {
                  const newCategory = e.target.value;
                  setFormData((prev) => ({
                    ...prev,
                    category: newCategory,
                    seasons_total: isSerialCategory(newCategory) ? prev.seasons_total : 0,
                  }));
                }}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              >
                {categories.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.status_label')}
              </label>
              <select
                value={formData.status}
                onChange={(e) => setFormData({ ...formData, status: e.target.value as any })}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              >
                <option value="want_to_watch">{t('status.want_to_watch')}</option>
                <option value="watching">{t('status.watching')}</option>
                <option value="watched">{t('status.watched')}</option>
                <option value="dropped">{t('status.dropped')}</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.year_label')}
              </label>
              <input
                type="number"
                min="1888"
                max="2100"
                value={formData.year || ''}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    year: e.target.value ? parseInt(e.target.value) : null,
                  })
                }
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.rating_label')}
              </label>
              <input
                type="number"
                min="1"
                max="10"
                value={formData.rating || ''}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    rating: e.target.value ? parseInt(e.target.value) : null,
                  })
                }
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.director_label')}
              </label>
              <input
                type="text"
                value={formData.director || ''}
                onChange={(e) => setFormData({ ...formData, director: e.target.value })}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.genres_label')}
              </label>
              <input
                type="text"
                value={formData.genres || ''}
                onChange={(e) => setFormData({ ...formData, genres: e.target.value })}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>
          </div>

          <div className={`grid ${isSerialCategory(formData.category) ? 'grid-cols-3' : 'grid-cols-2'} gap-3`}>
            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.country_label')}
              </label>
              <input
                type="text"
                value={formData.country || ''}
                onChange={(e) => setFormData({ ...formData, country: e.target.value })}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.runtime_label')}
              </label>
              <input
                type="number"
                value={formData.runtime_minutes || ''}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    runtime_minutes: e.target.value ? parseInt(e.target.value) : null,
                  })
                }
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            {isSerialCategory(formData.category) && (
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  {t('modal.seasons_label')}
                </label>
                <input
                  type="number"
                  min="0"
                  value={formData.seasons_total || ''}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      seasons_total: e.target.value ? parseInt(e.target.value) : 0,
                    })
                  }
                  className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                />
              </div>
            )}
          </div>

          {/* Description & Thoughts */}
          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              {t('modal.description_label')}
            </label>
            <textarea
              rows={2}
              value={formData.description || ''}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              {t('modal.thoughts_label')}
            </label>
            <textarea
              rows={2}
              value={formData.thoughts || ''}
              onChange={(e) => setFormData({ ...formData, thoughts: e.target.value })}
              className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              {t('modal.quotes_label')}
            </label>
            <textarea
              rows={2}
              value={quotesText || ''}
              onChange={(e) => setQuotesText(e.target.value)}
              className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none font-mono text-xs"
            />
          </div>

          {/* Poster Picker & Clipboard Paste */}
          <div className="flex flex-wrap items-center gap-2 pt-2">
            <button
              type="button"
              onClick={handlePickPoster}
              className="flex items-center gap-1.5 rounded-xl border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800/80 px-3.5 py-2 text-xs font-medium text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
            >
              <ImageIcon className="w-4 h-4 text-indigo-500" />
              <span>{t('modal.choose_poster')}</span>
            </button>

            <button
              type="button"
              onClick={handlePasteFromClipboard}
              title="Вставить скопированную обложку из буфера обмена (Ctrl+V)"
              className="flex items-center gap-1.5 rounded-xl border border-indigo-300 dark:border-indigo-700/60 bg-indigo-50/70 dark:bg-indigo-950/40 px-3.5 py-2 text-xs font-medium text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-900/60 transition-colors"
            >
              <ClipboardPaste className="w-4 h-4" />
              <span>Вставить из буфера</span>
            </button>

            {formData.poster_path && (
              <div className="flex items-center gap-2 ml-auto">
                <span className="text-xs text-emerald-600 dark:text-emerald-400 font-medium truncate max-w-[180px]">
                  ✓ {formData.poster_path}
                </span>
                <button
                  type="button"
                  onClick={() => setFormData((prev) => ({ ...prev, poster_path: null }))}
                  className="text-xs text-slate-400 hover:text-rose-500 transition-colors underline"
                >
                  Удалить
                </button>
              </div>
            )}
          </div>

          <div className="mt-6 flex justify-end gap-3 pt-4 border-t border-slate-200 dark:border-slate-800">
            <button
              type="button"
              onClick={onClose}
              className="rounded-xl px-4 py-2 text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
            >
              {t('modal.cancel')}
            </button>
            <button
              type="submit"
              disabled={loading || !(formData.title || '').trim()}
              className="rounded-xl bg-indigo-600 px-5 py-2 text-sm font-semibold text-white shadow-lg shadow-indigo-600/30 hover:bg-indigo-500 disabled:opacity-50 transition-all"
            >
              {loading ? t('modal.saving') : t('modal.save')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
