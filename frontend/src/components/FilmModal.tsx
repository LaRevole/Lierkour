import React, { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { Film, NewFilm, MovieMetadata, MediaType } from '../types';
import { isSerialCategory } from '../utils';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  X,
  Sparkles,
  Image as ImageIcon,
  ClipboardPaste,
  Film as FilmIcon,
  Gamepad2,
  Clock,
  Layers,
} from 'lucide-react';

interface FilmModalProps {
  isOpen: boolean;
  filmToEdit?: Film | null;
  parentFilmForSeason?: { parent: Film; seasonNumber: number } | null;
  initialMediaType?: MediaType;
  onClose: () => void;
  onSave: (film: NewFilm) => Promise<void>;
  categories: string[];
}

const PLATFORM_PRESETS = [
  'ПК',
  'PlayStation 5',
  'PlayStation 4',
  'Xbox Series',
  'Xbox One',
  'Switch',
  'Mobile',
];

const LAUNCHER_PRESETS = [
  'Steam',
  'GOG',
  'Epic Games',
  'Standalone',
  'Pirated / Repack',
  'Battle.net',
  'EA App',
];

const COMPLETION_STATUSES = [
  'В процессе',
  'Пройдена',
  '100% достижений',
  'Брошена',
];

export const FilmModal: React.FC<FilmModalProps> = ({
  isOpen,
  filmToEdit,
  parentFilmForSeason,
  initialMediaType = 'movie',
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
    category: initialMediaType === 'game' ? 'Игры' : (categories[0] || 'Фильмы'),
    director: '',
    year: 2026,
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
    media_type: initialMediaType,
    platform: '',
    launcher: '',
    playtime_hours: null,
    completion_status: 'В процессе',
  });

  const [quotesText, setQuotesText] = useState('');

  useEffect(() => {
    if (!isOpen) return;

    if (filmToEdit) {
      setFormData({
        title: filmToEdit.title || '',
        original_title: filmToEdit.original_title || '',
        category: filmToEdit.category || (filmToEdit.media_type === 'game' ? 'Игры' : 'Фильмы'),
        director: filmToEdit.director || '',
        year: filmToEdit.year ?? null,
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
        media_type: filmToEdit.media_type || 'movie',
        platform: filmToEdit.platform || '',
        launcher: filmToEdit.launcher || '',
        playtime_hours: filmToEdit.playtime_hours ?? null,
        completion_status: filmToEdit.completion_status || 'В процессе',
      });
      setQuotesText((filmToEdit.quotes || []).join('\n'));
    } else if (parentFilmForSeason && parentFilmForSeason.parent) {
      const parent = parentFilmForSeason.parent;
      const sNum = parentFilmForSeason.seasonNumber || 1;
      setFormData({
        title: `${parent.title || 'Сериал'} - Сезон ${sNum}`,
        original_title: parent.original_title ? `${parent.original_title} Season ${sNum}` : '',
        category: parent.category || 'Сериалы',
        director: parent.director || '',
        year: parent.year ?? null,
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
        media_type: 'movie',
        platform: '',
        launcher: '',
        playtime_hours: null,
        completion_status: '',
      });
      setQuotesText('');
    } else {
      const isGame = initialMediaType === 'game';
      setFormData({
        title: '',
        original_title: '',
        category: isGame ? 'Игры' : (categories[0] || 'Фильмы'),
        director: '',
        year: null,
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
        media_type: initialMediaType,
        platform: isGame ? 'ПК' : '',
        launcher: isGame ? 'Steam' : '',
        playtime_hours: null,
        completion_status: isGame ? 'В процессе' : '',
      });
      setQuotesText('');
    }
    setOmdbError(null);
    setSaveError(null);
  }, [filmToEdit, parentFilmForSeason, categories, initialMediaType, isOpen]);

  if (!isOpen) return null;

  const isGame = formData.media_type === 'game';

  const handleFetchOmdb = async () => {
    const query = formData.original_title.trim() || formData.title.trim();
    if (!query) return;

    setFetchingOmdb(true);
    setOmdbError(null);
    try {
      const meta = await invoke<MovieMetadata>('fetch_omdb_metadata', {
        title: query,
        year: formData.year || null,
      });

      let importedPosterPath = formData.poster_path;
      if (meta.poster_url) {
        try {
          importedPosterPath = await invoke<string>('import_poster_from_url', {
            url: meta.poster_url,
          });
        } catch (posterErr) {
          console.warn('Failed to download OMDb poster:', posterErr);
        }
      }

      setFormData((prev) => ({
        ...prev,
        title: prev.title || meta.title,
        original_title: meta.title,
        director: meta.director || prev.director,
        genres: meta.genres || prev.genres,
        country: meta.country || prev.country,
        year: meta.year || prev.year,
        runtime_minutes: meta.runtime_minutes || prev.runtime_minutes,
        description: meta.description || prev.description,
        imdb_id: meta.imdb_id || prev.imdb_id,
        seasons_total: meta.total_seasons || prev.seasons_total,
        poster_path: importedPosterPath,
      }));
    } catch (err: unknown) {
      console.error(err);
      setOmdbError(typeof err === 'string' ? err : 'Ошибка OMDb API');
    } finally {
      setFetchingOmdb(false);
    }
  };

  const handlePickPoster = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (selected && typeof selected === 'string') {
        const stored = await invoke<string>('import_poster_from_file', { sourcePath: selected });
        setFormData((prev) => ({ ...prev, poster_path: stored }));
      }
    } catch (err) {
      console.error(err);
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
              if (b64) {
                const stored = await invoke<string>('import_poster_from_base64', {
                  base64Data: b64,
                });
                setFormData((prev) => ({ ...prev, poster_path: stored }));
              }
            };
            reader.readAsDataURL(blob);
            return;
          }
        }
      }
      const stored = await invoke<string>('import_poster_from_clipboard');
      setFormData((prev) => ({ ...prev, poster_path: stored }));
    } catch (err) {
      console.error(err);
      alert('Не удалось вставить изображение из буфера обмена.');
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.title.trim()) return;

    setLoading(true);
    setSaveError(null);
    try {
      const parsedQuotes = quotesText
        .split('\n')
        .map((q) => q.trim())
        .filter((q) => q.length > 0);

      await onSave({
        ...formData,
        quotes: parsedQuotes,
        director: isGame ? '' : formData.director,
        runtime_minutes: isGame ? null : formData.runtime_minutes,
        seasons_total: isGame ? 0 : formData.seasons_total,
        platform: isGame ? (formData.platform?.trim() || null) : null,
        launcher: isGame ? (formData.launcher?.trim() || null) : null,
        playtime_hours: isGame ? (formData.playtime_hours ?? null) : null,
        completion_status: isGame ? (formData.completion_status?.trim() || null) : null,
      });
      onClose();
    } catch (err: unknown) {
      console.error(err);
      const msg = typeof err === 'string' ? err : (err as Error)?.message || 'Не удалось сохранить запись';
      setSaveError(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 overflow-y-auto">
      <div className="w-full max-w-2xl rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-2xl animate-in fade-in zoom-in-95 duration-200 my-auto text-slate-900 dark:text-slate-100">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-200 dark:border-slate-800 pb-4">
          <div>
            <h2 className="text-lg font-bold">
              {filmToEdit
                ? isGame
                  ? t('modal.game_edit_title')
                  : t('modal.edit_title')
                : parentFilmForSeason
                ? `Новый сезон (${parentFilmForSeason.parent?.title || ''})`
                : isGame
                ? t('modal.game_add_title')
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

        {/* Media Type Switcher (only for root items) */}
        {!parentFilmForSeason && (
          <div className="mt-4 flex items-center gap-2 p-1 rounded-xl bg-slate-100 dark:bg-slate-950 border border-slate-200 dark:border-slate-800">
            <button
              type="button"
              onClick={() => {
                setFormData((prev) => ({
                  ...prev,
                  media_type: 'movie',
                  category: prev.category === 'Игры' ? (categories[0] || 'Фильмы') : prev.category,
                }));
              }}
              className={`flex-1 flex items-center justify-center gap-2 py-2 rounded-lg text-xs font-bold transition-all ${
                !isGame
                  ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/30'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
              }`}
            >
              <FilmIcon className="w-4 h-4" />
              <span>{t('modal.media_movie')}</span>
            </button>
            <button
              type="button"
              onClick={() => {
                setFormData((prev) => ({
                  ...prev,
                  media_type: 'game',
                  category: 'Игры',
                  platform: prev.platform || 'ПК',
                  launcher: prev.launcher || 'Steam',
                  completion_status: prev.completion_status || 'В процессе',
                }));
              }}
              className={`flex-1 flex items-center justify-center gap-2 py-2 rounded-lg text-xs font-bold transition-all ${
                isGame
                  ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/30'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
              }`}
            >
              <Gamepad2 className="w-4 h-4" />
              <span>{t('modal.media_game')}</span>
            </button>
          </div>
        )}

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
          {/* Titles & (if movie) OMDb button */}
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
                placeholder={isGame ? t('modal.game_title_placeholder') : t('modal.title_placeholder')}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                  {t('modal.original_title_label')}
                </label>
                {!isGame && (
                  <button
                    type="button"
                    onClick={handleFetchOmdb}
                    disabled={fetchingOmdb || (!(formData.original_title || '').trim() && !(formData.title || '').trim())}
                    className="flex items-center gap-1 text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:underline disabled:opacity-40"
                  >
                    <Sparkles className="w-3 h-3" />
                    <span>{fetchingOmdb ? t('modal.fetching') : t('modal.fetch_omdb')}</span>
                  </button>
                )}
              </div>
              <input
                type="text"
                value={formData.original_title || ''}
                onChange={(e) => setFormData({ ...formData, original_title: e.target.value })}
                placeholder={isGame ? "The Witcher 3: Wild Hunt..." : "Original title for OMDb..."}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 focus:border-indigo-500 focus:outline-none"
              />
            </div>
          </div>

          {/* Row 2: Category, Status, Year */}
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
                {(isGame
                  ? Array.from(
                      new Set([
                        'Игры',
                        ...categories.filter(
                          (c) =>
                            c === 'Игры' ||
                            !['Фильмы', 'Сериалы', 'Мультфильмы', 'Аниме', 'Документальные'].includes(c)
                        ),
                      ])
                    )
                  : categories.filter((c) => c !== 'Игры')
                ).map((c) => (
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
                onChange={(e) => {
                  const newStatus = e.target.value as any;
                  setFormData((prev) => {
                    let updatedComp = prev.completion_status;
                    if (isGame) {
                      if (newStatus === 'watched') updatedComp = 'Пройдена';
                      else if (newStatus === 'dropped') updatedComp = 'Брошена';
                      else if (newStatus === 'watching') updatedComp = 'В процессе';
                    }
                    return { ...prev, status: newStatus, completion_status: updatedComp };
                  });
                }}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              >
                <option value="want_to_watch">
                  {isGame ? t('status.game_want_to_watch') : t('status.movie_want_to_watch')}
                </option>
                <option value="watching">
                  {isGame ? t('status.game_watching') : t('status.movie_watching')}
                </option>
                <option value="watched">
                  {isGame ? t('status.game_watched') : t('status.movie_watched')}
                </option>
                <option value="dropped">
                  {isGame ? t('status.game_dropped') : t('status.movie_dropped')}
                </option>
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
                placeholder="2026"
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>
          </div>

          {/* DYNAMIC FIELDS: GAME SPECIFIC vs MOVIE SPECIFIC */}
          {isGame ? (
            /* GAME FIELDS: Platform, Launcher, Playtime, Completion Status */
            <div className="space-y-4 p-4 rounded-2xl bg-indigo-50/50 dark:bg-indigo-950/20 border border-indigo-200/80 dark:border-indigo-900/50">
              <div className="flex items-center gap-2 text-xs font-bold text-indigo-600 dark:text-indigo-400">
                <Gamepad2 className="w-4 h-4" />
                <span>Параметры игры</span>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {/* Platform */}
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    {t('modal.platform_label')}
                  </label>
                  <input
                    type="text"
                    value={formData.platform || ''}
                    onChange={(e) => setFormData({ ...formData, platform: e.target.value })}
                    placeholder={t('modal.platform_placeholder')}
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                  />
                  <div className="flex flex-wrap gap-1 mt-1.5">
                    {PLATFORM_PRESETS.map((p) => (
                      <button
                        key={p}
                        type="button"
                        onClick={() => setFormData({ ...formData, platform: p })}
                        className={`text-[10px] px-2 py-0.5 rounded-md border transition-all ${
                          formData.platform === p
                            ? 'bg-indigo-600 text-white border-indigo-600 font-bold'
                            : 'bg-white dark:bg-slate-900 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-800 hover:border-indigo-400'
                        }`}
                      >
                        {p}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Launcher */}
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    {t('modal.launcher_label')}
                  </label>
                  <input
                    type="text"
                    value={formData.launcher || ''}
                    onChange={(e) => setFormData({ ...formData, launcher: e.target.value })}
                    placeholder={t('modal.launcher_placeholder')}
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                  />
                  <div className="flex flex-wrap gap-1 mt-1.5">
                    {LAUNCHER_PRESETS.map((l) => (
                      <button
                        key={l}
                        type="button"
                        onClick={() => setFormData({ ...formData, launcher: l })}
                        className={`text-[10px] px-2 py-0.5 rounded-md border transition-all ${
                          formData.launcher === l
                            ? 'bg-indigo-600 text-white border-indigo-600 font-bold'
                            : 'bg-white dark:bg-slate-900 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-800 hover:border-indigo-400'
                        }`}
                      >
                        {l}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-1">
                {/* Playtime Hours */}
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1 flex items-center gap-1.5">
                    <Clock className="w-3.5 h-3.5 text-indigo-500" />
                    <span>{t('modal.playtime_label')}</span>
                  </label>
                  <div className="relative">
                    <input
                      type="number"
                      min="0"
                      max="100000"
                      value={formData.playtime_hours !== null && formData.playtime_hours !== undefined ? formData.playtime_hours : ''}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          playtime_hours: e.target.value ? Math.max(0, parseInt(e.target.value)) : null,
                        })
                      }
                      placeholder="Например: 85"
                      className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 pr-10 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                    />
                    <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs font-semibold text-slate-400">
                      ч.
                    </span>
                  </div>
                </div>

                {/* Completion Status */}
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    {t('modal.completion_status_label')}
                  </label>
                  <select
                    value={formData.completion_status || 'В процессе'}
                    onChange={(e) => {
                      const val = e.target.value;
                      setFormData((prev) => {
                        let newStatus = prev.status;
                        if (val === 'Пройдена' || val === '100% достижений') newStatus = 'watched';
                        else if (val === 'Брошена') newStatus = 'dropped';
                        else if (val === 'В процессе') newStatus = 'watching';
                        return { ...prev, completion_status: val, status: newStatus };
                      });
                    }}
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                  >
                    {COMPLETION_STATUSES.map((status) => (
                      <option key={status} value={status}>
                        {status}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Developer / Publisher */}
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    {t('modal.game_country_label')}
                  </label>
                  <input
                    type="text"
                    value={formData.country || ''}
                    onChange={(e) => setFormData({ ...formData, country: e.target.value })}
                    placeholder="CD Projekt Red..."
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                  />
                </div>
              </div>
            </div>
          ) : (
            /* MOVIE FIELDS: Director, Runtime, Seasons */
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  {t('modal.director_label')}
                </label>
                <input
                  type="text"
                  value={formData.director || ''}
                  onChange={(e) => setFormData({ ...formData, director: e.target.value })}
                  placeholder="Режиссёр..."
                  className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                  {t('modal.runtime_label')}
                </label>
                <input
                  type="number"
                  min="1"
                  value={formData.runtime_minutes || ''}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      runtime_minutes: e.target.value ? parseInt(e.target.value) : null,
                    })
                  }
                  placeholder="Минут..."
                  className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                />
              </div>

              {isSerialCategory(formData.category) ? (
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1 flex items-center gap-1">
                    <Layers className="w-3.5 h-3.5 text-indigo-500" />
                    <span>{t('modal.seasons_label')}</span>
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
              ) : (
                <div>
                  <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                    {t('modal.country_label')}
                  </label>
                  <input
                    type="text"
                    value={formData.country || ''}
                    onChange={(e) => setFormData({ ...formData, country: e.target.value })}
                    placeholder="Страна..."
                    className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
                  />
                </div>
              )}
            </div>
          )}

          {/* Common metadata row: Rating, Genres, Country/Publisher */}
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
                placeholder="1 - 10"
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
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
                placeholder={isGame ? "RPG, Action, Открытый мир..." : "Фантастика, Драма..."}
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3.5 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
                {t('modal.tags_label')}
              </label>
              <input
                type="text"
                value={formData.tags || ''}
                onChange={(e) => setFormData({ ...formData, tags: e.target.value })}
                placeholder="шедевр, любимое..."
                className="w-full rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 focus:border-indigo-500 focus:outline-none"
              />
            </div>
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
              {isGame ? t('modal.game_quotes_label') : t('modal.quotes_label')}
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
              className="rounded-xl bg-indigo-600 px-5 py-2 text-sm font-semibold text-white shadow-lg shadow-indigo-600/30 hover:bg-indigo-500 disabled:opacity-50 transition-all cursor-pointer"
            >
              {loading ? t('modal.saving') : t('modal.save')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
