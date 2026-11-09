# Alternance Finder (v2.0.0)

[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev/)
[![SQLite](https://img.shields.io/badge/SQLite-WAL%20Mode-003B57?style=flat-square&logo=sqlite&logoColor=white)](https://sqlite.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

Native macOS desktop application engineered with Rust (Tauri v2) and React 19 to automate the work-study (*alternance*) job search. Alternance Finder aggregates postings concurrently across major French employment portals and APIs, extracts skills from PDF resumes with strict boundary heuristics, computes calibrated match percentages, and tracks applications entirely offline in an embedded SQLite database.

> **Résumé en Français** : Application desktop native développée avec Rust (Tauri v2) et React 19 pour automatiser la recherche d'alternance. L'application agrège en parallèle les offres des principales plateformes françaises (*Welcome to the Jungle, HelloWork, LinkedIn, Indeed, Jobijoba, La Bonne Alternance*), extrait les compétences des CV PDF sans hallucination via des expressions régulières à frontières strictes, calcule un score d'adéquation pondéré, automatise la rédaction de brouillons d'e-mails et persiste les données en local dans SQLite en mode WAL.

---

## Architecture Overview

```text
+---------------------------------------------------------------------------------------+
|                                    TAURI FRONTEND                                     |
|                       React 19 + Geist Typography + Phosphor SVGs                     |
|                   Accessible UI (WCAG 2.1 AA, Focus Trapping, ARIA)                   |
+-------------------------------------------+-------------------------------------------+
                                            |
                                 IPC Commands (Async)
                                            |
+-------------------------------------------v-------------------------------------------+
|                                  RUST DESKTOP CORE                                    |
|                                                                                       |
|   +-----------------------+   +-----------------------+   +-----------------------+   |
|   |    PDF Parser Engine  |   |    Multi-Scraper      |   |   Automations & Mail  |   |
|   |  - In-memory parsing  |   |  - Tokio async tasks  |   |  - osascript escaping |   |
|   |  - Word boundaries    |   |  - 6 portal engines   |   |  - Background daemon  |   |
|   |  - Zero hallucinations|   |  - Anti-stage filter  |   |  - URL protocol check |   |
|   +-----------+-----------+   +-----------+-----------+   +-----------+-----------+   |
|               |                           |                           |               |
+---------------+---------------------------+---------------------------+---------------+
                                            |
                                    rusqlite (WAL Mode)
                                            |
+-------------------------------------------v-------------------------------------------+
|                               EMBEDDED SQLITE STORAGE                                 |
|                       PRAGMA journal_mode = WAL; (Non-blocking)                       |
|                     Jobs, Search Preferences, Custom Ignore Lists                     |
+---------------------------------------------------------------------------------------+
```

---

## Key Features

### 1. Concurrent Multi-Source Aggregation
- **6 Integrated Job Sources**: Queries Welcome to the Jungle (Algolia search), HelloWork, LinkedIn, Indeed, Jobijoba, and the official French State API (*La Bonne Alternance*) concurrently.
- **Dynamic Geolocation**: Resolves French postal codes and regions (Paris, Lyon, Marseille, Bordeaux, Nantes, Toulouse, Lille, etc.) into GPS coordinates for accurate radius searching on map-based APIs.
- **Strict Contract Enforcement**: Filters out internships (*stages*) and full-time postings (*CDI*) when targeting *alternance* contracts, matching apprenticeship parameters across all platforms (`&f_JT=A` on LinkedIn, `&jt=apprenticeship` on Indeed).

### 2. High-Precision CV Parsing & Matching
- **Zero-Hallucination Regex Engine**: Evaluates resume text through explicit word-boundary expressions (`(?i)(?:^|[^a-zA-Z0-9_+#])key(?:$|[^a-zA-Z0-9_+#])`), completely eliminating false positives (e.g. "aujourd'hui" triggering "UI", "digital" triggering "Git", "en vue de" triggering "Vue.js").
- **Decoupled Search vs. Evaluation**: Queries employment boards exclusively with target job titles (e.g. "Développeur Fullstack", "Data Analyst") while reserving specific technologies (e.g. React, Docker, PostgreSQL) for ranking and score calculation.
- **Calibrated Scoring Matrix**: Broad terms provide a calibrated baseline, while matching core technologies increment confidence up to 100%. Irrelevant non-technical trades (real estate, collections, retail) are automatically penalized to 0%.

### 3. Native Desktop Automation
- **Apple Mail Integration**: One-click generation of outgoing application email drafts with verified subject and body formatting.
- **Background Auto-Scraper Daemon**: Runs periodically on a configurable schedule, checks for newly published postings, and sends desktop notifications upon completion.
- **Centralized Ignore Rules**: Company blocklist with automatic retroactive purging from the database.

### 4. Security & Privacy Hardening (GDPR)
- **Input & Protocol Sanitization**: The browser opening mechanism enforces strict `http://` and `https://` schemas and rejects control characters to prevent command execution.
- **AppleScript Code Injection Prevention**: Validates email format strictly via RFC regex and escapes all characters to neutralize subshell execution.
- **Local Data Isolation**: Zero personal identifiable information (PII) is stored in the binary or transmitted to external tracking services. Uploaded PDF bytes are parsed in memory and immediately discarded.
- **Production Guardrails**: Chromium DevTools are disabled in release builds.

### 5. Accessible Design System
- **WCAG 2.1 AA Compliance**: Enforces a minimum 5.0:1 color contrast ratio across all buttons and states in both dark and light modes.
- **Assistive Technology Support**: Full keyboard modal focus trapping, focus restoration on dismissal, and screen reader announcements (`aria-live="polite"`).

---

## Technical Stack

| Layer | Component | Description |
| :--- | :--- | :--- |
| **Runtime & Shell** | Tauri v2 | Native desktop application shell with minimal memory footprint |
| **Backend Core** | Rust 2021 | High-performance, memory-safe backend engine |
| **Frontend Framework** | React 19 + Vite | Component architecture and client build toolchain |
| **Typography & Icons** | Geist + Phosphor SVGs | Typography and SVG icon system |
| **Concurrency** | Tokio | Multi-threaded asynchronous runtime for parallel web scraping |
| **HTTP & Parsing** | Reqwest + Scraper | HTTP client with connection pooling and CSS selector parsing |
| **Resume Extraction** | pdf-extract | In-memory PDF text extraction |
| **Local Database** | Rusqlite (SQLite) | Embedded relational database configured with Write-Ahead Logging |

---

## Getting Started

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust & Cargo](https://rustup.rs/) (edition 2021)
- macOS (for native Apple Mail draft integration; cross-platform compatible for search and scraping)

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/IgorLuna10/Alternance-Finder.git
   cd Alternance-Finder
   ```

2. **Install frontend dependencies:**
   ```bash
   npm install
   ```

3. **Run in development mode:**
   ```bash
   npm run tauri dev
   ```

4. **Compile production bundle:**
   ```bash
   npm run tauri build -- --bundles app
   ```
   The compiled macOS application bundle will be generated at:
   `src-tauri/target/release/bundle/macos/Alternance Finder.app`

---

## Configuration & Storage

Alternance Finder stores user settings, custom ignore lists, and historical listings locally on the client machine:

- **macOS Database Path**:
  `~/Library/Application Support/com.alternance.finder/alternance.db`
- **Database Engine**:
  SQLite running in `WAL` mode (`PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;`) for uninterrupted read/write concurrency.

---

## License

This project is licensed under the [MIT License](LICENSE).
