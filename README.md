# Alternance Finder 🔍💼

Alternance Finder is a modern, cross-platform desktop application designed to automate and streamline your job search. Built with Rust, Tauri, and React, it crawls leading job boards, processes PDF CVs to score listing compatibility, and handles local application tracking.

---

## 🌟 Key Features

- **Multi-Platform Scraper**: Crawls Welcome to the Jungle, HelloWork, Jobijoba, LinkedIn, and Indeed using high-performance Rust web scrapers.
- **ATS CV Parser**: Extracts skills from your PDF resume using Rust's `pdf-extract` library.
- **Smart Skill Matching**: Compares CV skills with job descriptions to compute compatibility scores and rank opportunities.
- **Local SQLite Database**: Stores search configurations, historical job offers, and application states on your machine.
- **Background Automation**: Periodically monitors job listings while the application runs and alerts you when new offers match your criteria.
- **macOS Mail Integration**: Drafts formatted email summaries of target job opportunities in the macOS Mail client via AppleScript.

---

## 🛠️ Tech Stack

- **Desktop Shell**: [Tauri v2](https://tauri.app/) (Rust)
- **Frontend**: [React 19](https://react.dev/), [Vite](https://vite.dev/), custom styling (CSS)
- **Database**: [SQLite](https://www.sqlite.org/) (managed via `rusqlite`)
- **Scraping Engine**: Rust (`reqwest` & `scraper`)
- **PDF Extraction**: Rust (`pdf-extract`)
- **OS Automation**: AppleScript / `osascript` (macOS native)

---

## 🚀 Getting Started

### Prerequisites

To run or build this application, you must install:

1. [Node.js](https://nodejs.org/) (v18 or higher recommended)
2. [Rust / Cargo toolchain](https://www.rust-lang.org/tools/install)

### Installation & Development

Clone the repository and install the dependencies:

```bash
# Clone the repository
git clone https://github.com/igorluna/alternance-finder.git
cd alternance-finder

# Install frontend dependencies
npm install

# Run the application in development mode
npm run tauri dev
```

### Production Build

Create a standalone executable for your operating system:

```bash
# Build the production executable
npm run tauri build
```

---

## 📁 Project Structure

```text
├── src/                  # React Frontend
│   ├── assets/           # Frontend assets
│   ├── main.jsx          # UI layout and Tauri command invocations
│   └── styles.css        # Custom CSS styling
├── src-tauri/            # Tauri Rust Backend
│   ├── src/
│   │   ├── main.rs       # Application entry point
│   │   ├── lib.rs        # Tauri setup, command handlers, and background loop
│   │   └── scrapers.rs   # Core scraper logic and platform crawlers
│   ├── Cargo.toml        # Rust dependencies (rusqlite, pdf-extract, reqwest, etc.)
│   └── tauri.conf.json   # Tauri application configuration
└── package.json          # Node.js workspace dependencies and scripts
```

---

## 📝 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
