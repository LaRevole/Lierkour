-- Миграция 3: дополнительные данные о фильме, сезоны и ручной порядок карточек.

ALTER TABLE films ADD COLUMN original_title  TEXT    NOT NULL DEFAULT '';
ALTER TABLE films ADD COLUMN director        TEXT    NOT NULL DEFAULT '';

-- 0 — сезоны не указаны (обычный фильм или пользователь не заполнил).
ALTER TABLE films ADD COLUMN seasons_total   INTEGER NOT NULL DEFAULT 0;

-- Номера просмотренных сезонов через запятую, например "1,2,5".
ALTER TABLE films ADD COLUMN watched_seasons TEXT    NOT NULL DEFAULT '';

-- Порядок карточек на главной. Старые записи получают 0 и сортируются
-- по updated_at, как раньше; новые добавляются в начало.
ALTER TABLE films ADD COLUMN sort_order      INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_films_sort_order ON films (sort_order);
