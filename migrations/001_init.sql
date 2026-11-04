-- Миграция 1: базовая схема фильмотеки.
CREATE TABLE films (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL CHECK (length(trim(title)) > 0),
    status      TEXT    NOT NULL DEFAULT 'want_to_watch'
                        CHECK (status IN ('want_to_watch', 'watching', 'watched', 'dropped')),
    rating      INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 10),
    description TEXT    NOT NULL DEFAULT '',
    thoughts    TEXT    NOT NULL DEFAULT '',
    poster_path TEXT,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_films_status ON films (status);
CREATE INDEX idx_films_title  ON films (title COLLATE NOCASE);
