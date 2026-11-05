-- Миграция 6: скриншоты (кадры) и цитаты к фильмам.

-- Цитаты хранятся в виде JSON-массива строк: ["Цитата 1", "Цитата 2"].
ALTER TABLE films ADD COLUMN quotes TEXT NOT NULL DEFAULT '[]';

-- Кадры и скриншоты фильма.
CREATE TABLE film_screenshots (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    film_id    INTEGER NOT NULL REFERENCES films(id) ON DELETE CASCADE,
    image_path TEXT    NOT NULL,
    caption    TEXT    NOT NULL DEFAULT '',
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_film_screenshots_film ON film_screenshots (film_id, position);
