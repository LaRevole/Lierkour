export type FilmStatus = 'want_to_watch' | 'watching' | 'watched' | 'dropped';

export type RelationKind = 'sequel' | 'prequel' | 'remake' | 'spin_off' | 'related';

export interface Film {
  id: number;
  title: string;
  original_title: string;
  category: string;
  director: string;
  year: number | null;
  genres: string;
  country: string;
  runtime_minutes: number | null;
  imdb_id: string;
  tags: string;
  status: FilmStatus;
  rating: number | null;
  description: string;
  thoughts: string;
  poster_path: string | null;
  seasons_total: number;
  watched_seasons: number[];
  quotes: string[];
  parent_id?: number | null;
}

export interface NewFilm {
  title: string;
  original_title: string;
  category: string;
  director: string;
  year: number | null;
  genres: string;
  country: string;
  runtime_minutes: number | null;
  imdb_id: string;
  tags: string;
  status: FilmStatus;
  rating: number | null;
  description: string;
  thoughts: string;
  poster_path: string | null;
  seasons_total: number;
  watched_seasons: number[];
  quotes: string[];
  parent_id?: number | null;
}

export interface FilmFilter {
  search: string;
  status: FilmStatus | null;
  category: string | null;
}

export interface DashboardStats {
  total_films: number;
  watched_films: number;
  want_to_watch: number;
  watching: number;
  dropped: number;
  average_rating: number | null;
  total_runtime_minutes: number;
  top_genres: [string, number][];
  top_directors: [string, number][];
  total_seasons: number;
  watched_seasons: number;
}

export interface RelatedFilm {
  relation_id: number;
  film: Film;
  kind: RelationKind;
  outgoing: boolean;
}

export interface FilmScreenshot {
  id: number;
  film_id: number;
  image_path: string;
  caption: string;
  position: number;
}

export interface MovieMetadata {
  title: string;
  year: number | null;
  director: string;
  genres: string;
  country: string;
  runtime_minutes: number | null;
  description: string;
  imdb_id: string;
  total_seasons: number;
  poster_url: string | null;
}

export interface ExportSummary {
  films: number;
  relations: number;
  posters: number;
}

export interface ImportSummary {
  added: number;
  skipped: number;
  relations_added: number;
  posters_restored: number;
  warnings: string[];
}
