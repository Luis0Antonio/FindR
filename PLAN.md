# Rust Finder Alternative — Implementation Plan

Target: a macOS file manager app built in Rust. This document is the **only** context the implementing agent has. Follow it; where it says "verify against current docs", do so before writing code, since crate/framework APIs move.

---

## 1. Requirements traceability

Every user requirement, where it lands, and how to check it is done (checked in the **running app**, not only in tests).

| # | Requirement | Milestone | Acceptance check |
|---|---|---|---|
| R1 | Filter/search files by name within the current folder | M3 | Typing in the search box instantly narrows the list (case-insensitive substring). Clearing restores the full list. |
| R2 | Recursive search inside a folder (include subfolders) | M3 | Toggle "Include subfolders": results stream in with their relative path shown. Typing again cancels the previous search. |
| R3 | Create folders | M4 | Cmd+Shift+N, the toolbar button or the context menu creates "untitled folder" (or "untitled folder 2", …) and drops straight into rename mode. |
| R4 | Show only files of certain types | M3 | Type filter chips (multi-select): Folders, Images, Documents, Video, Audio, Archives, Code, Other. Only matching items are shown. |
| R5 | Sort by name, date modified, type or size; ascending/descending | M2 | Clicking a column header sorts by it. Clicking again flips the direction. Sort persists across filtering, searching and navigation. |
| R6 | Change item size | M2 | A slider (and Cmd+= / Cmd+-) scales icon size and row height, 16 px → 64 px. Persisted per pane. |
| R7 | Space bar previews (Quick Look) | M5 (spike in M0) | Space on a selected file opens the native Quick Look panel. Space again or Esc closes it. Arrow keys while it's open move the selection, and the preview follows. |
| R8 | Double-click opens | M2 | Double-clicking a folder navigates into it. Double-clicking a file (or a bundle such as `.app`) opens it in its default app. Cmd+↓ does the same. |
| R9 | Enter renames | M4 | Enter on a single selection turns the name into an inline editor with the stem preselected (not the extension). Enter commits, Esc cancels. |
| R10 | Drag & drop files into folders | M6 | Dragging the selection onto a folder row moves it there. Same volume means move; cross-volume means copy then delete. Holding Option forces copy. |
| R11 | Sidebar with user-chosen quick-access folders | M4 | Drag a folder onto the sidebar, or use "Add to Sidebar" in the context menu, to add it. Remove it via the context menu. Reorder by drag. Persisted. Missing paths show greyed out. Sidebar items are also drop targets. |
| R12 | Split view into two panes | M7 | Cmd+Shift+S or the toolbar button toggles a second pane. Each pane has independent path, history, sort, filter, search, selection and item size. The divider is resizable. |
| R13 | Drag & drop between the two panes | M7 | Dropping onto the other pane's empty area moves into that pane's current folder. Dropping onto a folder row in the other pane moves into that folder. |

**Assumption to confirm with the user:** "Maintaining the order of date or name" is read as *the chosen sort order is kept when filtering, searching or navigating, and is remembered between launches*. If the user meant something else (for example a secondary sort key), adjust M2.

---

## 2. Stack (decided)

- **Tauri v2.** The Rust backend owns all filesystem work. The UI is a web frontend in WKWebView.
- **Frontend:** TypeScript, React, Vite. Use `@tanstack/react-virtual` for the virtualized list (folders can have 100k+ entries) and Zustand for state.
- **Rust crates** (verify current versions and APIs):
  - `jwalk` for parallel recursive walking
  - `notify` and `notify-debouncer-full` for FSEvents watching
  - `natord` for natural sort
  - `serde`, `thiserror` and `tokio` (via Tauri's async runtime)
  - `tauri-plugin-opener` for opening files in their default app
  - `tauri-plugin-store` (or a plain JSON file in the app data dir) for settings
- **macOS native bridges:** `objc2`, `objc2-app-kit`, `objc2-foundation`, `objc2-quick-look-ui` (verify the exact crate names). Use them for Quick Look, file icons and volume detection where `std` isn't enough.
- **Distribution:** Developer ID, outside the App Store, **not sandboxed**. Some locations (Desktop, Documents, Downloads, external volumes) trigger macOS permission prompts, and some need Full Disk Access. Permission errors must show an inline message in the pane ("No permission to read this folder" plus an "Open System Settings" button linking to Privacy & Security). The app must never crash on them.

---

## 3. Milestone 0 — de-risking spike (do this first; STOP and report if it fails)

Two things could sink this stack. Prove both in a throwaway Tauri app before building anything else.

### 0a. Quick Look from a Tauri/WKWebView app
- Goal: pressing a key in the webview calls a Rust command that opens the **native `QLPreviewPanel`** for a given list of paths, at a given index.
- `QLPreviewPanel` needs an object in the responder chain that implements `acceptsPreviewPanelControl:`, `beginPreviewPanelControl:` and `endPreviewPanelControl:`. It also needs a data source and delegate (`numberOfPreviewItemsInPreviewPanel:`, `previewPanel:previewItemAtIndex:`). `NSURL` already conforms to `QLPreviewItem`.
- Approach: use `objc2`'s `define_class!` to make a controller object. Either install it as the window's next responder, or set the panel's `dataSource`/`delegate` directly after `QLPreviewPanel::sharedPreviewPanel()` and call `reloadData` and `makeKeyAndOrderFront`. Run all AppKit calls on the main thread (`app.run_on_main_thread`).
- The panel must also report key events back. When the user presses ↑/↓ inside the panel, the delegate's `previewPanel:handleEvent:` should emit a Tauri event so the UI moves its selection, and the panel's current item should follow.
- **Fallback** if this fails within a reasonable timebox: spawn `qlmanage -p <path>` (a separate, less integrated window). Document the limitation and tell the user.

### 0b. In-app drag & drop in Tauri v2
- Tauri's native file-drop handling (the window option that controls OS file drops; verify its current name in the Tauri v2 docs, likely `dragDropEnabled`) can interfere with HTML5 drag-and-drop inside the webview.
- Prove that you can drag a row element onto another row and onto a second pane, and get the source and target in JS, **while also** still receiving drops of files from Finder into the window (nice to have, not required).
- If HTML5 DnD is unreliable, implement **pointer-event-based DnD**: pointerdown, then a movement threshold of more than 4 px, then a floating drag ghost element, hit-testing via `document.elementFromPoint` against `data-drop-target` attributes, and pointerup to drop. Prefer this path anyway if HTML5 DnD has any quirks. It gives full control over the drag ghost and the Option-key copy indicator.

**Spike output:** a short note at the top of the PR or in the agent's report saying which approach works for 0a and 0b. Delete the spike code afterwards, or turn it into the real modules.

---

## 4. Project layout

```
rfinder/
  Cargo.toml                # workspace
  crates/
    fs-core/                # pure Rust, no Tauri, unit-tested
      src/
        entry.rs            # FileEntry, FileKind
        list.rs             # list_dir
        sort.rs             # sort_entries
        filter.rs           # name filter + kind filter
        kind.rs             # extension → FileKind mapping
        ops.rs              # create_folder, rename, move/copy, conflict detection
        search.rs           # recursive streaming search with cancellation
        names.rs            # unique name generation, name validation
  src-tauri/
    src/
      main.rs
      commands.rs           # #[tauri::command] wrappers over fs-core
      watcher.rs            # per-pane directory watchers
      settings.rs           # load/save settings JSON
      macos/
        quicklook.rs        # QLPreviewPanel bridge
        icons.rs            # NSWorkspace icon → PNG cache
        volumes.rs          # same-volume detection (st_dev)
      icon_protocol.rs      # custom URI scheme serving icons
  ui/
    src/
      App.tsx
      state/                # Zustand stores: panes, settings, dnd, sidebar
      components/
        Sidebar.tsx
        Toolbar.tsx         # back/forward/up, path bar, search, type chips, size slider, split toggle
        Pane.tsx
        FileList.tsx        # virtualized, column headers
        FileRow.tsx         # icon, name / inline rename editor, date, kind, size
        ConflictDialog.tsx
      keyboard.ts           # central key map & focus routing
      dnd.ts                # drag & drop controller
      api.ts                # typed wrappers for invoke() and events
```

---

## 5. Contracts

### 5.1 Data types (Rust, serialized with serde camelCase; mirror them in TS)

```rust
pub enum FileKind { Folder, Package, Image, Document, Video, Audio, Archive, Code, Other }

pub struct FileEntry {
    pub path: String,          // absolute
    pub name: String,          // display name
    pub ext: Option<String>,   // lowercase, no dot
    pub kind: FileKind,
    pub is_dir: bool,          // true for real folders; false for packages (.app, .photoslibrary, …)
    pub is_hidden: bool,       // dotfile or UF_HIDDEN flag
    pub is_symlink: bool,
    pub size: Option<u64>,     // None for folders (don't compute recursive sizes)
    pub modified: Option<i64>, // unix ms
    pub created: Option<i64>,  // unix ms
}

pub enum SortKey { Name, Modified, Kind, Size }
pub enum SortDir { Asc, Desc }

pub enum ConflictPolicy { Replace, KeepBoth, Skip }

pub struct OpResult {
    pub done: Vec<String>,             // resulting paths
    pub conflicts: Vec<Conflict>,      // returned when no policy was given
    pub errors: Vec<OpError>,          // { path, message }
}
pub struct Conflict { pub source: String, pub target: String }
```

**Packages:** treat directories with a package extension (`app`, `bundle`, `framework`, `photoslibrary`, `pkg`, `rtfd`, …) as **files**. They get `kind = Package` and `is_dir = false`, double-click opens them, and they are never drop targets. Prefer asking `NSWorkspace.isFilePackageAtPath:` via `objc2`, with the extension list as the fallback.

**Kind mapping (`kind.rs`):** a static extension → kind table. It must at least cover:
- **Image:** png jpg jpeg gif heic webp tiff bmp svg raw
- **Document:** pdf doc docx pages txt md rtf key ppt pptx numbers xls xlsx csv
- **Video:** mp4 mov m4v avi mkv webm
- **Audio:** mp3 m4a wav aiff flac aac ogg
- **Archive:** zip tar gz tgz bz2 xz 7z rar dmg
- **Code:** rs ts tsx js jsx py go java c h cpp swift kt rb sh json yaml yml toml html css sql

Anything else is Other.

### 5.2 Commands (`#[tauri::command]`, all async; errors returned as `{ code, message }`)

| Command | Signature | Notes |
|---|---|---|
| `list_dir` | `(path, show_hidden) -> Vec<FileEntry>` | Unsorted; the UI sorts. Must handle 100k entries in under ~1 s. Error code `permission_denied` for EACCES/EPERM. |
| `open_path` | `(path)` | Default app via `tauri-plugin-opener`. |
| `create_folder` | `(parent) -> FileEntry` | Picks a unique "untitled folder[ N]" name. |
| `rename` | `(path, new_name) -> FileEntry` | Validates the name (§7.3). Fails with `exists` if the target exists. Case-only renames (`a.txt` → `A.txt`) must work on case-insensitive APFS. |
| `move_items` | `(sources, target_dir, copy: bool, policy: Option<ConflictPolicy>) -> OpResult` | See §7.4. |
| `start_search` | `(root, query, kinds, show_hidden) -> search_id` | Streams `search://results` events. |
| `cancel_search` | `(search_id)` | |
| `watch_dir` | `(pane_id, path)` | Replaces that pane's previous watcher. |
| `quicklook_show` | `(paths, index)` | |
| `quicklook_update` | `(paths, index)` | Called when the selection changes while the panel is open. |
| `quicklook_close` | `()` | |
| `volume_of` | `(path) -> u64` | `st_dev`, used for the move-vs-copy cursor hint. |
| `load_settings` / `save_settings` | `(Settings)` | |

### 5.3 Events (backend → UI)

| Event | Payload |
|---|---|
| `dir://changed` | `{ paneId, path }`, debounced 150 ms; the UI re-calls `list_dir` and preserves selection and scroll |
| `search://results` | `{ searchId, entries: FileEntry[], done: bool }`, batched every ~50 ms or 200 entries |
| `quicklook://navigate` | `{ delta: -1 \| 1 }`, from the arrow keys inside the panel |
| `quicklook://closed` | `{}` |
| `op://progress` | `{ opId, doneBytes, totalBytes }`, for cross-volume copies over ~50 MB (simple progress bar in the status bar) |

### 5.4 Icons
Register a custom URI scheme (`icon://`, verify the Tauri v2 API: `register_asynchronous_uri_scheme_protocol`). `icon://<size>/<urlencoded path>` returns a PNG from `NSWorkspace.iconForFile:`, rendered at the requested size times 2 for Retina.

Cache it in memory (an LRU of about 2,000 entries) keyed by:
- extension and size for regular files
- path and size for folders, packages and anything with a custom icon

Thumbnails (`QLThumbnailGenerator`) are **out of scope** for v1.

---

## 6. UI state model

```ts
type PaneId = 'left' | 'right';

interface PaneState {
  path: string;
  history: string[]; historyIndex: number;
  entries: FileEntry[];          // raw from list_dir
  sortKey: SortKey; sortDir: SortDir;
  foldersFirst: boolean;         // default true
  nameFilter: string;            // instant filter text
  kindFilter: Set<FileKind>;     // empty = all
  recursive: boolean;            // search includes subfolders
  searchResults: FileEntry[] | null; searchId: string | null;
  selection: Set<string>;        // paths
  anchor: string | null;         // for shift-range selection
  cursor: string | null;         // focused row
  renaming: string | null;       // path being renamed
  itemSize: number;              // 16–64 px
  error: { code: string; message: string } | null;
}

interface AppState {
  panes: Record<PaneId, PaneState>;
  splitEnabled: boolean;
  splitRatio: number;            // 0.2–0.8
  activePane: PaneId;            // receives keyboard input; highlighted border
  showHidden: boolean;
  favorites: string[];           // sidebar paths, ordered
}
```

- `visibleEntries(pane)` is a memoized selector. It runs `(searchResults ?? entries)`, then `kindFilter`, then `nameFilter` (when not recursive), then sort.
- Clicking anywhere in a pane makes it `activePane`. Sidebar clicks navigate the active pane.
- **Persisted settings** (`settings.json` in the app data dir): `favorites`, `splitEnabled`, `splitRatio`, `showHidden`, plus per pane: `path`, `sortKey`, `sortDir`, `foldersFirst` and `itemSize`. Defaults: a single pane at `~`, sorted by name ascending, item size 20, and favorites `~`, `~/Desktop`, `~/Documents` and `~/Downloads`.

---

## 7. Behavior specs

### 7.1 Sorting (R5)
- Sort with `foldersFirst` (default on) as the primary group, then the chosen key, then **name ascending as the stable tie-breaker**.
- **Name** uses a natural, case-insensitive comparison (`file2` < `file10`). Use `Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })` in TS, and `natord` in Rust if sorting there.
- **Modified** compares `modified`, with None last.
- **Kind** sorts by kind label, then by extension.
- **Size** compares `size`. Folders have no size: they sort as 0 and show "--".
- The column header shows ▲/▼ on the active key.

### 7.2 Filter & search (R1, R2, R4)
- The search box (Cmd+F focuses it) filters the current folder instantly with a case-insensitive substring match on `name`.
- The "Include subfolders" toggle switches to recursive mode:
  - Debounce input by 200 ms, then call `start_search` and cancel any previous search.
  - Results stream in and show an extra "Folder" column with the path relative to the pane's path.
  - Show a spinner until `done`.
  - The walk skips hidden entries unless `showHidden` is on, and never descends into packages.
  - Cancellation uses an `Arc<AtomicBool>` checked inside the `jwalk` loop.
  - Cap at 10,000 results and show "Showing first 10,000".
- Type chips apply to both modes. Multi-select is an OR across kinds. A "Clear" chip resets them.
- Esc in the search box clears it and returns focus to the list.
- Show hidden files with Cmd+Shift+. (default off, as in Finder).

### 7.3 Rename & create folder (R3, R9)
- Enter with exactly one selected item and focus in the list opens the inline editor:
  - It preselects the name without its extension. Folders and packages are fully selected.
  - Enter commits. Esc cancels. Blur commits.
- Name validation:
  - Reject names that are empty, `.`, `..`, contain `/`, or exceed 255 UTF-8 bytes.
  - On an `exists` error, show an inline error and keep the editor open.
  - Changing the extension needs no confirmation dialog in v1.
- **While renaming, don't re-sort or refresh the list under the editor.** Queue watcher refreshes until the rename ends, then re-sort and keep the renamed item selected and scrolled into view.
- Create folder works from Cmd+Shift+N, the toolbar button, or the context menu on an empty area. Then:
  1. The backend creates the "untitled folder" (or "untitled folder 2", …) in the pane's current folder.
  2. The UI inserts it, selects it, scrolls to it and enters rename mode immediately.
  3. In recursive search mode, the folder is created in the pane's current path.

### 7.4 Drag & drop (R10, R11, R13)
- **Drag source:** the current selection. Dragging an unselected row first selects only that row.
- **Drop targets:**
  - (a) folder rows (not packages)
  - (b) the empty area or background of a pane, meaning that pane's current folder
  - (c) sidebar favorites
  - (d) the sidebar's "add" zone, which adds the folder as a favorite instead of moving it
- **Feedback:** highlight the hovered target. When the drag hovers a folder row for 1 s, the pane navigates into it (spring-loading; nice to have, can be cut).
- **Invalid targets** (show a not-allowed cursor and do nothing):
  - the source's own parent (no-op)
  - a source folder itself
  - **any descendant of a source folder**
  - packages
- **Move vs copy:**
  - Same volume (`st_dev` equal) means move, via `std::fs::rename`.
  - Different volume means copy then delete. Also fall back to this when `rename` returns `EXDEV`.
  - Holding **Option** during the drop forces copy. Show a "+" badge on the drag ghost.
- **Conflicts:** the first call passes `policy: None`. If `conflicts` is non-empty, show `ConflictDialog` ("An item named X already exists": Replace / Keep Both / Skip, plus an "Apply to all" checkbox). Then re-call with the chosen policy for those items.
  - **Keep Both** names the file `name 2.ext`.
  - **Replace** moves the existing target to the Trash (`trash` crate), not a hard delete.
- **After the drop:** both affected directories refresh through the watcher. The dropped items are selected in the target pane if it shows the target folder.
- Copies preserve modification times. Use a recursive copy that keeps symlinks as links. Don't follow them.

### 7.5 Open & preview (R7, R8)
- Double-click or Cmd+↓ on a folder navigates. On a file or package it calls `open_path`. On multiple selected files it opens each.
- **Space** with focus in the list and a non-empty selection calls `quicklook_show(visibleSelectionPaths, indexOfCursor)`. Space again or Esc calls `quicklook_close`.
  - While the panel is open, ↑/↓ in the list still move the cursor and call `quicklook_update`.
  - `quicklook://navigate` from the panel moves the cursor the same way.

### 7.6 Navigation (minimal, required to make R8 usable)
- Back/forward buttons: Cmd+[ and Cmd+].
- Up to parent: Cmd+↑.
- A clickable path bar (breadcrumb) at the top of each pane.
- Cmd+Shift+G opens "Go to folder" with a text path input.

### 7.7 Watching
- One `notify` debounced watcher per pane, non-recursive on the pane's path. Emit `dir://changed`.
- If both panes show the same path, both refresh.
- If the watched folder is deleted, navigate the pane to the nearest existing ancestor.

### 7.8 Sidebar (R11)
- A "Favorites" section, plus a "Locations" section listing mounted volumes (read from `/Volumes`). Clicking an item navigates the active pane.
- Add a favorite by dragging a folder to the sidebar, or with "Add to Sidebar" in the row context menu or Cmd+Ctrl+T.
- Remove it via the context menu. Reorder by dragging within the sidebar.
- Missing paths render greyed out. Clicking one offers "Remove from Sidebar".

### 7.9 Split view (R12, R13)
- Cmd+Shift+S toggles split. When it's turned on, the right pane opens at the left pane's current path.
- The divider is draggable, clamped to 20–80 %.
- Tab and Shift+Tab (when focus isn't in a text field) switch `activePane`. The active pane has an accent border.
- Each pane has its own toolbar row (path bar, search, type chips, size slider). The sidebar and the split toggle are global.

---

## 8. Keyboard map & focus routing (`keyboard.ts`)

All global shortcuts go through one handler. **When an `<input>` has focus (search box, rename editor, go-to-folder), only Esc and Enter are handled by that input. Space, Enter-to-rename, arrows, Cmd+A and the rest must not fire.**

| Key | Action (in the active pane's list) |
|---|---|
| ↑ / ↓ | Move cursor; select single |
| Shift+↑ / ↓ | Extend selection from anchor |
| Cmd+A | Select all visible |
| Click / Cmd+Click / Shift+Click | Select / toggle / range |
| Space | Quick Look toggle |
| Enter | Rename (single selection) |
| Cmd+↓ / double-click | Open |
| Cmd+↑ | Parent folder |
| Cmd+[ / Cmd+] | Back / forward |
| Cmd+Shift+N | New folder |
| Cmd+F | Focus search |
| Cmd+= / Cmd+- | Item size up / down |
| Cmd+Shift+. | Toggle hidden files |
| Cmd+Shift+S | Toggle split |
| Tab / Shift+Tab | Switch active pane |
| Cmd+Shift+G | Go to folder |
| Esc | Clear selection / close Quick Look / cancel rename / clear search |
| Typing letters | Type-to-select: jump to the first name starting with the typed prefix (500 ms buffer). Nice to have. |

---

## 9. Milestones (each one ends in a runnable app, verified by hand in the app)

| Milestone | Scope | Done when |
|---|---|---|
| **M0** | Spike (§3) | A report on Quick Look and DnD feasibility, with the chosen approaches |
| **M1** | Scaffold the workspace, `fs-core::list_dir`, a single pane listing `~` with name/date/kind/size columns, real icons via `icon://`, virtualized list | Opening `/usr/lib` or any folder with 10k+ files scrolls smoothly. Permission errors show inline. |
| **M2** | Sorting (R5), item-size slider (R6), double-click/open and navigation (R8, §7.6), selection model, keyboard map basics, settings persistence | R5, R6 and R8 acceptance checks pass, and settings survive a restart |
| **M3** | Instant filter (R1), type chips (R4), recursive streaming search (R2), hidden-files toggle, watcher (§7.7) | R1, R2 and R4 pass. Creating a file in Terminal appears in the app within about 0.5 s. |
| **M4** | Rename (R9), create folder (R3), sidebar favorites, add/remove/reorder (R11, minus DnD-to-add), volumes list | R3, R9 and R11 (non-DnD parts) pass |
| **M5** | Quick Look (R7) | R7 passes, including arrow navigation while it's open |
| **M6** | Drag & drop within a pane and onto the sidebar (R10, R11 DnD parts), conflict dialog, cross-volume copy, Option-to-copy | R10 passes, including a drop onto a USB/external drive or a disk image (`hdiutil create`) and a conflict case |
| **M7** | Split view (R12) and cross-pane DnD (R13) | R12 and R13 pass |
| **M8** | Polish: context menus (New Folder, Rename, Open, Quick Look, Add to Sidebar, Show in Finder), empty-state messages, the app icon, a signed `.app` build via `tauri build` | The app launches from `/Applications` and every R-check passes in one session |

**Tests:** unit-test `fs-core` against temp directories (`tempfile`). Cover the natural sort, the kind mapping, unique-name generation, name validation, conflict detection, descendant-drop rejection and the cross-volume fallback logic. UI behavior is verified by hand in the running app at each milestone.

---

## 10. Out of scope for v1 (suggested next steps; do not build these now)

Move to Trash / delete (Cmd+Backspace), copy/cut/paste, undo, tabs, grid/icon view and column view, thumbnails, drag-out to other apps and drop-in from Finder (keep it if the M0 spike gets it for free), Finder tags, Get Info, file permissions editing, iCloud status, network shares, compress/extract, folder sizes, and Spotlight-backed search.
