import React, { useEffect, useState } from 'react';
import type { Film } from '../types';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { isSerialCategory } from '../utils';
import { Star, Clock, Film as FilmIcon, Edit, Trash2, Layers, Gamepad2 } from 'lucide-react';

interface FilmCardProps {
  film: Film;
  onOpen: (film: Film) => void;
  onEdit: (film: Film) => void;
  onDelete: (id: number) => void;
}

export const FilmCard: React.FC<FilmCardProps> = ({ film, onOpen, onEdit, onDelete }) => {
  const { t } = useTranslation();
  const [posterSrc, setPosterSrc] = useState<string | null>(null);
  const [seasons, setSeasons] = useState<Film[]>([]);
  const [seasonPosters, setSeasonPosters] = useState<Record<number, string>>({});
  const [isHovered, setIsHovered] = useState(false);

  // Load Main Poster
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

  // Load Child Seasons (Deck)
  useEffect(() => {
    if (!isSerialCategory(film.category)) {
      setSeasons([]);
      return;
    }
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
  }, [film.id, film.category]);

  const isGame = film.media_type === 'game';
  const isDeck =
    !isGame &&
    (isSerialCategory(film.category) ||
      seasons.length > 0 ||
      (film.seasons_total ? film.seasons_total > 1 : false));
  const hasSeasons = seasons.length > 0;

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'watched':
        return 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border-emerald-500/30';
      case 'watching':
        return 'bg-blue-500/20 text-blue-600 dark:text-blue-400 border-blue-500/30';
      case 'dropped':
        return 'bg-rose-500/20 text-rose-600 dark:text-rose-400 border-rose-500/30';
      case 'want_to_watch':
      default:
        return 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30';
    }
  };

  return (
    <div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className="group relative flex flex-col"
    >
      {/* Visual DECK EFFECT for Serials / Anime - Always Visible! */}
      {isDeck && (
        <>
          {/* Layer 3 (Back-most card) */}
          <div className="absolute -top-2.5 -right-2.5 w-full h-full rounded-2xl bg-indigo-100/80 dark:bg-slate-800/90 border border-indigo-200/90 dark:border-slate-700/80 shadow-sm pointer-events-none transition-transform duration-300 group-hover:-translate-y-1.5 group-hover:translate-x-1.5" />
          {/* Layer 2 (Middle card) */}
          <div className="absolute -top-1.25 -right-1.25 w-full h-full rounded-2xl bg-slate-100/90 dark:bg-slate-850 dark:bg-slate-800 border border-slate-300/80 dark:border-slate-700/90 shadow-sm pointer-events-none transition-transform duration-300 group-hover:-translate-y-0.75 group-hover:translate-x-0.75" />
        </>
      )}

      {/* Main Front Card */}
      <div
        onClick={() => onOpen(film)}
        className="relative z-10 flex flex-col h-full rounded-2xl bg-white dark:bg-slate-900/95 border border-slate-200 dark:border-slate-800 shadow-sm transition-all duration-300 group-hover:-translate-y-1 group-hover:border-indigo-500/50 group-hover:shadow-xl group-hover:shadow-indigo-500/10 cursor-pointer overflow-visible"
      >
        {/* Main Poster Container */}
        <div className="aspect-[2/3] w-full bg-slate-100 dark:bg-slate-950 rounded-t-2xl overflow-hidden relative">
          {posterSrc ? (
            <img
              src={posterSrc}
              alt={film.title}
              className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
            />
          ) : (
            <div className="flex h-full w-full flex-col items-center justify-center p-4 text-center bg-gradient-to-br from-slate-100 to-slate-200 dark:from-slate-900 dark:to-slate-950 text-slate-400 dark:text-slate-600 select-none">
              {isGame ? (
                <Gamepad2 className="w-10 h-10 mb-2 opacity-40 text-indigo-500" />
              ) : (
                <FilmIcon className="w-10 h-10 mb-2 opacity-40" />
              )}
              <span className="text-xs font-bold opacity-50 tracking-wider uppercase">
                {film.category || (isGame ? 'Игра' : 'Film')}
              </span>
            </div>
          )}

          {/* Deck Count Badge */}
          {isDeck && (
            <div className="absolute top-2.5 left-2.5 flex items-center gap-1.5 rounded-lg bg-indigo-600/95 text-white px-2 py-0.5 text-[11px] font-bold backdrop-blur-md shadow-md border border-indigo-400/40">
              <Layers className="w-3.5 h-3.5" />
              <span>{seasons.length > 0 ? `${seasons.length} сез.` : (film.category || 'Сериал')}</span>
            </div>
          )}

          {/* Status badge */}
          <span
            className={`absolute top-2.5 right-2.5 rounded-md px-2 py-0.5 text-[11px] font-semibold backdrop-blur-md border ${getStatusBadge(
              film.status
            )}`}
          >
            {isGame
              ? film.completion_status || t(`status.game_${film.status}`)
              : t(`status.movie_${film.status}`)}
          </span>

          {/* Quick action buttons on hover (when not a deck) */}
          {!isDeck && (
            <div
              className="absolute top-2.5 left-2.5 flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity z-10"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
              }}
            >
              <button
                type="button"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  onEdit(film);
                }}
                title={isGame ? t('game.edit') : t('film.edit')}
                className="p-1.5 rounded-lg bg-white/90 dark:bg-slate-900/80 backdrop-blur-md border border-slate-300 dark:border-slate-700 text-slate-700 dark:text-slate-200 hover:text-white hover:bg-indigo-600 transition-colors shadow-sm cursor-pointer"
              >
                <Edit className="w-3.5 h-3.5" />
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
                title={isGame ? t('game.delete') : t('film.delete')}
                className="p-1.5 rounded-lg bg-white/90 dark:bg-slate-900/80 backdrop-blur-md border border-slate-300 dark:border-slate-700 text-rose-600 dark:text-rose-300 hover:text-white hover:bg-rose-600 transition-colors shadow-sm cursor-pointer"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          )}

        {/* Rating */}
        {film.rating !== null && film.rating !== undefined && (
          <div className="absolute bottom-2.5 left-2.5 flex items-center gap-1 rounded-md bg-white/90 dark:bg-slate-950/85 px-2 py-0.5 text-xs font-bold text-amber-500 dark:text-amber-400 backdrop-blur-sm border border-slate-200 dark:border-slate-700/60 shadow">
            <Star className="w-3 h-3 fill-amber-400 text-amber-400" />
            <span>{film.rating}</span>
          </div>
        )}

        {/* Playtime / Runtime */}
        {isGame ? (
          film.playtime_hours !== null && film.playtime_hours !== undefined && (
            <div className="absolute bottom-2.5 right-2.5 flex items-center gap-1 rounded-md bg-white/90 dark:bg-slate-950/85 px-2 py-0.5 text-[11px] font-medium text-slate-700 dark:text-slate-300 backdrop-blur-sm border border-slate-200 dark:border-slate-700/60 shadow">
              <Clock className="w-3 h-3 text-indigo-500 dark:text-indigo-400" />
              <span>
                {film.playtime_hours} {t('game.playtime_short')}
              </span>
            </div>
          )
        ) : (
          film.runtime_minutes && (
            <div className="absolute bottom-2.5 right-2.5 flex items-center gap-1 rounded-md bg-white/90 dark:bg-slate-950/85 px-2 py-0.5 text-[11px] font-medium text-slate-700 dark:text-slate-300 backdrop-blur-sm border border-slate-200 dark:border-slate-700/60 shadow">
              <Clock className="w-3 h-3 text-slate-500 dark:text-slate-400" />
              <span>
                {film.runtime_minutes} {t('film.minutes_short')}
              </span>
            </div>
          )
        )}
      </div>

      {/* Info Block */}
      <div className="flex flex-1 flex-col p-3.5 justify-between gap-2">
        <div>
          {isGame ? (
            <div className="flex items-center gap-1.5 text-[11px] text-indigo-600 dark:text-indigo-400 font-semibold mb-0.5">
              <span>{film.category || 'Игры'}</span>
              {film.launcher && (
                <>
                  <span>•</span>
                  <span className="truncate max-w-[100px]">{film.launcher}</span>
                </>
              )}
            </div>
          ) : (
            <div className="flex items-center gap-1.5 text-[11px] text-indigo-600 dark:text-indigo-400 font-semibold mb-0.5">
              <span>{film.category}</span>
              {film.year && (
                <>
                  <span>•</span>
                  <span>{film.year}</span>
                </>
              )}
            </div>
          )}
          <h3 className="line-clamp-1 text-sm font-semibold text-slate-900 dark:text-slate-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-300 transition-colors">
            {film.title}
          </h3>
          {film.original_title && (
            <p className="line-clamp-1 text-xs text-slate-500 dark:text-slate-400 italic mt-0.5">
              {film.original_title}
            </p>
          )}
        </div>

        {isGame ? (
          <div className="border-t border-slate-200 dark:border-slate-800/80 pt-2 text-xs text-slate-500 dark:text-slate-400 flex items-center justify-between">
            <span className="truncate max-w-[110px] font-semibold text-slate-700 dark:text-slate-300">
              {film.platform || t('game.unknown_platform')}
            </span>
            {film.playtime_hours !== null && film.playtime_hours !== undefined ? (
              <span className="text-[11px] font-bold text-indigo-600 dark:text-indigo-400 bg-indigo-500/10 px-1.5 py-0.5 rounded border border-indigo-500/20">
                {film.playtime_hours} {t('game.playtime_short')}
              </span>
            ) : film.completion_status ? (
              <span className="text-[11px] text-slate-400 font-medium truncate max-w-[90px]">
                {film.completion_status}
              </span>
            ) : null}
          </div>
        ) : (
          <div className="border-t border-slate-200 dark:border-slate-800/80 pt-2 text-xs text-slate-500 dark:text-slate-400 flex items-center justify-between">
            <span className="truncate max-w-[150px]">
              {film.director || t('film.unknown_director')}
            </span>
            {film.country && (
              <span className="text-[11px] text-slate-400 dark:text-slate-500 uppercase tracking-wider font-semibold">
                {film.country}
              </span>
            )}
          </div>
        )}
      </div>
      </div>

      {/* EXPANDABLE SEASONS DRAWER (Колода при hover) */}
      {hasSeasons && isHovered && (
        <div
          onClick={(e) => e.stopPropagation()}
          className="absolute top-full left-0 right-0 z-40 mt-1 p-3 rounded-2xl bg-white/95 dark:bg-slate-900/95 backdrop-blur-xl border border-indigo-500/40 shadow-2xl animate-in fade-in zoom-in-95 duration-200"
        >
          <div className="flex items-center justify-between mb-2 pb-1.5 border-b border-slate-200 dark:border-slate-800">
            <span className="text-xs font-bold text-indigo-600 dark:text-indigo-400 flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5" />
              <span>Сезоны ({seasons.length})</span>
            </span>
            <span className="text-[10px] text-slate-400">Кликните сезон</span>
          </div>

          <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto pr-1">
            {seasons.map((season) => (
              <div
                key={season.id}
                onClick={(e) => {
                  e.stopPropagation();
                  onOpen(season);
                }}
                className="group/item flex items-center gap-2 p-1.5 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 hover:border-indigo-500/50 hover:bg-indigo-50 dark:hover:bg-indigo-950/30 transition-all cursor-pointer"
              >
                <div className="w-9 h-12 rounded-lg bg-slate-200 dark:bg-slate-800 overflow-hidden shrink-0">
                  {seasonPosters[season.id] ? (
                    <img
                      src={seasonPosters[season.id]}
                      alt={season.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-[10px] font-bold text-slate-400">
                      S
                    </div>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <span className="text-[11px] font-semibold text-slate-800 dark:text-slate-200 line-clamp-1 group-hover/item:text-indigo-600 dark:group-hover/item:text-indigo-400">
                    {season.title}
                  </span>
                  <div className="flex items-center gap-1 text-[10px] text-slate-400">
                    <span>{season.year || '—'}</span>
                    {season.rating && (
                      <span className="text-amber-500 font-bold">★ {season.rating}</span>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
