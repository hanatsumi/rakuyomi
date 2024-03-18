-- Create initial tables
CREATE TABLE manga_informations (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    title TEXT NULL,
    author TEXT NULL,
    artist TEXT NULL,
    cover_url TEXT NULL,
    PRIMARY KEY (source_id, manga_id)
) STRICT;

CREATE TABLE chapter_informations (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    chapter_id TEXT NOT NULL,
    manga_order INTEGER NOT NULL,
    title TEXT NULL,
    scanlator TEXT NULL,
    chapter_number REAL NULL,
    volume_number REAL NULL,
    PRIMARY KEY (source_id, manga_id, chapter_id)
) STRICT;

CREATE TABLE manga_library (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    PRIMARY KEY (source_id, manga_id)
) STRICT;

CREATE TABLE chapter_state (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    chapter_id TEXT NOT NULL,
    read INTEGER NOT NULL
) STRICT;
