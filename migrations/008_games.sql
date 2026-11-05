-- Миграция 008: универсальный трекер медиа (поддержка видеоигр).
-- Добавляет media_type со значением по умолчанию 'movie' для сохранения обратной совместимости
-- со всеми существующими фильмами и сериалами, а также специализированные поля для игр.

ALTER TABLE films ADD COLUMN media_type TEXT NOT NULL DEFAULT 'movie';
ALTER TABLE films ADD COLUMN platform TEXT;
ALTER TABLE films ADD COLUMN launcher TEXT;
ALTER TABLE films ADD COLUMN playtime_hours INTEGER;
ALTER TABLE films ADD COLUMN completion_status TEXT;

CREATE INDEX IF NOT EXISTS idx_films_media_type ON films(media_type);
CREATE INDEX IF NOT EXISTS idx_films_platform ON films(platform);

