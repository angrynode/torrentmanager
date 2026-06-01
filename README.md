# TorrentManager

> [!CAUTION]
> This branch is not usable. Uploads are not yet implemented.

TorrentManager is a torrenting web interface to manage your public archives. By using symlinks from the content directory to the torrent download directory, you can filter and rename exposed files without breaking the torrent structure for seeding.

## Planned features

TorrentManager is not feature-complete yet. This branch is the third iteration of the project, which should prove more mature to continue development. For history's sake:

- first prototype: PHP interface + bash/python processing scripts, only used for uploading torrents to qBittorrent (no follow-up)
- second iteration: Rust/rocket + bash/python processing scripts, allows viewing torrents from qBittorrent and filtering stuck torrents
- third iteration (this branch): Rust/axum + qBittorrent backend

In the future, TorrentManager will become federated and allow you to find content from your friends to help and distribute it. For example, subscribing to a hypothetical [media.ccc.de](https://media.ccc.de/) instance would help seeding their video files and automatically importing them into your local media library.

- [ ] Torrent backend integrations
  - [x] qBittorrent v5.0.x/v5.1.x
  - [ ] Transmission
  - [ ] rqbit
- [x] Torrent categories (video, iso, etc...) for placement in different folders
- [ ] Torrent meta files (eg. associated subtitles)
- [ ] Federation
  - [ ] Following new imports on other instances (RSS/ActivityPub)
  - [ ] Importing from other instances
  - [ ] Auto-importing from other instances (social redundancy/backup)

## Usage

### Command-line usage

For the moment: `torrentmanager` will start the HTTP server.

The `-c/--config` flag will specify a config file to load that's not the default.

### Settings

Settings are defined in the `$XDG_CONFIG_DIR/torrentmanager/config.toml` (usually `~/.config/torrentmanager/config.toml`).

# Architecture

TorrentManager is now a single binary embedding all assets/templates: the only external dependency is a supported torrent client (only qBittorrent v5.0+ at the moment).

The following files/folders are used:

- `$XDG_CONFIG_DIR/torrentmanager/config.toml`: the daemon configuration file
- `$XDG_DATA_DIR/torrentmanager/uploads`: folder where a copy of the user-uploaded magnets/torrents is stored

## Request lifecyle

When a request arrives:

- it's encapsulated in a [timing middleware](src/middleware/timing.rs), which will:
  - add a `x-generation-time: XXms` HTTP header to all responses
  - replace the magic string `__GENERATION_TIME__` in any HTML response
- on most routes, global `AppStateContext` is computed, containing:
  - the username of the logged-in user, or None
  - free space calculation for the configured `media_dir`

Additionally, request handlers which interact with the torrent backend check that it's alive and credentials are good, preventing you from uploading/removing torrents when that's not the case.

## Error management

TorrentManager provides a global `AppStateContext`, extracted from the current request and application state. It should only fail if fetching free space information fails, or if the reverse proxy submits bogus `remote-user` information. Such errors produce a global internal server error which will prevent the route template from rendering:

```rust
fn my_route_handler(State(app_state): State<AppState>) -> Result<impl IntoResponse, AppStateError> {
    // ↓ This will return a global error page
    let mut context = app_state.context().await?;
    // ...
```

Additionally, each route handler may encounters errors which may or may not be fatal to rendering the page. For example:

- failing to parse a form is usually not a fatal error: an error is populated in the `form_errors` field of the route context and the form is re-rendered with submitted values (and an accompanying error message)
- failing to communicate with the torrent client may or may not be fatal depending on the context:
  - in a torrent list view, that's a fatal error: `let list = app_state.torrent_list().await?;`
  - in a torrent upload view, that's a transient form error which should not prevent to re-render the form with previous values: `let res = context.result(app_state.torrent_list().await);`

In the future, non-fatal errors should be added to the current `StateContext`, like so:

```rust
// StateContext::result turns the Result<T, AppStateError> into an Option<T> to avoid
// returning early by accident with the `?` operator.
if let Some(context.result(app_state.torrent_list().await)) {
// …
}
```

In the future, all route handlers will have a signature indicating whether they are fallible. `AppStateContext` will be loaded via a dedicated extractor and will not affect the route signature:

- a fallible route must return `Result<C, AppStateError>`
- a non-fallible route must return `C` (not yet, because `AppStateContext` is loaded in the route)

# TODO

- [x] Keep local database of known torrents
- Import unmanaged torrents (unknown to the local database)
- Upload new torrents to the client
- Keep track of associated files:
  - all files which are not symlinks to torrent content
- Remove torrent content, delete it from the client, and delete symlinks and associated files
- Chores:
  - list symlinks which point to non-existing torrent content (eg. partially removed content)
  - list torrents whose content don't have a single symlink pointing to it: having at least 1 symlink
    enables to filter useless torrent content you don't want to expose, but having 0 means it was
    not properly imported, or was later only partially removed
  - list files in symlink dirs which are not known associated files of a specific torrent

# License

GNU AGPL v3. See [LICENSE](LICENSE) file.
