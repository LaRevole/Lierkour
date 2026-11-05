import React from 'react';
import { useTranslation } from 'react-i18next';
import type { DashboardStats } from '../types';
import { Film, CheckCircle2, Bookmark, Eye, XCircle, Star, Clock, Trophy, Layers } from 'lucide-react';

interface AnalyticsViewProps {
  stats: DashboardStats | null;
}

export const AnalyticsView: React.FC<AnalyticsViewProps> = ({ stats }) => {
  const { t } = useTranslation();

  if (!stats) {
    return (
      <div className="flex h-64 items-center justify-center text-slate-500">
        {t('status.loading')}
      </div>
    );
  }

  const hours = Math.floor(stats.total_runtime_minutes / 60);
  const minutes = stats.total_runtime_minutes % 60;

  return (
    <div className="space-y-6 animate-in fade-in duration-200">
      <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
        <Trophy className="w-5 h-5 text-indigo-400" />
        <span>{t('analytics.title')}</span>
      </h2>

      {/* KPI Cards Grid */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.total_films')}</span>
            <Film className="w-4 h-4 text-indigo-500 dark:text-indigo-400" />
          </div>
          <div className="text-2xl font-black text-slate-900 dark:text-slate-100">{stats.total_films}</div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.watched_films')}</span>
            <CheckCircle2 className="w-4 h-4 text-emerald-500 dark:text-emerald-400" />
          </div>
          <div className="text-2xl font-black text-emerald-600 dark:text-emerald-400">{stats.watched_films}</div>
        </div>

        <div className="rounded-2xl border border-indigo-200 dark:border-indigo-900/50 bg-indigo-50/50 dark:bg-indigo-950/30 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-indigo-600 dark:text-indigo-400 text-xs mb-2 font-semibold">
            <span>{t('analytics.watched_seasons')}</span>
            <Layers className="w-4 h-4 text-indigo-500 dark:text-indigo-400" />
          </div>
          <div className="text-2xl font-black text-indigo-600 dark:text-indigo-300">
            {stats.watched_seasons}
            <span className="text-xs font-semibold text-slate-500 dark:text-slate-400 ml-1.5">
              {t('analytics.seasons_out_of')} {stats.total_seasons}
            </span>
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.want_to_watch')}</span>
            <Bookmark className="w-4 h-4 text-amber-500 dark:text-amber-400" />
          </div>
          <div className="text-2xl font-black text-amber-500 dark:text-amber-400">{stats.want_to_watch}</div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.watching')}</span>
            <Eye className="w-4 h-4 text-blue-500 dark:text-blue-400" />
          </div>
          <div className="text-2xl font-black text-blue-500 dark:text-blue-400">{stats.watching}</div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.dropped')}</span>
            <XCircle className="w-4 h-4 text-rose-500 dark:text-rose-400" />
          </div>
          <div className="text-2xl font-black text-rose-500 dark:text-rose-400">{stats.dropped}</div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.average_rating')}</span>
            <Star className="w-4 h-4 text-amber-500 dark:text-amber-400" />
          </div>
          <div className="text-2xl font-black text-amber-500 dark:text-amber-400">
            {stats.average_rating ? stats.average_rating.toFixed(1) : '—'}
          </div>
        </div>

        <div className="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 backdrop-blur-md shadow-xs">
          <div className="flex items-center justify-between text-slate-500 dark:text-slate-400 text-xs mb-2">
            <span>{t('analytics.total_runtime')}</span>
            <Clock className="w-4 h-4 text-indigo-500 dark:text-indigo-400" />
          </div>
          <div className="text-2xl font-black text-indigo-600 dark:text-indigo-300">
            {hours} {t('analytics.hours')} {minutes} {t('analytics.minutes')}
          </div>
        </div>
      </div>

      {/* Top Lists */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-2">
        {/* Top Genres */}
        <div className="rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <h3 className="text-sm font-bold text-slate-200 mb-4">{t('analytics.top_genres')}</h3>
          <div className="space-y-3">
            {stats.top_genres.length === 0 ? (
              <span className="text-xs text-slate-500">—</span>
            ) : (
              stats.top_genres.map(([genre, count], idx) => (
                <div key={idx} className="flex items-center justify-between text-xs">
                  <span className="text-slate-300 font-medium">{genre}</span>
                  <span className="font-bold text-indigo-400 bg-indigo-500/10 px-2 py-0.5 rounded-md border border-indigo-500/20">
                    {count}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Top Directors */}
        <div className="rounded-2xl border border-slate-800 bg-slate-900/60 p-5 backdrop-blur-md">
          <h3 className="text-sm font-bold text-slate-200 mb-4">{t('analytics.top_directors')}</h3>
          <div className="space-y-3">
            {stats.top_directors.length === 0 ? (
              <span className="text-xs text-slate-500">—</span>
            ) : (
              stats.top_directors.map(([director, count], idx) => (
                <div key={idx} className="flex items-center justify-between text-xs">
                  <span className="text-slate-300 font-medium">{director}</span>
                  <span className="font-bold text-indigo-400 bg-indigo-500/10 px-2 py-0.5 rounded-md border border-indigo-500/20">
                    {count}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

