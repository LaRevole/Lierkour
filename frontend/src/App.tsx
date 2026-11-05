import React, { useEffect, useState, useRef, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import type { Film, FilmFilter, DashboardStats, NewFilm } from './types';
import { FilmCard } from './components/FilmCard';
import { FilmModal } from './components/FilmModal';
import { FilmDetailView } from './components/FilmDetailView';
import { AnalyticsView } from './components/AnalyticsView';
import { BackupView } from './components/BackupView';
import { SettingsView } from './components/SettingsView';
import {
  Search,
  Film as FilmIcon,
  Plus,
  Globe,
  BarChart3,
  Archive,
  Settings,
  Library,
  Sun,
  Moon,
  ChevronLeft,
  ChevronRight,
  Gamepad2,
  Layers,
} from 'lucide-react';

type ActiveTab = 'library' | 'analytics' | 'backup' | 'settings';

interface NavigationEntry {
  tab: ActiveTab;
  film: Film | null;
  scrollY: number;
}

export const App: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<ActiveTab>('library');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  const [films, setFilms] = useState<Film[]>([]);
  const [allFilmsRaw, setAllFilmsRaw] = useState<Film[]>([]);
  const [categories, setCategories] = useState<string[]>([]);
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [loading, setLoading] = useState(true);

  // Search & Filter
  const [search, setSearch] = useState('');
  const [selectedMediaType, setSelectedMediaType] = useState<'all' | 'movie' | 'game'>('all');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [selectedStatus, setSelectedStatus] = useState<string>('all');

  // Modal State
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingFilm, setEditingFilm] = useState<Film | null>(null);
  const [parentForSeason, setParentForSeason] = useState<{ parent: Film; seasonNumber: number } | null>(null);

  // Detailed View State & Navigation History
  const [selectedFilm, setSelectedFilm] = useState<Film | null>(null);
  const [detailRefreshCounter, setDetailRefreshCounter] = useState(0);
  const [historyStack, setHistoryStack] = useState<NavigationEntry[]>([]);
  const [forwardStack, setForwardStack] = useState<NavigationEntry[]>([]);

  // Navigation Gesture Refs (to survive re-renders and eliminate skipping)
  const lastGestureTimeRef = useRef<number>(0);
  const accumulatedDeltaXRef = useRef<number>(0);
  const isGestureLockedRef = useRef<boolean>(false);
  const lastMouseNavTimeRef = useRef<number>(0);

  const handleSelectMediaType = (type: 'all' | 'movie' | 'game') => {
    setSelectedMediaType(type);
    setSelectedCategory('all');
    setSelectedStatus('all');
  };

  // Filtered categories depending on media type tab
  const visibleCategories = useMemo(() => {
    if (selectedMediaType === 'game') {
      const gameCategories = new Set<string>(['Игры']);
      allFilmsRaw.forEach((f) => {
        if (f.media_type === 'game' && f.category) {
          gameCategories.add(f.category);
        }
      });
      return Array.from(gameCategories);
    }
    if (selectedMediaType === 'movie') {
      const movieCategories = new Set<string>();
      categories.forEach((c) => {
        if (c !== 'Игры') movieCategories.add(c);
      });
      allFilmsRaw.forEach((f) => {
        if (f.media_type !== 'game' && f.category && f.category !== 'Игры') {
          movieCategories.add(f.category);
        }
      });
      return Array.from(movieCategories);
    }
    return categories;
  }, [selectedMediaType, categories, allFilmsRaw]);

  // Contextual status options
  const statusOptions = useMemo(() => {
    if (selectedMediaType === 'game') {
      return [
        { value: 'all', label: t('status.game_all') },
        { value: 'want_to_watch', label: t('status.game_want_to_watch') },
        { value: 'watching', label: t('status.game_watching') },
        { value: 'watched', label: t('status.game_watched') },
        { value: 'dropped', label: t('status.game_dropped') },
      ];
    }
    if (selectedMediaType === 'movie') {
      return [
        { value: 'all', label: t('status.movie_all') },
        { value: 'want_to_watch', label: t('status.movie_want_to_watch') },
        { value: 'watching', label: t('status.movie_watching') },
        { value: 'watched', label: t('status.movie_watched') },
        { value: 'dropped', label: t('status.movie_dropped') },
      ];
    }
    return [
      { value: 'all', label: t('status.all_union') },
      { value: 'want_to_watch', label: t('status.want_to_watch_all') },
      { value: 'watching', label: t('status.watching_all') },
      { value: 'watched', label: t('status.watched_all') },
      { value: 'dropped', label: t('status.dropped_all') },
    ];
  }, [selectedMediaType, t]);

  // Settings
  const [omdbApiKey, setOmdbApiKey] = useState('');

  // Load initial theme and settings from DB
  useEffect(() => {
    invoke<string>('get_theme')
      .then((savedTheme) => {
        if (savedTheme === 'light' || savedTheme === 'dark') {
          setTheme(savedTheme);
          if (savedTheme === 'light') {
            document.documentElement.classList.remove('dark');
          } else {
            document.documentElement.classList.add('dark');
          }
        }
      })
      .catch((err) => console.error(err));

    invoke<string | null>('get_setting', { key: 'omdb_api_key' })
      .then((val) => {
        if (val) setOmdbApiKey(val);
      })
      .catch((err) => console.error(err));
  }, []);

  const handleThemeChange = async (newTheme: string) => {
    const val = newTheme === 'light' ? 'light' : 'dark';
    setTheme(val);
    if (val === 'light') {
      document.documentElement.classList.remove('dark');
    } else {
      document.documentElement.classList.add('dark');
    }
    try {
      await invoke('set_theme', { themeStr: val });
    } catch (err) {
      console.error(err);
    }
  };

  const handleSaveApiKey = async (key: string) => {
    setOmdbApiKey(key);
    try {
      await invoke('set_setting', { key: 'omdb_api_key', value: key });
    } catch (err) {
      console.error(err);
    }
  };

  const fetchData = async () => {
    setLoading(true);
    try {
      const filter: FilmFilter = {
        search: search.trim(),
        category: selectedCategory === 'all' ? null : selectedCategory,
        status: selectedStatus === 'all' ? null : (selectedStatus as any),
        media_type: selectedMediaType === 'all' ? null : selectedMediaType,
      };

      const [filmsRes, allRaw, categoriesRes, statsRes] = await Promise.all([
        invoke<Film[]>('get_filtered_films', { filter }),
        invoke<Film[]>('get_all_films'),
        invoke<string[]>('get_categories'),
        invoke<DashboardStats>('get_stats'),
      ]);

      setFilms(filmsRes);
      setAllFilmsRaw(allRaw);
      setCategories(categoriesRes);
      setStats(statsRes);

      // Refresh currently open film if needed
      if (selectedFilm) {
        const updated = allRaw.find((f) => f.id === selectedFilm.id);
        if (updated) setSelectedFilm(updated);
      }
    } catch (err) {
      console.error('Failed to invoke Tauri commands:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, [search, selectedCategory, selectedStatus, selectedMediaType]);

  const toggleLanguage = () => {
    const nextLang = i18n.language.startsWith('ru') ? 'en' : 'ru';
    i18n.changeLanguage(nextLang);
  };

  const handleOpenAddModal = () => {
    setEditingFilm(null);
    setParentForSeason(null);
    setIsModalOpen(true);
  };

  const handleOpenAddSeasonModal = (parent: Film, seasonNumber: number) => {
    setEditingFilm(null);
    setParentForSeason({ parent, seasonNumber });
    setIsModalOpen(true);
  };

  const handleOpenEditModal = (film: Film) => {
    setEditingFilm(film);
    setParentForSeason(null);
    setIsModalOpen(true);
  };

  const handleSaveFilm = async (filmData: NewFilm) => {
    if (editingFilm) {
      const updated = await invoke<Film>('update_film', { id: editingFilm.id, film: filmData });
      if (selectedFilm && selectedFilm.id === editingFilm.id) {
        setSelectedFilm(updated);
      }
    } else {
      await invoke<Film>('add_film', { film: filmData });
      if (selectedFilm && filmData.parent_id === selectedFilm.id) {
        invoke<Film>('get_film', { id: selectedFilm.id }).then(setSelectedFilm).catch(() => {});
        setDetailRefreshCounter((c) => c + 1);
      }
    }
    fetchData();
  };

  const handleOpenFilm = (film: Film) => {
    const currentScroll = window.scrollY;
    setHistoryStack((prev) => [
      ...prev,
      { tab: activeTab, film: selectedFilm, scrollY: currentScroll },
    ]);
    setForwardStack([]);
    setSelectedFilm(film);
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
  };

  const handleTabChange = (newTab: ActiveTab) => {
    if (newTab === activeTab && !selectedFilm) return;
    const currentScroll = window.scrollY;
    setHistoryStack((prev) => [
      ...prev,
      { tab: activeTab, film: selectedFilm, scrollY: currentScroll },
    ]);
    setForwardStack([]);
    setSelectedFilm(null);
    setActiveTab(newTab);
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
  };

  const handleBack = () => {
    if (historyStack.length > 0) {
      const currentScroll = window.scrollY;
      const currentEntry: NavigationEntry = {
        tab: activeTab,
        film: selectedFilm,
        scrollY: currentScroll,
      };
      const prevEntry = historyStack[historyStack.length - 1];
      setHistoryStack((h) => h.slice(0, -1));
      setForwardStack((f) => [currentEntry, ...f]);

      setActiveTab(prevEntry.tab);
      setSelectedFilm(prevEntry.film);

      setTimeout(() => {
        window.scrollTo({ top: prevEntry.scrollY, left: 0, behavior: 'instant' });
      }, 20);
    } else if (selectedFilm) {
      if (selectedFilm.parent_id) {
        const parent = allFilmsRaw.find((f) => f.id === selectedFilm.parent_id);
        if (parent) {
          setSelectedFilm(parent);
          window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
          return;
        }
      }
      setSelectedFilm(null);
      window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
    }
  };

  const handleForward = () => {
    if (forwardStack.length > 0) {
      const currentScroll = window.scrollY;
      const currentEntry: NavigationEntry = {
        tab: activeTab,
        film: selectedFilm,
        scrollY: currentScroll,
      };
      const nextEntry = forwardStack[0];
      setForwardStack((f) => f.slice(1));
      setHistoryStack((h) => [...h, currentEntry]);

      setActiveTab(nextEntry.tab);
      setSelectedFilm(nextEntry.film);

      setTimeout(() => {
        window.scrollTo({ top: nextEntry.scrollY, left: 0, behavior: 'instant' });
      }, 20);
    }
  };

  // Mouse side buttons: 3 = Back (XBUTTON1), 4 = Forward (XBUTTON2)
  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      if (e.button === 3 || e.button === 4) {
        e.preventDefault();
      }
    };

    const handleMouseUp = (e: MouseEvent) => {
      const now = Date.now();
      if (now - lastMouseNavTimeRef.current < 400) return;

      if (e.button === 3) {
        e.preventDefault();
        lastMouseNavTimeRef.current = now;
        handleBack();
      } else if (e.button === 4) {
        e.preventDefault();
        lastMouseNavTimeRef.current = now;
        handleForward();
      }
    };

    window.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [historyStack, forwardStack, selectedFilm, activeTab]);

  // Touchpad 2-finger swipe navigation: Right -> Back, Left -> Forward
  useEffect(() => {
    let resetTimer: ReturnType<typeof setTimeout> | null = null;

    const handleWheel = (e: WheelEvent) => {
      if (isModalOpen) return;

      const absX = Math.abs(e.deltaX);
      const absY = Math.abs(e.deltaY);

      // Trigger if horizontal movement is strictly dominant and significant
      if (absX > absY * 1.5 && absX > 18) {
        const now = Date.now();
        if (now - lastGestureTimeRef.current < 750 || isGestureLockedRef.current) {
          return;
        }

        accumulatedDeltaXRef.current += e.deltaX;
        if (resetTimer) clearTimeout(resetTimer);
        resetTimer = setTimeout(() => {
          accumulatedDeltaXRef.current = 0;
          isGestureLockedRef.current = false;
        }, 220);

        if (accumulatedDeltaXRef.current < -120) {
          accumulatedDeltaXRef.current = 0;
          lastGestureTimeRef.current = now;
          isGestureLockedRef.current = true;
          handleBack();
        } else if (accumulatedDeltaXRef.current > 120) {
          accumulatedDeltaXRef.current = 0;
          lastGestureTimeRef.current = now;
          isGestureLockedRef.current = true;
          handleForward();
        }
      }
    };

    let touchStartX = 0;
    let touchStartY = 0;
    let touchStartTime = 0;

    const handleTouchStart = (e: TouchEvent) => {
      if (e.touches.length >= 1) {
        touchStartX = e.touches[0].clientX;
        touchStartY = e.touches[0].clientY;
        touchStartTime = Date.now();
      }
    };

    const handleTouchEnd = (e: TouchEvent) => {
      if (e.changedTouches.length >= 1) {
        const now = Date.now();
        if (now - lastGestureTimeRef.current < 500) return;

        const deltaX = e.changedTouches[0].clientX - touchStartX;
        const deltaY = e.changedTouches[0].clientY - touchStartY;
        const duration = now - touchStartTime;

        if (duration < 500 && Math.abs(deltaX) > 90 && Math.abs(deltaX) > Math.abs(deltaY) * 1.5) {
          lastGestureTimeRef.current = now;
          if (deltaX > 0) {
            handleBack();
          } else {
            handleForward();
          }
        }
      }
    };

    window.addEventListener('wheel', handleWheel, { passive: true });
    window.addEventListener('touchstart', handleTouchStart, { passive: true });
    window.addEventListener('touchend', handleTouchEnd, { passive: true });

    return () => {
      window.removeEventListener('wheel', handleWheel);
      window.removeEventListener('touchstart', handleTouchStart);
      window.removeEventListener('touchend', handleTouchEnd);
      if (resetTimer) clearTimeout(resetTimer);
    };
  }, [historyStack, forwardStack, selectedFilm, activeTab, isModalOpen]);

  const handleDeleteFilm = async (id: number) => {
    try {
      await invoke('delete_film', { id });
      setHistoryStack((h) => h.filter((entry) => entry.film?.id !== id));
      setForwardStack((f) => f.filter((entry) => entry.film?.id !== id));
      if (selectedFilm && selectedFilm.id === id) {
        handleBack();
      }
      fetchData();
    } catch (err) {
      console.error(err);
    }
  };

  const isLight = theme === 'light';

  return (
    <div
      className={`min-h-screen flex flex-col font-sans transition-colors duration-200 selection:bg-indigo-500/30 ${
        isLight ? 'bg-slate-50 text-slate-900' : 'bg-slate-950 text-slate-100'
      }`}
    >
      {/* Top Header */}
      <header
        className={`sticky top-0 z-30 border-b backdrop-blur-xl px-3 sm:px-6 py-2.5 sm:py-3.5 ${
          isLight
            ? 'border-slate-200 bg-white/80'
            : 'border-slate-800/80 bg-slate-950/80'
        }`}
      >
        <div className="mx-auto flex max-w-7xl items-center justify-between gap-2 sm:gap-4">
          {/* Logo & Navigation */}
          <div className="flex items-center gap-2 sm:gap-3 min-w-0">
            <div
              onClick={() => handleTabChange('library')}
              className="flex items-center gap-2 sm:gap-2.5 select-none cursor-pointer shrink-0"
            >
              <div className="h-9 w-9 sm:h-10 sm:w-10 rounded-xl bg-gradient-to-tr from-indigo-600 via-indigo-500 to-violet-500 flex items-center justify-center font-black text-white shadow-lg shadow-indigo-600/30 shrink-0">
                <Layers className="w-4 h-4 sm:w-5 sm:h-5 text-white" />
              </div>
              <div className="flex flex-col">
                <div className="flex items-center gap-1.5">
                  <h1 className="text-sm sm:text-base font-extrabold tracking-tight bg-gradient-to-r from-indigo-500 to-violet-500 bg-clip-text text-transparent">
                    {t('app.title')}
                  </h1>
                  <span className="text-[9px] sm:text-[10px] bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 font-bold px-1.5 py-0.2 rounded border border-indigo-500/20">
                    v2.0
                  </span>
                </div>
                <span className="text-[9px] sm:text-[10px] text-slate-500 dark:text-slate-400 font-medium tracking-tight hidden md:block">
                  {t('app.tagline')}
                </span>
              </div>
            </div>

            {/* Back / Forward Navigation Buttons */}
            <div className="flex items-center gap-0.5 bg-slate-100 dark:bg-slate-900/80 p-0.5 sm:p-1 rounded-xl border border-slate-200 dark:border-slate-800 shrink-0">
              <button
                onClick={handleBack}
                disabled={historyStack.length === 0 && !selectedFilm}
                title="Назад (Мышь: боковая кнопка назад / Тачпад: свайп вправо)"
                className="p-1 sm:p-1.5 rounded-lg text-slate-600 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-white dark:hover:bg-slate-800 disabled:opacity-25 disabled:hover:text-slate-400 disabled:hover:bg-transparent transition-all cursor-pointer disabled:cursor-not-allowed"
              >
                <ChevronLeft className="w-4 h-4" />
              </button>
              <button
                onClick={handleForward}
                disabled={forwardStack.length === 0}
                title="Вперед (Мышь: боковая кнопка вперед / Тачпад: свайп влево)"
                className="p-1 sm:p-1.5 rounded-lg text-slate-600 dark:text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:bg-white dark:hover:bg-slate-800 disabled:opacity-25 disabled:hover:text-slate-400 disabled:hover:bg-transparent transition-all cursor-pointer disabled:cursor-not-allowed"
              >
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>

            {/* Nav Links */}
            <nav className="flex items-center gap-0.5 sm:gap-1 shrink-0">
              <button
                onClick={() => handleTabChange('library')}
                title={t('app.nav_library')}
                className={`flex items-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-xl text-xs font-semibold transition-all ${
                  activeTab === 'library' && !selectedFilm
                    ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/25'
                    : isLight
                    ? 'text-slate-600 hover:bg-slate-200/60'
                    : 'text-slate-400 hover:bg-slate-900'
                }`}
              >
                <Library className="w-4 h-4 shrink-0" />
                <span className="hidden lg:inline">{t('app.nav_library')}</span>
              </button>

              <button
                onClick={() => handleTabChange('analytics')}
                title={t('app.nav_analytics')}
                className={`flex items-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-xl text-xs font-semibold transition-all ${
                  activeTab === 'analytics'
                    ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/25'
                    : isLight
                    ? 'text-slate-600 hover:bg-slate-200/60'
                    : 'text-slate-400 hover:bg-slate-900'
                }`}
              >
                <BarChart3 className="w-4 h-4 shrink-0" />
                <span className="hidden lg:inline">{t('app.nav_analytics')}</span>
              </button>

              <button
                onClick={() => handleTabChange('backup')}
                title={t('app.nav_backup')}
                className={`flex items-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-xl text-xs font-semibold transition-all ${
                  activeTab === 'backup'
                    ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/25'
                    : isLight
                    ? 'text-slate-600 hover:bg-slate-200/60'
                    : 'text-slate-400 hover:bg-slate-900'
                }`}
              >
                <Archive className="w-4 h-4 shrink-0" />
                <span className="hidden lg:inline">{t('app.nav_backup')}</span>
              </button>

              <button
                onClick={() => handleTabChange('settings')}
                title={t('app.nav_settings')}
                className={`flex items-center gap-1.5 px-2.5 sm:px-3 py-1.5 rounded-xl text-xs font-semibold transition-all ${
                  activeTab === 'settings'
                    ? 'bg-indigo-600 text-white shadow-md shadow-indigo-600/25'
                    : isLight
                    ? 'text-slate-600 hover:bg-slate-200/60'
                    : 'text-slate-400 hover:bg-slate-900'
                }`}
              >
                <Settings className="w-4 h-4 shrink-0" />
                <span className="hidden lg:inline">{t('app.nav_settings')}</span>
              </button>
            </nav>
          </div>

          {/* Quick Search in Header (when in library) */}
          {activeTab === 'library' && !selectedFilm && (
            <div className="relative flex-1 max-w-xs xl:max-w-md hidden md:block">
              <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
              <input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={
                  selectedMediaType === 'game'
                    ? t('app.search_placeholder_games')
                    : selectedMediaType === 'movie'
                    ? t('app.search_placeholder_movies')
                    : t('app.search_placeholder')
                }
                className={`w-full rounded-xl border pl-10 pr-4 py-2 text-xs focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 transition-all ${
                  isLight
                    ? 'border-slate-300 bg-white placeholder-slate-400 text-slate-900'
                    : 'border-slate-800 bg-slate-900 placeholder-slate-500 text-slate-100 shadow-inner'
                }`}
              />
            </div>
          )}

          {/* Actions & Language & Theme Switcher */}
          <div className="flex items-center gap-1.5 sm:gap-2.5 shrink-0">
            <button
              onClick={() => handleThemeChange(isLight ? 'dark' : 'light')}
              title={isLight ? t('settings.theme_dark') : t('settings.theme_light')}
              className={`p-2 rounded-xl border transition-colors shrink-0 ${
                isLight
                  ? 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'
                  : 'border-slate-800 bg-slate-900 text-slate-300 hover:bg-slate-800'
              }`}
            >
              {isLight ? <Moon className="w-4 h-4 text-indigo-500" /> : <Sun className="w-4 h-4 text-amber-500" />}
            </button>

            <button
              onClick={toggleLanguage}
              title="Переключить язык (RU/EN)"
              className={`flex items-center gap-1 rounded-xl border px-2.5 sm:px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors shrink-0 ${
                isLight
                  ? 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'
                  : 'border-slate-800 bg-slate-900 text-slate-300 hover:bg-slate-800'
              }`}
            >
              <Globe className="w-3.5 h-3.5 text-slate-400" />
              <span>{i18n.language.startsWith('ru') ? 'EN' : 'RU'}</span>
            </button>

            {activeTab === 'library' && (
              <button
                onClick={handleOpenAddModal}
                title={selectedMediaType === 'game' ? t('app.add_game') : t('app.add_film')}
                className="flex items-center gap-1.5 rounded-xl bg-indigo-600 px-3 sm:px-4 py-2 text-xs font-semibold text-white shadow-lg shadow-indigo-600/25 hover:bg-indigo-500 transition-all hover:scale-[1.02] active:scale-[0.98] cursor-pointer shrink-0"
              >
                {selectedMediaType === 'game' ? (
                  <Gamepad2 className="w-4 h-4 shrink-0" />
                ) : (
                  <Plus className="w-4 h-4 shrink-0" />
                )}
                <span>
                  {selectedMediaType === 'game' ? t('app.add_game') : t('app.add_film')}
                </span>
              </button>
            )}
          </div>
        </div>
      </header>

      {/* Main Container */}
      <div className="mx-auto w-full max-w-7xl flex-1 px-6 py-6 flex flex-col gap-6">
        {/* VIEW 1: FILM DETAIL VIEW */}
        {selectedFilm ? (
          <FilmDetailView
            film={selectedFilm}
            allFilms={allFilmsRaw}
            onBack={handleBack}
            onEdit={handleOpenEditModal}
            onDelete={handleDeleteFilm}
            onAddSeason={handleOpenAddSeasonModal}
            onOpenFilm={handleOpenFilm}
            onRefresh={fetchData}
            refreshTrigger={detailRefreshCounter}
          />
        ) : (
          <>
            {/* VIEW 2: LIBRARY GRID */}
            {activeTab === 'library' && (
              <div className="space-y-6">
                {/* Filter Dropdowns */}
                <div
                  className={`flex flex-wrap items-center justify-between gap-4 rounded-2xl border p-4 backdrop-blur-md ${
                    isLight
                      ? 'border-slate-200 bg-white/70 shadow-sm'
                      : 'border-slate-800/80 bg-slate-900/40'
                  }`}
                >
                  {/* Media Type Tabs: All | Movies | Games */}
                  <div className="flex items-center gap-1 p-1 rounded-xl bg-slate-100 dark:bg-slate-900/80 border border-slate-200 dark:border-slate-800">
                    <button
                      onClick={() => handleSelectMediaType('all')}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                        selectedMediaType === 'all'
                          ? 'bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-sm'
                          : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
                      }`}
                    >
                      <Layers className="w-3.5 h-3.5" />
                      <span>{t('app.media_all')}</span>
                    </button>
                    <button
                      onClick={() => handleSelectMediaType('movie')}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                        selectedMediaType === 'movie'
                          ? 'bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-sm'
                          : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
                      }`}
                    >
                      <FilmIcon className="w-3.5 h-3.5" />
                      <span>{t('app.media_movies')}</span>
                    </button>
                    <button
                      onClick={() => handleSelectMediaType('game')}
                      className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                        selectedMediaType === 'game'
                          ? 'bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-sm'
                          : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'
                      }`}
                    >
                      <Gamepad2 className="w-3.5 h-3.5" />
                      <span>{t('app.media_games')}</span>
                    </button>
                  </div>

                  <div className="text-xs font-medium text-slate-500">
                    {selectedMediaType === 'game'
                      ? t('analytics.total_games')
                      : selectedMediaType === 'movie'
                      ? t('analytics.total_films')
                      : t('analytics.total_items')}
                    : <span className="font-bold text-slate-800 dark:text-slate-200">{films.length}</span>
                  </div>

                  <div className="flex items-center gap-3">
                    <select
                      value={selectedCategory}
                      onChange={(e) => setSelectedCategory(e.target.value)}
                      className={`rounded-xl border px-3 py-1.5 text-xs focus:border-indigo-500 focus:outline-none ${
                        isLight
                          ? 'border-slate-300 bg-white text-slate-800'
                          : 'border-slate-800 bg-slate-900 text-slate-300'
                      }`}
                    >
                      <option value="all">
                        {selectedMediaType === 'game'
                          ? t('app.all_categories_games')
                          : selectedMediaType === 'movie'
                          ? t('app.all_categories_movies')
                          : t('app.all_categories')}
                      </option>
                      {visibleCategories.map((c) => (
                        <option key={c} value={c}>
                          {c}
                        </option>
                      ))}
                    </select>

                    <select
                      value={selectedStatus}
                      onChange={(e) => setSelectedStatus(e.target.value)}
                      className={`rounded-xl border px-3 py-1.5 text-xs focus:border-indigo-500 focus:outline-none ${
                        isLight
                          ? 'border-slate-300 bg-white text-slate-800'
                          : 'border-slate-800 bg-slate-900 text-slate-300'
                      }`}
                    >
                      {statusOptions.map((opt) => (
                        <option key={opt.value} value={opt.value}>
                          {opt.label}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                {/* Film Cards Grid */}
                <main className="flex-1">
                  {loading ? (
                    <div className="flex h-72 items-center justify-center text-slate-500">
                      <div className="flex flex-col items-center gap-3">
                        <div className="w-8 h-8 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" />
                        <span className="text-xs font-medium">{t('status.loading')}</span>
                      </div>
                    </div>
                  ) : films.length === 0 ? (
                    <div
                      className={`flex h-72 flex-col items-center justify-center text-center p-6 border border-dashed rounded-2xl ${
                        isLight
                          ? 'border-slate-300 bg-slate-100/50'
                          : 'border-slate-800 bg-slate-900/20'
                      }`}
                    >
                      {selectedMediaType === 'game' ? (
                        <Gamepad2 className="w-12 h-12 text-slate-400 dark:text-slate-600 mb-3 stroke-1" />
                      ) : (
                        <FilmIcon className="w-12 h-12 text-slate-400 dark:text-slate-600 mb-3 stroke-1" />
                      )}
                      <h3 className="text-base font-semibold text-slate-700 dark:text-slate-300">{t('status.empty')}</h3>
                      <p className="text-xs text-slate-500 mt-1 max-w-sm">
                        {selectedMediaType === 'game'
                          ? t('app.empty_games_desc')
                          : selectedMediaType === 'movie'
                          ? t('app.empty_movies_desc')
                          : t('app.empty_all_desc')}
                      </p>
                    </div>
                  ) : (
                    <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                      {films.map((film) => (
                        <FilmCard
                          key={film.id}
                          film={film}
                          onOpen={handleOpenFilm}
                          onEdit={handleOpenEditModal}
                          onDelete={handleDeleteFilm}
                        />
                      ))}
                    </div>
                  )}
                </main>
              </div>
            )}

            {/* VIEW 3: ANALYTICS */}
            {activeTab === 'analytics' && <AnalyticsView stats={stats} />}

            {/* VIEW 4: BACKUP & EXPORT */}
            {activeTab === 'backup' && <BackupView onRefresh={fetchData} />}

            {/* VIEW 5: SETTINGS */}
            {activeTab === 'settings' && (
              <SettingsView
                currentTheme={theme}
                onThemeChange={handleThemeChange}
                apiKey={omdbApiKey}
                onSaveApiKey={handleSaveApiKey}
              />
            )}
          </>
        )}
      </div>

      {/* Film Add / Edit / Add Season Modal */}
      <FilmModal
        isOpen={isModalOpen}
        filmToEdit={editingFilm}
        parentFilmForSeason={parentForSeason}
        initialMediaType={selectedMediaType === 'game' ? 'game' : 'movie'}
        onClose={() => {
          setIsModalOpen(false);
          setParentForSeason(null);
        }}
        onSave={handleSaveFilm}
        categories={categories}
      />
    </div>
  );
};

export default App;
