# Alternance Finder 🎯 `v2.0.0`

> **TL;DR (FR)** : Application desktop native développée avec **Rust (Tauri v2)** et **React 19** pour automatiser la recherche d'alternance. L'application scrappe en parallèle les principales plateformes d'emploi françaises (*Welcome to the Jungle, HelloWork, LinkedIn, Indeed, Jobijoba, La Bonne Alternance*), analyse le texte des CV en PDF via des expressions régulières à frontières strictes (zéro hallucination), calcule un score d'adéquation compétences/offres calibré, automatise la création de brouillons d'e-mails et persiste l'historique dans SQLite en mode WAL.

A high-performance desktop application that automates the job search workflow for work-study contracts (*alternance*). It aggregates job postings concurrently across 6 major job boards and APIs, extracts technical skills directly from PDF resumes using strict word-boundary matching, calculates calibrated compatibility scores, drafts email summaries, and tracks application pipelines offline in a local SQLite database with Write-Ahead Logging.

---

## ⚡ Key Features (v2.0.0)

- **Concurrent Multi-Platform Aggregation**: Fetches and deduplicates job postings in parallel across **Welcome to the Jungle**, **HelloWork**, **LinkedIn**, **Indeed**, **Jobijoba**, and the official French API **La Bonne Alternance** using asynchronous Rust (`tokio` + `reqwest`).
- **Precision PDF Resume Parsing**: Automatically extracts technical skills and roles from uploaded PDF resumes with strict word-boundary regex patterns, eliminating false positive substring matching.
- **Decoupled Search Targeting & Calibrated Scoring**: Decouples target job titles (for portal queries) from evaluation skills (for scoring). Broad generic terms only award base confidence, while specific technology matches dynamically scale the match percentage up to 100%. Irrelevant non-tech trades are automatically filtered out.
- **Automated Email Drafting**: One-click Apple Mail draft creation (`osascript`) with strict input sanitization and recipient validation.
- **Background Auto-Scraper Daemon**: Configurable background polling scheduler with desktop event notifications.
- **Local SQLite Storage with WAL**: Persists preferences, search parameters, ignore lists, and historical jobs with Write-Ahead Logging (`WAL` mode) for non-blocking concurrent reads and writes.
- **Accessible, Modern macOS UI**: Crafted according to the `ai-design-skills` system featuring **Geist** typography, **Phosphor** iconography, WCAG 2.1 AA compliant contrast ratios (5.0:1 in light mode), keyboard focus trapping, and `aria-live` screen reader announcements.

---

## 🛠 Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Desktop Core** | Rust 2021, Tauri v2 |
| **Frontend UI** | React 19, Vite, Modern CSS (Geist, Phosphor SVGs) |
| **Concurrency & Async** | Tokio multi-threaded runtime |
| **Web Scraping & APIs**| `reqwest`, `scraper`, `regex`, `urlencoding`, Algolia & LBA APIs |
| **Data Extraction** | `pdf-extract` (in-memory buffer parsing) |
| **Database** | Embedded SQLite with Write-Ahead Logging (`rusqlite`) |

---

## 🚀 Quick Start

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust toolchain & Cargo](https://rustup.rs/)

### Installation & Run

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

4. **Build release desktop binary:**
   ```bash
   npm run tauri build
   ```

---

## 💡 What I Learned

- **Tauri v2 Inter-Process Communication (IPC)**: Designed command handlers in Rust invoked asynchronously from React, keeping the UI non-blocking during heavy network scraping.
- **Robust Web Scraping in Rust**: Handled divergent HTML page structures, pagination, rate-limiting, and error recovery across five distinct job platforms.
- **Embedded Persistence**: Structured a clean SQLite relational schema for application pipeline tracking without requiring external database services.
