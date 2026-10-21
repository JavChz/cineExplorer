# Open Source License & Third-Party Software Compliance

`cineExplorer` is committed to full transparency and compliance with open source licenses. This document contains detailed attribution, licenses, and legal notices for all third-party software, libraries, and tools utilized by `cineExplorer`.

---

## 1. External Media Processing Engines: FFmpeg & FFprobe

`cineExplorer` relies on the open-source **FFmpeg** multimedia framework (specifically `ffmpeg` and `ffprobe`) for media container inspection, video stream metadata extraction, and thumbnail generation.

### Architecture & Compliance Notice
- **Process Separation**: `cineExplorer` does **not** link directly (statically or dynamically) against FFmpeg shared libraries (`libavcodec`, `libavformat`, etc.). Instead, `cineExplorer` interacts with FFmpeg strictly as an independent command-line subprocess via standard operating system process spawning (`std::process::Command`) and inter-process I/O pipes.
- **Licensing**: FFmpeg is licensed under the **GNU Lesser General Public License (LGPL) version 2.1 or later**, or the **GNU General Public License (GPL) version 2 or later** (depending on build configuration and third-party encoders enabled such as `libx264` or `libx265`).
- **No Derivative Work of FFmpeg**: Because `cineExplorer` communicates with FFmpeg across an independent process boundary via standard CLI arguments and stdout pipes, `cineExplorer` is an independent work and does not alter the licensing terms of FFmpeg.
- **Source Code Availability**: The complete source code of FFmpeg is publicly available at:
  - Official Website: [https://ffmpeg.org](https://ffmpeg.org)
  - Source Repository: [https://git.ffmpeg.org/ffmpeg.git](https://git.ffmpeg.org/ffmpeg.git)
  - GitHub Mirror: [https://github.com/FFmpeg/FFmpeg](https://github.com/FFmpeg/FFmpeg)
- **User Modifiability**: Users and developers may freely upgrade, replace, recompile, or modify the `ffmpeg.exe` and `ffprobe.exe` binaries located on their system `PATH` without affecting the compilation of `cineExplorer`.

---

## 2. Iconography: Phosphor Icons

The user interface icons in `cineExplorer` are provided by the **Phosphor Icons** library via `egui-phosphor`.

- **Copyright (c)**: 2020–2026 Phosphor Icons (Helena Zhang & Tobias Fried)
- **License**: MIT License
- **Repository**: [https://github.com/phosphor-icons/core](https://github.com/phosphor-icons/core)

```text
MIT License

Copyright (c) 2020 Phosphor Icons

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## 3. Database Engine: SQLite

`cineExplorer` catalogs are stored using **SQLite3** through the `rusqlite` crate with bundled SQLite.

- **SQLite License**: Public Domain (The SQLite Blessing)
- **Blessing Text**:
  > *May you do good and not evil.*  
  > *May you find forgiveness for yourself and forgive others.*  
  > *May you share freely, never taking more than you give.*  
- **Official Website**: [https://www.sqlite.org](https://www.sqlite.org)

---

## 4. Rust Crates & Libraries

The following Rust libraries are compiled into the `cineExplorer` binary. All are licensed under permissive open source licenses (MIT, Apache-2.0, Zlib, or Unlicense):

| Crate | Version | License | Primary Authors / Maintainers | Repository |
| :--- | :--- | :--- | :--- | :--- |
| **`eframe`** | 0.29 | MIT / Apache-2.0 | Emil Ernerfeldt | [github.com/emilk/egui](https://github.com/emilk/egui) |
| **`egui`** | 0.29 | MIT / Apache-2.0 | Emil Ernerfeldt | [github.com/emilk/egui](https://github.com/emilk/egui) |
| **`egui-phosphor`** | 0.7 | MIT | Luc Cooper | [github.com/lucc/egui-phosphor](https://github.com/lucc/egui-phosphor) |
| **`rusqlite`** | 0.32 | MIT | John Gallagher & contributors | [github.com/rusqlite/rusqlite](https://github.com/rusqlite/rusqlite) |
| **`bytemuck`** | 1.16 | MIT / Apache-2.0 | Lokathor | [github.com/Lokathor/bytemuck](https://github.com/Lokathor/bytemuck) |
| **`serde`** | 1.0 | MIT / Apache-2.0 | Erick Tryzelaar, David Tolnay | [github.com/serde-rs/serde](https://github.com/serde-rs/serde) |
| **`serde_json`** | 1.0 | MIT / Apache-2.0 | Erick Tryzelaar, David Tolnay | [github.com/serde-rs/json](https://github.com/serde-rs/json) |
| **`rayon`** | 1.10 | MIT / Apache-2.0 | Niko Matsakis, Josh Stone | [github.com/rayon-rs/rayon](https://github.com/rayon-rs/rayon) |
| **`crossbeam-channel`** | 0.5 | MIT / Apache-2.0 | The Crossbeam Project | [github.com/crossbeam-rs/crossbeam](https://github.com/crossbeam-rs/crossbeam) |
| **`walkdir`** | 2.5 | Unlicense / MIT | Andrew Gallant (BurntSushi) | [github.com/BurntSushi/walkdir](https://github.com/BurntSushi/walkdir) |
| **`rfd`** (Rusty File Dialogs)| 0.15 | MIT | PolyMeilex | [github.com/PolyMeilex/rfd](https://github.com/PolyMeilex/rfd) |
| **`open`** | 5.3 | MIT | Sam Geeraerts, Maciej Hirsz | [github.com/Byron/open-rs](https://github.com/Byron/open-rs) |
| **`chrono`** | 0.4 | MIT / Apache-2.0 | Kang Seonghoon, Brandon W Maister | [github.com/chronotope/chrono](https://github.com/chronotope/chrono) |
| **`image`** | 0.25 | MIT / Apache-2.0 | The PistonDevelopers | [github.com/image-rs/image](https://github.com/image-rs/image) |
| **`rodio`** | 0.19 | MIT / Apache-2.0 | Pierre Krieger | [github.com/RustAudio/rodio](https://github.com/RustAudio/rodio) |
| **`trash`** | 5.2 | MIT | Byron | [github.com/Byron/trash-rs](https://github.com/Byron/trash-rs) |
| **`wgpu`** | 22.1 | MIT / Apache-2.0 | Wgpu Project Contributors | [github.com/gfx-rs/wgpu](https://github.com/gfx-rs/wgpu) |

---

## 5. Common Permissive License Texts

### The MIT License
```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### The Apache License, Version 2.0 (Summary)
Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at:
`http://www.apache.org/licenses/LICENSE-2.0`

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
