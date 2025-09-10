CREATE TABLE artists(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    about TEXT NOT NULL
);

CREATE UNIQUE INDEX artists_name_about ON artists (name, about);

CREATE TABLE publishers(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    about TEXT NOT NULL
);

CREATE UNIQUE INDEX publishers_name_about ON publishers (name, about);

CREATE TABLE playlists(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE tracks(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    date TEXT NOT NULL,
    number INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    artist_id INTEGER NOT NULL,
    release_id INTEGER NOT NULL,
    FOREIGN KEY (artist_id) REFERENCES artists(id),
    FOREIGN KEY (release_id) REFERENCES releases(id)
);

CREATE UNIQUE INDEX tracks_for_real ON tracks (
    name,
    date,
    number,
    duration,
    artist_id,
    release_id
);

CREATE TABLE playlist_items(
    id INTEGER NOT NULL PRIMARY KEY,
    date TEXT NOT NULL,
    number INTEGER NOT NULL,
    item_type INTEGER NOT NULL,
    deleted BOOL NOT NULL,
    playlist_id INTEGER NOT NULL,
    item_playlist_id INTEGER,
    item_release_id INTEGER,
    item_track_id INTEGER,
    FOREIGN KEY (item_release_id) REFERENCES releases(id),
    FOREIGN KEY (item_track_id) REFERENCES tracks(id)
);

CREATE TABLE releases(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    release_type TEXT,
    date TEXT NOT NULL,
    publisher_id INTEGER,
    artist_id INTEGER NOT NULL,
    FOREIGN KEY (publisher_id) REFERENCES publishers(id),
    FOREIGN KEY (artist_id) REFERENCES artists(id)
);

CREATE UNIQUE INDEX release_for_real ON releases (
    name,
    release_type,
    date,
    publisher_id,
    artist_id
);

CREATE TABLE genres(
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    track_id INTEGER NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks(id)
);

CREATE UNIQUE INDEX genres_for_real ON genres (name, track_id);

CREATE TABLE track_locations(
    path TEXT NOT NULL PRIMARY KEY,
    track_id INTEGER NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks(id)
);
