# cineExplorer 🎬

[![CI Build & Test](https://github.com/javch/cineExplorer/actions/workflows/ci.yml/badge.svg)](https://github.com/javch/cineExplorer/actions/workflows/ci.yml)
[![Release Build](https://github.com/javch/cineExplorer/actions/workflows/release.yml/badge.svg)](https://github.com/javch/cineExplorer/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 1.75+](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Platform: Windows | macOS | Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

A blazing-fast, modern desktop video catalog, browser, and metadata inspector built in **Rust** for professional filmmaking, post-production, DIT, and color grading workflows.

`cineExplorer` is engineered for commercial and cinema camera formats: **MP4, MOV, BRAW (Blackmagic RAW), MKV, MXF, ProRes, H.264, H.265 (HEVC), 8-bit, 10-bit, and 12-bit deep color**, with direct project export into **DaVinci Resolve** and **Adobe Premiere Pro**.

---

## Highlights

- 📁 **Independent SQLite Catalogs (`.cinecat`)**: Every catalog is a self-contained, lightning-fast SQLite database. Keep separate catalogs for clients, projects, or shooting days.
- 🎞️ **Comprehensive Pro Format Support**: Native parsing for MP4, MOV, BRAW, MKV, MXF, AVI, WebM, ProRes (all profiles), H.265 / HEVC, H.264 / AVC, DNxHD/HR, AV1.
- 🎨 **10-bit & 12-bit Deep Color Detection**: Automatic identification of 10-bit and 12-bit log and HDR material with visual badges.
- 🚀 **Direct NLE Export with Keyword Smart Bins**: One-click export into **DaVinci Resolve** (`.fcpxml`) and **Adobe Premiere Pro** (`.xml`). User tags and technical specs automatically convert into DaVinci Resolve **Keyword Smart Bins**!
- ⚡ **Interactive Hover Scrubbing**: Hover and glide horizontally across video cards to preview frames and timecodes in real time.
- 📥 **High-Speed Multi-Threaded Ingestion**: Non-destructive indexing via background workers with automated duplicate detection and thumbnail caching.
- 🔍 **Deep Technical & Slate Inspector**: Inspect codecs, profiles, frame rates, color spaces (Rec.709, Rec.2020, HDR10, HLG), audio channels, camera/lens EXIF, and edit production slate metadata (`Scene`, `Shot`, `Take`, `Reel`).
- ⏯️ **Integrated Video Player**: Large video viewport with aspect-ratio letterboxing, frame stepping (1 frame / 10 frames), In/Out markers, and keyboard shortcuts.

---

## Supported Video Formats & Codecs

| Category | Supported Formats / Codecs |
| :--- | :--- |
| **Containers** | `.mp4`, `.mov`, `.braw`, `.mxf`, `.mkv`, `.avi`, `.webm`, `.m4v`, `.ts`, `.mts`, `.m2ts`, `.flv` |
| **Video Codecs** | Apple ProRes (Proxy, LT, 422, HQ, 4444, 4444 XQ), H.265 / HEVC, H.264 / AVC, Blackmagic RAW (BRAW), Avid DNxHD/DNxHR, AV1, VP9 |
| **Bit Depths** | 8-bit, 10-bit, 12-bit (Deep Color / HDR / RAW) |
| **Color Spaces** | Rec.709, BT.2020, DCI-P3, HDR10 (SMPTE 2084 / PQ), HLG (ARIB STD-B67), Camera Log curves |
| **Audio Codecs** | Linear PCM (16/24-bit), AAC, AC-3, E-AC-3, Opus, MP3, FLAC (1–8 channels) |

---

## Multi-Editor NLE Integration (DaVinci Resolve & Premiere Pro)

Instead of manually re-importing and re-tagging footage in your editor, `cineExplorer` creates ready-to-cut project files:

### 1. DaVinci Resolve (`.fcpxml`)
- Generates **FCPXML 1.9** files that load directly into DaVinci Resolve.
- Injects `<keyword value="..."/>` tags for user tags (e.g. `Exterior`, `Drone`, `Interview`) and technical attributes (`10-bit`, `4K UHD`, `ProRes HQ`, `Sony FX3`, `Pick`).
- **Resolve automatically creates Keyword Smart Bins** in your Media Pool, grouping your clips instantly!
- Transmits production slate (`Scene`, `Shot`, `Take`, `Reel`, `Camera Name`) into Resolve's native Media Pool columns.
- One-click **"🎬 Export & Open in DaVinci Resolve"** writes the file and launches Resolve automatically.

### 2. Adobe Premiere Pro (`.xml`)
- Generates **Final Cut Pro 7 XML (`<xmeml version="5">`)** compatible with Adobe Premiere Pro.
- Creates a dedicated Bin with all media files linked, log notes containing keywords, and sequence tracks.

### 3. DaVinci Resolve Media Pool Metadata (`.csv`)
- Native CSV export formatted for Resolve's `File > Import > Media Pool Metadata...` feature.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Space` | Play / Pause video |
| `Left Arrow` | Step backward 1 frame |
| `Right Arrow` | Step forward 1 frame |
| `Shift + Left Arrow` | Step backward 10 frames |
| `Shift + Right Arrow` | Step forward 10 frames |
| `Tab` / `Double Click` | Toggle Integrated Player View |
| `Esc` | Return to Grid View / Close modal dialogs |
| `1` – `5` | Set Star Rating (1 to 5 stars) |
| `P` | Mark clip as **Pick** |
| `X` | Mark clip as **Rejected** |
| `U` | Clear Pick/Reject flag (**Unflag**) |

---

## Architecture & Codebase Map

For architectural details, borrow checker patterns, and development guides for AI agents or contributors, refer to [**`AGENTS.md`**](AGENTS.md).

```text
src/
├── main.rs                 # eframe application entry point & wgpu window initialization
├── app.rs                  # CineExplorerApp state, UI routing, shortcuts
├── config.rs               # Persistent application settings (recent catalogs, zoom, preferences)
├── db/
│   ├── mod.rs              # Database connection wrapper
│   ├── models.rs           # VideoRecord, PickFlag, SmartCollection, Tag models
│   ├── queries.rs          # Filtering, sorting, and metadata queries
│   └── schema.rs           # SQLite schema and migration definitions
├── media/
│   ├── mod.rs              # Supported extensions & Windows process configuration
│   ├── probe.rs            # ffprobe JSON extractor for technical specs & EXIF
│   ├── thumbnailer.rs      # ffmpeg JPEG frame extraction for thumbnail caching
│   └── player_source.rs    # Video player frame extraction & seeking engine
├── export/
│   ├── mod.rs              # NLE export coordination
│   ├── resolve_fcpxml.rs   # DaVinci Resolve FCPXML 1.9 exporter with smart keywords
│   ├── premiere_xml.rs     # Adobe Premiere Pro FCP 7 XML exporter
│   ├── metadata_csv.rs     # DaVinci Resolve Media Pool CSV exporter
│   └── launcher.rs         # Local NLE detection (Resolve / Premiere) & launch engine
├── importer.rs             # Multi-threaded background scanner & ingester
├── playback.rs             # Video player engine & frame read-ahead
├── thumbnails.rs           # Thumbnail texture caching & SQLite sync
└── ui/
    ├── theme.rs            # Cinematic dark palette, typography, badges
    ├── widgets.rs          # Badges, star rating, pick flags, color swatches
    ├── top_bar.rs          # Header toolbar with search, filters, zoom, export
    ├── navigator.rs        # Sidebar smart collections, formats, codecs, tags
    ├── grid/               # Virtualized video cards with hover scrubber
    ├── inspector/          # Collapsible tech specs, camera EXIF, slate, notes
    ├── player/             # Integrated video player viewport & transport controls
    └── dialogs/            # Ingest, Catalog, Delete, and Export modal dialogs
```

---

## Installation & Build Instructions

### Prerequisites
1. **Rust Toolchain**: Install Rust 1.75+ via [rustup.rs](https://rustup.rs/).
2. **FFmpeg & FFprobe**: Ensure `ffmpeg` and `ffprobe` are installed and accessible on your system `PATH`:
   - **Windows**: `winget install Gyan.FFmpeg` or `choco install ffmpeg`
   - **macOS**: `brew install ffmpeg`
   - **Linux**: `sudo apt install ffmpeg`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/javch/cineExplorer.git
cd cineExplorer

# Run tests
cargo test

# Run development build
cargo run

# Build optimized release binary
cargo build --release
```

The compiled release binary is located at `target/release/cine-explorer` (or `cine-explorer.exe` on Windows).

---

## Open Source Compliance & Legal Notice

`cineExplorer` is licensed under the [**MIT License**](LICENSE).

This application communicates with **FFmpeg** and **FFprobe** strictly across an independent process boundary via standard operating system CLI calls and pipes, adhering to LGPL/GPL licensing boundaries without static or dynamic linking. For a complete inventory of all third-party libraries, author credits, and license disclosures, please read [**`THIRD_PARTY_LICENSES.md`**](THIRD_PARTY_LICENSES.md) and [**`NOTICE`**](NOTICE).
