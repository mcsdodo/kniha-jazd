CREATE TABLE paperless_trip_links (
    trip_id TEXT PRIMARY KEY,
    paperless_document_id INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);
CREATE INDEX idx_paperless_links_doc ON paperless_trip_links(paperless_document_id);
