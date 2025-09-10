// @generated automatically by Diesel CLI.

diesel::table! {
    artists (id) {
        id -> Integer,
        name -> Text,
        about -> Text,
    }
}

diesel::table! {
    genres (id) {
        id -> Integer,
        name -> Text,
        track_id -> Integer,
    }
}

diesel::table! {
    playlists (id) {
        id -> Integer,
        name -> Text,
        description -> Text,
    }
}

diesel::table! {
    playlist_items (id) {
        id -> Integer,
        date -> Date,
        number -> Integer,
        item_type -> Integer,
        deleted -> Bool,
        playlist_id -> Integer,
        item_playlist_id -> Nullable<Integer>,
        item_release_id -> Nullable<Integer>,
        item_track_id -> Nullable<Integer>,
    }
}

diesel::table! {
    publishers (id) {
        id -> Integer,
        name -> Text,
        about -> Text,
    }
}

diesel::table! {
    releases (id) {
        id -> Integer,
        name -> Text,
        release_type -> Nullable<Text>,
        date -> Date,
        publisher_id -> Nullable<Integer>,
        artist_id -> Integer,
    }
}

diesel::table! {
    tracks (id) {
        id -> Integer,
        name -> Text,
        date -> Date,
        number -> Integer,
        duration -> Integer,
        artist_id -> Integer,
        release_id -> Integer,
    }
}

diesel::table! {
    track_locations (path) {
        path -> Text,
        track_id -> Integer,
    }
}

diesel::joinable!(genres -> tracks (track_id));
diesel::joinable!(playlist_items -> playlists (playlist_id));
diesel::joinable!(playlist_items -> releases (item_release_id));
diesel::joinable!(playlist_items -> tracks (item_track_id));
diesel::joinable!(releases -> artists (artist_id));
diesel::joinable!(releases -> publishers (publisher_id));
diesel::joinable!(tracks -> artists (artist_id));
diesel::joinable!(tracks -> releases (release_id));
diesel::joinable!(track_locations -> tracks (track_id));

diesel::allow_tables_to_appear_in_same_query!(
    artists,
    genres,
    playlists,
    playlist_items,
    publishers,
    releases,
    tracks,
    track_locations,
);
