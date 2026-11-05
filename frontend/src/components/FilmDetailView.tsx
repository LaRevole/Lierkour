import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { Film, RelatedFilm, FilmScreenshot, RelationKind } from '../types';
import { isSerialCategory } from '../utils';
import {
  ArrowLeft,
  Edit,
  Trash2,
  Star,
  Clock,
  Film as FilmIcon,
  Image as ImageIcon,
  Plus,
  Trash,
  ExternalLink,
  Quote,
  Calendar,
  Layers,
  Unlink,
  ClipboardPaste,
  Gamepad2,
} from 'lucide-react';

type RelationChoice =
  | 'sequel_to'
  | 'prequel_to'
  | 'has_sequel'
  | 'has_prequel'
  | 'remake_of'
  | 'original_of'
  | 'spin_off_to'
  | 'has_spin_off'
  | 'related';

interface FilmDetailViewProps {
  film: Film;
  allFilms: Film[];
  onBack: () => void;
  onEdit: (film: Film) => void;
  onDelete: (id: number) => void;
  onAddSeason: (parentFilm: Film, nextSeasonNumber: number) => void;
  onOpenFilm: (film: Film) => void;
  onRefresh: () => void;
  refreshTrigger?: number;
}

export const FilmDetailView: React.FC<FilmDetailViewProps> = ({
  film,
  allFilms,
  onBack,
  onEdit,
  onDelete,
  onAddSeason,
  onOpenFilm,
  onRefresh,
  refreshTrigger,
}) => {
  const { t } = useTranslation();
  const [posterSrc, setPosterSrc] = useState<string | null>(null);
  const [relations, setRelations] = useState<RelatedFilm[]>([]);
  const [screenshots, setScreenshots] = useState<FilmScreenshot[]>([]);
  const [screenshotSrcs, setScreenshotSrcs] = useState<Record<number, string>>({});
  const [seasons, setSeasons] = useState<Film[]>([]);
  const [seasonPosters, setSeasonPosters] = useState<Record<number, string>>({});

  const [relationPosters, setRelationPosters] = useState<Record<number, string>>({});

  // Add Relation state
  const [isAddingRelation, setIsAddingRelation] = useState(false);
  const [targetFilmId, setTargetFilmId] = useState<number | ''>('');
  const [relationChoice, setRelationChoice] = useState<RelationChoice>('sequel_to');

  // Load Poster
  useEffect(() => {
    let isMounted = true;
    if (film.poster_path) {
      invoke<string>('resolve_poster_asset', { storedPath: film.poster_path })
        .then((src) => {
          if (isMounted) setPosterSrc(src);
        })
        .catch(() => {
          if (isMounted) setPosterSrc(null);
        });
    } else {
      setPosterSrc(null);
    }
    return () => {
      isMounted = false;
    };
  }, [film.poster_path]);

  // Load Relations, Screenshots & Seasons
  const loadDetails = () => {
    invoke<RelatedFilm[]>('get_film_relations', { filmId: film.id })
      .then((rels) => {
        setRelations(rels);
        rels.forEach((rel) => {
          if (rel.film.poster_path) {
            invoke<string>('resolve_poster_asset', { storedPath: rel.film.poster_path })
              .then((src) => {
                setRelationPosters((prev) => ({ ...prev, [rel.film.id]: src }));
              })
              .catch(() => {});
          }
        });
      })
      .catch((err) => console.error('Failed to load relations:', err));

    invoke<FilmScreenshot[]>('get_film_screenshots', { filmId: film.id })
      .then((shots) => {
        setScreenshots(shots);
        shots.forEach((shot) => {
          invoke<string>('resolve_screenshot_asset', { storedPath: shot.image_path })
            .then((src) => {
              setScreenshotSrcs((prev) => ({ ...prev, [shot.id]: src }));
            })
            .catch((err) => console.error(err));
        });
      })
      .catch((err) => console.error('Failed to load screenshots:', err));

    // If it's a parent film, load child seasons
    if (!film.parent_id) {
      invoke<Film[]>('get_seasons', { parentId: film.id })
        .then((childSeasons) => {
          setSeasons(childSeasons);
          childSeasons.forEach((season) => {
            if (season.poster_path) {
              invoke<string>('resolve_poster_asset', { storedPath: season.poster_path })
                .then((src) => {
                  setSeasonPosters((prev) => ({ ...prev, [season.id]: src }));
                })
                .catch(() => {});
            }
          });
        })
        .catch(() => setSeasons([]));
    }
  };

  useEffect(() => {
    loadDetails();
  }, [film.id, refreshTrigger]);

  const handleDetachSeason = async () => {
    if (!film.parent_id) return;
    try {
      await invoke('detach_season', { seasonId: film.id });
      onRefresh();
      onBack();
    } catch (err) {
      console.error('Failed to detach season:', err);
    }
  };

  const handlePastePosterInDetail = async () => {
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
                const stored = await invoke<string>('import_poster_from_base64', { base64Data: b64 });
                await invoke('update_film', {
                  id: film.id,
                  film: { ...film, poster_path: stored },
                });
                onRefresh();
              }
            };
            reader.readAsDataURL(blob);
            return;
          }
        }
      }
      alert('В буфере обмена нет изображения. Скопируйте изображение и повторите.');
    } catch (err) {
      console.error(err);
      alert('Не удалось прочитать буфер обмена. Вы можете нажать Ctrl+V для вставки.');
    }
  };

  // Paste event listener for detail view
  useEffect(() => {
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
              if (b64) {
                try {
                  const stored = await invoke<string>('import_poster_from_base64', { base64Data: b64 });
                  await invoke('update_film', {
                    id: film.id,
                    film: { ...film, poster_path: stored },
                  });
                  onRefresh();
                } catch (err) {
                  console.error(err);
                }
              }
            };
            reader.readAsDataURL(file);
            return;
          }
        }
      }
    };
    window.addEventListener('paste', handlePaste);
    return () => window.removeEventListener('paste', handlePaste);
  }, [film.id]);

  const handleAddRelation = async () => {
    if (!targetFilmId) return;
    const targetId = Number(targetFilmId);

    let sourceId = film.id;
    let actualTargetId = targetId;
    let kind: RelationKind = 'related';

    switch (relationChoice) {
      case 'sequel_to':
        sourceId = film.id;
        actualTargetId = targetId;
        kind = 'sequel';
        break;
      case 'has_sequel':
        sourceId = targetId;
        actualTargetId = film.id;
        kind = 'sequel';
        break;
      case 'prequel_to':
        sourceId = film.id;
        actualTargetId = targetId;
        kind = 'prequel';
        break;
      case 'has_prequel':
        sourceId = targetId;
        actualTargetId = film.id;
        kind = 'prequel';
        break;
      case 'remake_of':
        sourceId = film.id;
        actualTargetId = targetId;
        kind = 'remake';
        break;
      case 'original_of':
        sourceId = targetId;
        actualTargetId = film.id;
        kind = 'remake';
        break;
      case 'spin_off_to':
        sourceId = film.id;
        actualTargetId = targetId;
        kind = 'spin_off';
        break;
      case 'has_spin_off':
        sourceId = targetId;
        actualTargetId = film.id;
        kind = 'spin_off';
        break;
      case 'related':
      default:
        sourceId = film.id;
        actualTargetId = targetId;
        kind = 'related';
        break;
    }

    try {
      await invoke('add_film_relation', {
        sourceId,
        targetId: actualTargetId,
        kind,
      });
      setIsAddingRelation(false);
      setTargetFilmId('');
      loadDetails();
    } catch (err) {
      console.error(err);
    }
  };

  const handleRemoveRelation = async (relationId: number) => {
    try {
      await invoke('remove_film_relation', { relationId });
      loadDetails();
    } catch (err) {
      console.error(err);
    }
  };

  const handleAddScreenshot = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (selected && typeof selected === 'string') {
        const caption = prompt('Подпись к кадру (необязательно):') || '';
        await invoke('add_film_screenshot', {
          filmId: film.id,
          sourcePath: selected,
          caption,
        });
        loadDetails();
      }
    } catch (err) {
      console.error(err);
    }
  };

  const handleDeleteScreenshot = async (id: number) => {
    try {
      await invoke('delete_film_screenshot', { screenshotId: id });
      loadDetails();
    } catch (err) {
      console.error(err);
    }
  };

  const isGame = film.media_type === 'game';

  const getRelationKindLabel = (kind: RelationKind, outgoing: boolean) => {
    if (isGame) {
      if (outgoing) {
        switch (kind) {
          case 'sequel':
            return 'Сиквел (продолжение) к:';
          case 'prequel':
            return 'Приквел (предыстория) к:';
          case 'remake':
            return 'Ремейк / Ремастер игры:';
          case 'spin_off':
            return 'Спин-офф (ответвление) от:';
          case 'related':
          default:
            return 'Связана с игрой:';
        }
      } else {
        switch (kind) {
          case 'sequel':
            return 'Следующая игра (сиквел):';
          case 'prequel':
            return 'Предыдущая игра (приквел):';
          case 'remake':
            return 'Ремейк / Ремастер этой игры:';
          case 'spin_off':
            return 'Спин-офф этой игры:';
          case 'related':
          default:
            return 'Связана с игрой:';
        }
      }
    } else {
      if (outgoing) {
        switch (kind) {
          case 'sequel':
            return 'Сиквел (продолжение) к:';
          case 'prequel':
            return 'Приквел (предыстория) к:';
          case 'remake':
            return 'Ремейк фильма:';
          case 'spin_off':
            return 'Спин-офф (ответвление) от:';
          case 'related':
          default:
            return 'Связан с фильмом:';
        }
      } else {
        switch (kind) {
          case 'sequel':
            return 'Следующая часть (сиквел):';
          case 'prequel':
            return 'Предыдущая часть (приквел):';
          case 'remake':
            return 'Ремейк этого фильма:';
          case 'spin_off':
            return 'Спин-офф этого фильма:';
          case 'related':
          default:
            return 'Связан с фильмом:';
        }
      }
    }
  };

  const availableFilmsForRelation = allFilms.filter(
    (f) =>
      f.id !== film.id &&
      (isGame ? f.media_type === 'game' : (f.media_type || 'movie') === 'movie') &&
      !relations.some((r) => r.film.id === f.id)
  );

  const parentFilm = film.parent_id
    ? allFilms.find((f) => f.id === film.parent_id)
    : null;

  return (
    <div className="space-y-6 animate-in fade-in duration-200 max-w-6xl mx-auto">
      {/* Top action bar */}
      <div className="flex items-center justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-2 rounded-xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-900 px-4 py-2 text-xs font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors shadow-sm"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>
            {parentFilm
              ? isGame
                ? `Назад к игре (${parentFilm.title})`
                : `Назад к сериалу (${parentFilm.title})`
              : 'Назад к библиотеке'}
          </span>
        </button>

        <div className="flex items-center gap-2">
          {/* If it's a child season, show Detach button */}
          {film.parent_id && (
            <button
              onClick={handleDetachSeason}
              title="Сделать самостоятельным тайтлом на главной"
              className="flex items-center gap-1.5 rounded-xl border border-amber-500/30 bg-amber-50 dark:bg-amber-500/10 px-4 py-2 text-xs font-semibold text-amber-600 dark:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-500/20 transition-colors shadow-sm"
            >
              <Unlink className="w-4 h-4" />
              <span>Отвязать от сериала</span>
            </button>
          )}

          {/* If it's a parent title AND it's a serial/anime category, show Add Season button */}
          {!isGame && !film.parent_id && isSerialCategory(film.category) && (
            <button
              onClick={() => onAddSeason(film, seasons.length + 1)}
              className="flex items-center gap-1.5 rounded-xl border border-indigo-500/40 bg-indigo-600 text-white px-4 py-2 text-xs font-semibold hover:bg-indigo-500 transition-colors shadow-sm"
            >
              <Plus className="w-4 h-4" />
              <span>+ Добавить сезон</span>
            </button>
          )}

          <button
            onClick={() => onEdit(film)}
            className="flex items-center gap-1.5 rounded-xl border border-indigo-500/30 bg-indigo-50 dark:bg-indigo-500/10 px-4 py-2 text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-500/20 transition-colors shadow-sm"
          >
            <Edit className="w-4 h-4" />
            <span>{isGame ? t('game.edit') : t('film.edit')}</span>
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (window.confirm(isGame ? t('game.delete_confirm') : t('film.delete_confirm'))) {
                onDelete(film.id);
              }
            }}
            className="flex items-center gap-1.5 rounded-xl border border-rose-300 dark:border-rose-500/30 bg-rose-50 dark:bg-rose-500/10 px-4 py-2 text-xs font-semibold text-rose-600 dark:text-rose-400 hover:bg-rose-100 dark:hover:bg-rose-500/20 transition-colors shadow-sm cursor-pointer"
          >
            <Trash2 className="w-4 h-4" />
            <span>{isGame ? t('game.delete') : t('film.delete')}</span>
          </button>
        </div>
      </div>

      {/* Main Film Info Section */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md shadow-sm">
        {/* Left: Poster */}
        <div className="flex flex-col gap-2.5">
          <div className="aspect-[2/3] w-full rounded-2xl bg-slate-100 dark:bg-slate-950 overflow-hidden relative border border-slate-200 dark:border-slate-800 shadow-md">
            {posterSrc ? (
              <img src={posterSrc} alt={film.title} className="h-full w-full object-cover" />
            ) : (
              <div className="flex h-full w-full flex-col items-center justify-center p-4 text-center text-slate-400 select-none">
                {isGame ? (
                  <Gamepad2 className="w-16 h-16 mb-2 opacity-30 text-indigo-500" />
                ) : (
                  <FilmIcon className="w-16 h-16 mb-2 opacity-30" />
                )}
                <span className="text-xs font-bold uppercase tracking-wider">{film.category || (isGame ? 'Игра' : 'Фильм')}</span>
              </div>
            )}
          </div>
          <button
            onClick={handlePastePosterInDetail}
            title="Вставить скопированную картинку из буфера обмена (Ctrl+V)"
            className="flex items-center justify-center gap-1.5 w-full rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900/80 px-3 py-2 text-xs font-semibold text-slate-700 dark:text-slate-300 hover:bg-indigo-50 dark:hover:bg-indigo-950/40 hover:text-indigo-600 dark:hover:text-indigo-400 hover:border-indigo-500/30 transition-all shadow-xs cursor-pointer"
          >
            <ClipboardPaste className="w-3.5 h-3.5 text-indigo-500" />
            <span>Вставить обложку из буфера</span>
          </button>
        </div>

        {/* Center & Right: Details */}
        <div className="md:col-span-2 flex flex-col justify-between space-y-6">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span className="rounded-lg bg-indigo-500/10 border border-indigo-500/20 px-2.5 py-1 text-xs font-bold text-indigo-600 dark:text-indigo-400">
                {film.category}
              </span>
              <span className="rounded-lg bg-slate-100 dark:bg-slate-800 px-2.5 py-1 text-xs font-semibold text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-slate-700">
                {isGame
                  ? film.completion_status || t(`status.game_${film.status}`)
                  : t(`status.movie_${film.status}`)}
              </span>
              {film.media_type === 'game' && film.platform && (
                <span className="rounded-lg bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1 text-xs font-bold text-emerald-600 dark:text-emerald-400">
                  {film.platform}
                </span>
              )}
              {film.parent_id && (
                <span className="rounded-lg bg-amber-500/10 border border-amber-500/20 px-2.5 py-1 text-xs font-bold text-amber-600 dark:text-amber-400">
                  Сезон сериала
                </span>
              )}
            </div>

            <h1 className="text-3xl font-extrabold text-slate-900 dark:text-slate-100 tracking-tight">
              {film.title}
            </h1>
            {film.original_title && (
              <p className="text-base font-medium text-slate-500 dark:text-slate-400 italic mt-0.5">
                {film.original_title}
              </p>
            )}

            {/* Badges / Quick stats */}
            <div className="flex flex-wrap items-center gap-4 mt-4 text-xs font-semibold text-slate-600 dark:text-slate-400 border-y border-slate-200 dark:border-slate-800/80 py-3">
              {film.media_type === 'game' ? (
                <>
                  {film.platform && (
                    <div className="flex items-center gap-1.5 text-indigo-600 dark:text-indigo-400">
                      <Gamepad2 className="w-4 h-4" />
                      <span>{film.platform}</span>
                    </div>
                  )}
                  {film.playtime_hours !== null && film.playtime_hours !== undefined && (
                    <div className="flex items-center gap-1.5">
                      <Clock className="w-4 h-4 text-indigo-500" />
                      <span>
                        {film.playtime_hours} {t('game.playtime_short')}
                      </span>
                    </div>
                  )}
                  {film.launcher && (
                    <div className="flex items-center gap-1.5 text-slate-700 dark:text-slate-300">
                      <span className="text-slate-400 font-normal">Лаунчер:</span>
                      <span>{film.launcher}</span>
                    </div>
                  )}
                </>
              ) : (
                <>
                  {film.year && (
                    <div className="flex items-center gap-1.5">
                      <Calendar className="w-4 h-4 text-indigo-500" />
                      <span>{film.year}</span>
                    </div>
                  )}
                  {film.runtime_minutes && (
                    <div className="flex items-center gap-1.5">
                      <Clock className="w-4 h-4 text-indigo-500" />
                      <span>
                        {film.runtime_minutes} {t('film.minutes_short')}
                      </span>
                    </div>
                  )}
                </>
              )}
              {film.rating !== null && film.rating !== undefined && (
                <div className="flex items-center gap-1.5 text-amber-500">
                  <Star className="w-4 h-4 fill-amber-400" />
                  <span className="font-bold text-sm">{film.rating} / 10</span>
                </div>
              )}
              {film.imdb_id && (
                <a
                  href={`https://www.imdb.com/title/${film.imdb_id}`}
                  target="_blank"
                  rel="noreferrer"
                  className="flex items-center gap-1 text-amber-600 dark:text-amber-400 hover:underline"
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                  <span>IMDb</span>
                </a>
              )}
            </div>

            {/* Info grid */}
            {film.media_type === 'game' ? (
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-4 mt-4 text-xs">
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('game.platform')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.platform || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('game.launcher')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.launcher || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('game.playtime')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.playtime_hours !== null && film.playtime_hours !== undefined
                      ? `${film.playtime_hours} ${t('game.playtime_short')}`
                      : '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('game.completion_status')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.completion_status || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.genres')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.genres || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.tags')}:</span>
                  <span className="font-semibold text-indigo-600 dark:text-indigo-400 text-sm">
                    {film.tags || '—'}
                  </span>
                </div>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-4 mt-4 text-xs">
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.director')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.director || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.genres')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.genres || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.country')}:</span>
                  <span className="font-semibold text-slate-800 dark:text-slate-200 text-sm">
                    {film.country || '—'}
                  </span>
                </div>
                <div>
                  <span className="text-slate-400 block mb-0.5">{t('film.tags')}:</span>
                  <span className="font-semibold text-indigo-600 dark:text-indigo-400 text-sm">
                    {film.tags || '—'}
                  </span>
                </div>
              </div>
            )}

            {/* Description */}
            {film.description && (
              <div className="mt-5">
                <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-1.5">
                  {t('film.description')}
                </h3>
                <p className="text-sm text-slate-700 dark:text-slate-300 leading-relaxed bg-slate-50 dark:bg-slate-950/40 p-4 rounded-2xl border border-slate-200 dark:border-slate-800/80">
                  {film.description}
                </p>
              </div>
            )}

            {/* Thoughts / Review */}
            {film.thoughts && (
              <div className="mt-4">
                <h3 className="text-xs font-bold uppercase tracking-wider text-indigo-500 mb-1.5">
                  {t('film.thoughts')}
                </h3>
                <p className="text-sm text-slate-700 dark:text-slate-300 leading-relaxed italic bg-indigo-50/50 dark:bg-indigo-950/20 p-4 rounded-2xl border border-indigo-200 dark:border-indigo-900/40">
                  "{film.thoughts}"
                </p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* SEASONS SECTION (Only for serial / anime films, never for games) */}
      {!isGame && !film.parent_id && isSerialCategory(film.category) && (
        <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2 text-slate-800 dark:text-slate-100 font-bold text-base">
              <Layers className="w-5 h-5 text-indigo-500" />
              <span>Сезоны тайтла ({seasons.length})</span>
            </div>
            <button
              onClick={() => onAddSeason(film, seasons.length + 1)}
              className="flex items-center gap-1.5 rounded-xl border border-indigo-500/30 bg-indigo-50 dark:bg-indigo-500/10 px-3 py-1.5 text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-500/20 transition-colors"
            >
              <Plus className="w-4 h-4" />
              <span>Добавить сезон</span>
            </button>
          </div>

          {seasons.length === 0 ? (
            <p className="text-xs text-slate-500 italic">
              У этого тайтла ещё нет отдельных сезонов. Вы можете добавить сезон отдельной карточкой!
            </p>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
              {seasons.map((season) => (
                <div
                  key={season.id}
                  onClick={() => onOpenFilm(season)}
                  className="group flex gap-3 p-3 rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/40 hover:border-indigo-500/50 transition-all cursor-pointer"
                >
                  <div className="w-16 h-24 rounded-xl bg-slate-200 dark:bg-slate-800 overflow-hidden shrink-0">
                    {seasonPosters[season.id] ? (
                      <img
                        src={seasonPosters[season.id]}
                        alt={season.title}
                        className="w-full h-full object-cover"
                      />
                    ) : (
                      <div className="w-full h-full flex items-center justify-center text-xs font-bold text-slate-400">
                        S
                      </div>
                    )}
                  </div>
                  <div className="flex-1 min-w-0 flex flex-col justify-between py-1">
                    <div>
                      <h4 className="text-xs font-bold text-slate-800 dark:text-slate-200 line-clamp-2 group-hover:text-indigo-600 dark:group-hover:text-indigo-400">
                        {season.title}
                      </h4>
                      <span className="text-[11px] text-slate-400">{season.year || '—'}</span>
                    </div>
                    {season.rating && (
                      <div className="flex items-center gap-1 text-amber-500 text-xs font-bold">
                        <Star className="w-3 h-3 fill-amber-400" />
                        <span>{season.rating}</span>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Memorable Quotes / Game Notes Section */}
      {film.quotes && film.quotes.length > 0 && (
        <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md shadow-sm">
          <div className="flex items-center gap-2 mb-4 text-slate-800 dark:text-slate-100 font-bold text-base">
            <Quote className="w-5 h-5 text-indigo-500" />
            <span>{isGame ? t('game.quotes') : t('film.quotes')}</span>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {film.quotes.map((quote, idx) => (
              <div
                key={idx}
                className="p-3.5 rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50 text-xs text-slate-700 dark:text-slate-300 italic flex gap-2.5 items-start"
              >
                <span className="text-indigo-500 font-bold text-lg leading-none">“</span>
                <span className="flex-1">{quote}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Relations Section */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md shadow-sm">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2 text-slate-800 dark:text-slate-100 font-bold text-base">
            {isGame ? (
              <Gamepad2 className="w-5 h-5 text-indigo-500" />
            ) : (
              <FilmIcon className="w-5 h-5 text-indigo-500" />
            )}
            <span>{isGame ? t('relations.game_title') : t('relations.film_title')}</span>
          </div>
          <button
            onClick={() => setIsAddingRelation(!isAddingRelation)}
            className="flex items-center gap-1.5 rounded-xl border border-indigo-500/30 bg-indigo-50 dark:bg-indigo-500/10 px-3 py-1.5 text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-500/20 transition-colors"
          >
            <Plus className="w-4 h-4" />
            <span>{t('relations.add_btn')}</span>
          </button>
        </div>

        {/* Add Relation Form */}
        {isAddingRelation && (
          <div className="mb-4 p-4 rounded-2xl border border-indigo-200 dark:border-indigo-900/50 bg-indigo-50/40 dark:bg-indigo-950/20 space-y-3 animate-in fade-in">
            <div className="text-xs font-semibold text-slate-700 dark:text-slate-300">
              {isGame ? t('relations.this_game') : t('relations.this_film')} (
              <span className="text-indigo-600 dark:text-indigo-400 font-bold">{film.title}</span>
              ):
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <select
                value={relationChoice}
                onChange={(e) => setRelationChoice(e.target.value as RelationChoice)}
                className="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-2 text-xs text-slate-800 dark:text-slate-200 font-medium"
              >
                {isGame ? (
                  <>
                    <option value="sequel_to">является продолжением (сиквелом) к игре...</option>
                    <option value="prequel_to">является предысторией (приквелом) к игре...</option>
                    <option value="has_sequel">имеет продолжение (сиквел) — игра...</option>
                    <option value="has_prequel">имеет предысторию (приквел) — игра...</option>
                    <option value="remake_of">является ремейком / ремастером игры...</option>
                    <option value="original_of">является оригиналом для ремейка...</option>
                    <option value="spin_off_to">является спин-оффом (ответвлением) от игры...</option>
                    <option value="has_spin_off">имеет спин-офф (ответвление) — игра...</option>
                    <option value="related">связана по серии / вселенной с игрой...</option>
                  </>
                ) : (
                  <>
                    <option value="sequel_to">является продолжением (сиквелом) к фильму...</option>
                    <option value="prequel_to">является предысторией (приквелом) к фильму...</option>
                    <option value="has_sequel">имеет продолжение (сиквел) — фильм...</option>
                    <option value="has_prequel">имеет предысторию (приквел) — фильм...</option>
                    <option value="remake_of">является ремейком фильма...</option>
                    <option value="original_of">является оригиналом для ремейка...</option>
                    <option value="spin_off_to">является спин-оффом (ответвлением) от фильма...</option>
                    <option value="has_spin_off">имеет спин-офф (ответвление) — фильм...</option>
                    <option value="related">связан по сюжету / франшизе с фильмом...</option>
                  </>
                )}
              </select>

              <select
                value={targetFilmId}
                onChange={(e) => setTargetFilmId(e.target.value ? Number(e.target.value) : '')}
                className="rounded-xl border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-2 text-xs text-slate-800 dark:text-slate-200 flex-1 min-w-[220px]"
              >
                <option value="">
                  {isGame ? t('relations.select_game') : t('relations.select_film')}
                </option>
                {availableFilmsForRelation.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.title} ({f.media_type === 'game' ? f.platform || f.launcher || 'Игра' : f.year || '—'})
                  </option>
                ))}
              </select>

              <button
                onClick={handleAddRelation}
                disabled={!targetFilmId}
                className="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white disabled:opacity-50 hover:bg-indigo-500 transition-colors shadow-sm cursor-pointer"
              >
                {t('relations.save_btn')}
              </button>
              <button
                onClick={() => setIsAddingRelation(false)}
                className="rounded-xl border border-slate-300 dark:border-slate-700 px-3 py-2 text-xs font-semibold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
              >
                {t('relations.cancel_btn')}
              </button>
            </div>
          </div>
        )}

        {relations.length === 0 ? (
          <p className="text-xs text-slate-500 italic">
            {isGame ? t('relations.empty_game') : t('relations.empty_film')}
          </p>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3">
            {relations.map((rel) => {
              const relIsGame = rel.film.media_type === 'game';
              return (
                <div
                  key={rel.relation_id}
                  onClick={() => onOpenFilm(rel.film)}
                  className="group flex items-center justify-between p-3 rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/40 hover:border-indigo-500/50 hover:bg-indigo-50/20 dark:hover:bg-indigo-950/10 transition-all cursor-pointer shadow-xs"
                >
                  <div className="flex items-center gap-3 min-w-0 flex-1 pr-2">
                    <div className="w-10 h-14 rounded-lg bg-slate-200 dark:bg-slate-800 overflow-hidden shrink-0 flex items-center justify-center">
                      {relationPosters[rel.film.id] ? (
                        <img
                          src={relationPosters[rel.film.id]}
                          alt={rel.film.title}
                          className="w-full h-full object-cover"
                        />
                      ) : relIsGame ? (
                        <Gamepad2 className="w-4 h-4 text-slate-400" />
                      ) : (
                        <FilmIcon className="w-4 h-4 text-slate-400" />
                      )}
                    </div>
                    <div className="min-w-0">
                      <span className="text-[11px] font-bold text-indigo-600 dark:text-indigo-400 block mb-0.5 truncate">
                        {getRelationKindLabel(rel.kind, rel.outgoing)}
                      </span>
                      <span className="text-sm font-semibold text-slate-800 dark:text-slate-200 line-clamp-1 group-hover:text-indigo-600 dark:group-hover:text-indigo-400">
                        {rel.film.title}
                      </span>
                      <span className="text-xs text-slate-400">
                        {relIsGame ? rel.film.platform || rel.film.launcher || 'Игра' : rel.film.year || '—'}
                      </span>
                    </div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemoveRelation(rel.relation_id);
                    }}
                    title={t('relations.delete_title')}
                    className="p-1.5 rounded-lg text-slate-400 hover:text-rose-500 hover:bg-rose-50 dark:hover:bg-rose-950/30 transition-colors"
                  >
                    <Trash className="w-3.5 h-3.5" />
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Screenshots Gallery Section */}
      <div className="rounded-3xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-6 backdrop-blur-md shadow-sm">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2 text-slate-800 dark:text-slate-100 font-bold text-base">
            <ImageIcon className="w-5 h-5 text-indigo-500" />
            <span>{isGame ? t('screenshots.game_title') : t('screenshots.film_title')}</span>
          </div>
          <button
            onClick={handleAddScreenshot}
            className="flex items-center gap-1.5 rounded-xl border border-indigo-500/30 bg-indigo-50 dark:bg-indigo-500/10 px-3 py-1.5 text-xs font-semibold text-indigo-600 dark:text-indigo-400 hover:bg-indigo-100 dark:hover:bg-indigo-500/20 transition-colors"
          >
            <Plus className="w-4 h-4" />
            <span>{isGame ? t('screenshots.add_game_btn') : t('screenshots.add_film_btn')}</span>
          </button>
        </div>

        {screenshots.length === 0 ? (
          <p className="text-xs text-slate-500 italic">
            {isGame ? t('screenshots.empty_game') : t('screenshots.empty_film')}
          </p>
        ) : (
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {screenshots.map((shot) => (
              <div
                key={shot.id}
                className="group relative rounded-2xl overflow-hidden border border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 aspect-video shadow-sm"
              >
                {screenshotSrcs[shot.id] ? (
                  <img
                    src={screenshotSrcs[shot.id]}
                    alt={shot.caption || (isGame ? t('screenshots.game_caption') : t('screenshots.film_caption'))}
                    className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                  />
                ) : (
                  <div className="flex h-full w-full items-center justify-center text-slate-400 text-xs">
                    Загрузка...
                  </div>
                )}

                <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-between p-2.5">
                  <button
                    onClick={() => handleDeleteScreenshot(shot.id)}
                    title={t('screenshots.delete_btn')}
                    className="self-end p-1 rounded-lg bg-black/60 text-rose-400 hover:bg-rose-600 hover:text-white transition-colors"
                  >
                    <Trash className="w-3.5 h-3.5" />
                  </button>
                  {shot.caption && (
                    <span className="text-[11px] text-white font-medium line-clamp-2">
                      {shot.caption}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
