-- Миграция 4: расширенные метаданные, связи между фильмами и настройки.

-- Год выпуска. NULL — не указан.
ALTER TABLE films ADD COLUMN year            INTEGER;

-- Жанры через запятую, например "Фантастика, Приключения".
ALTER TABLE films ADD COLUMN genres          TEXT NOT NULL DEFAULT '';

-- Страна производства.
ALTER TABLE films ADD COLUMN country         TEXT NOT NULL DEFAULT '';

-- Длительность в минутах. NULL — не указана.
ALTER TABLE films ADD COLUMN runtime_minutes INTEGER;

-- Идентификатор IMDb (tt0133093). Используется автозаполнением.
ALTER TABLE films ADD COLUMN imdb_id         TEXT NOT NULL DEFAULT '';

-- Связи направленные: source — это kind по отношению к target.
-- Например, source = «Дюна: Часть вторая», target = «Дюна», kind = 'sequel'.
CREATE TABLE film_relations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id  INTEGER NOT NULL REFERENCES films(id) ON DELETE CASCADE,
    target_id  INTEGER NOT NULL REFERENCES films(id) ON DELETE CASCADE,
    kind       TEXT    NOT NULL CHECK (kind IN
                   ('sequel', 'prequel', 'remake', 'spin_off', 'related')),
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (source_id, target_id, kind),
    CHECK (source_id != target_id)
);

CREATE INDEX idx_film_relations_source ON film_relations (source_id);
CREATE INDEX idx_film_relations_target ON film_relations (target_id);

-- Простые настройки приложения (например, ключ OMDb).
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
