-- Миграция 007: иерархия сезонов (колоды тайтлов).
-- Позволяет привязать сезон (дочернюю карточку) к родительскому сериалу или аниме.

ALTER TABLE films ADD COLUMN parent_id INTEGER REFERENCES films(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_films_parent_id ON films(parent_id);

