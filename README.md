# cineExplorer 🎬

A blazing-fast, modern desktop video browser, catalog, and inspector built in **Rust** for professional video workflows.

Designed for filmmakers, colorists, DITs, and video editors working with commercial and cinema camera formats: **MP4, MOV, BRAW (Blackmagic RAW), MKV, MXF, ProRes, H.264, H.265 (HEVC), 8-bit, 10-bit, and 12-bit deep color**.

---

## Key Features

### 📁 Independent SQLite Catalogs (`.cinecat`)
- Each catalog is stored as an independent, lightweight, high-performance SQLite database.
- Create, open, and switch between separate catalogs (e.g. per project, client, or shoot).
- Recent catalogs list for instant switching.
- Stores comprehensive video metadata, camera settings, tags, user ratings, color labels, production slate, and thumbnail caches.

### 🎥 Broad Commercial & Pro Video Format Support
- **Containers**: MP4, MOV, BRAW (Blackmagic RAW), MKV, MXF, AVI, WebM, M4V, TS, MTS.
- **Codecs**: Apple ProRes (Proxy, LT, 422, HQ, 4444, 4444 XQ), H.265 / HEVC, H.264 / AVC, Blackmagic RAW (BRAW), Avid DNxHD/DNxHR, AV1, VP9.
- **Bit Depths**: 8-bit, 10-bit, and 12-bit deep color detection.
- **Color Spaces**: Rec.709, BT.2020, DCI-P3, HDR10 (PQ / SMPTE 2084), HLG.

### 📥 High-Speed Multi-Threaded Ingestion
- Recursive folder scanner with duplicate detection.
- Asynchronous metadata extraction using `ffprobe`.
- Background thumbnail extraction via `ffmpeg` with SQLite BLOB caching.
- Non-destructive "Link in place" cataloging.

### 🗂️ Rich Visual Grid & Smart Collections
- Hardware-accelerated virtualized card grid (powered by `egui` and `wgpu`).
- **Interactive Hover Scrubbing**: Hover and glide horizontally across cards for instant timecode and scrub preview.
- **Visual Badges**: Badges for 10-bit deep color, 4K+ UHD, codec (ProRes, BRAW, HEVC), frame rate, duration, and audio tracks.
- **Smart Collections**: One-click filtering by 10-bit+, 4K+ UHD, High Frame Rate (≥50fps), ProRes, BRAW, HEVC, H.264, Camera models, Containers, and Tags.
- **Rating & Status**: 1-5 Star ratings, Pick / Reject flags, and color labels.

### 🔍 Comprehensive Metadata Inspector
- **Tech Specs**: Codec, profile, exact resolution, aspect ratio, frame rate, frame count, duration, bit depth, pixel format, color space, transfer function, and bitrate.
- **Audio Specs**: Codec, channels, and sample rate.
- **Camera & Lens**: Camera make/model, lens, focal length, aperture, ISO, shutter speed, white balance, and capture date.
- **Production Slate**: Editable Scene, Shot, Take, Reel, Camera Unit, and Location fields.
- **User Notes & Tags**: Editable notes and custom colored tags.

### 🎞️ Integrated Video Player & Scrubber
- Clean cinematic player canvas with aspect-ratio letterboxing.
- Real-time scrubbing, play/pause, in/out points, and loop toggling.
- Frame stepping (1-frame and 10-frame step with arrow keys or J-K-L).
- Quick launch into default external player (VLC, MPV, QuickTime, DaVinci Resolve).

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause |
| `Left Arrow` | Step backward 1 frame |
| `Right Arrow` | Step forward 1 frame |
| `Shift + Left Arrow` | Step backward 10 frames |
| `Shift + Right Arrow` | Step forward 10 frames |
| `Tab` / `Double Click` | Toggle Integrated Player View |
| `Esc` | Exit Player View / Close dialogs |
| `1` – `5` | Set Star Rating (1 to 5) |
| `P` | Mark as Pick |
| `X` | Mark as Rejected |
| `U` | Clear Pick/Reject flag |

---

## Architecture

```
d:\Dev\githubprojects\cineExplorer/
├── Cargo.toml                  # Cargo dependencies and build profiles
├── README.md                   # Documentation
├── .gitignore
└── src/
    ├── main.rs                 # eframe application entry point & wgpu window setup
    ├── app.rs                  # Main application state, UI router, event handling
    ├── config.rs               # Application settings & recent catalogs persistence
    ├── db/
    │   ├── mod.rs              # Database connection manager
    │   ├── models.rs           # VideoRecord, PickFlag, SmartCollection, Tag models
    │   ├── queries.rs          # SQLite CRUD, filtering, sorting, and stats
    │   └── schema.rs           # SQLite schema and migration definitions
    ├── media/
    │   ├── mod.rs              # Supported video extensions and OS process config
    │   ├── probe.rs            # ffprobe JSON extractor for tech specs and camera EXIF
    │   ├── thumbnailer.rs      # ffmpeg JPEG frame extractor
    │   └── player_source.rs    # Video player frame extraction and seek engine
    ├── importer.rs             # Multi-threaded background scanner & ingester
    ├── playback.rs             # Playhead transport and frame caching engine
    ├── thumbnails.rs           # Thumbnail texture manager and cache worker
    └── ui/
        ├── mod.rs              # UI coordination
        ├── theme.rs            # Pro dark cinematic palette & fonts
        ├── widgets.rs          # Badges, star rating, pick flags, color swatches
        ├── top_bar.rs          # Header toolbar with search, filters, zoom, catalog switch
        ├── navigator.rs        # Left sidebar smart collections & tag tree
        ├── grid/               # Virtualized video card grid
        ├── inspector/          # Metadata inspector and production slate panel
        ├── player/             # Integrated video player & transport scrubber
        └── dialogs/            # Catalog, Ingest, and Delete modal dialogs
```

---

## Prerequisites

- **Rust** 1.75+ (`cargo`, `rustc`)
- **FFmpeg & FFprobe** available in your system `PATH` (used for video inspection and frame extraction).

---

## Build & Run

```bash
# Debug build
cargo run

# Optimized release build
cargo build --release
./target/release/cine-explorer.exe
```

---

## License

MIT OR Apache-2.0
