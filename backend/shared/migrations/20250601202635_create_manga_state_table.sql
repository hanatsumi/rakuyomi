-- Create manga_state table to track preferred_scanlator
CREATE TABLE manga_state (
    source_id TEXT NOT NULL,
    manga_id TEXT NOT NULL,
    preferred_scanlator TEXT NULL,
    PRIMARY KEY (source_id, manga_id)
) STRICT;