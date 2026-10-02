# Alternance Finder 🎯

> **TL;DR (FR)** : Application desktop native développée avec **Rust (Tauri v2)** et **React 19** pour automatiser la recherche d'alternance. L'application scrappe les plateformes d'emploi françaises (*Welcome to the Jungle, HelloWork, LinkedIn, Indeed, Jobijoba*), analyse le texte des CV en PDF, calcule un score d'adéquation compétences/offres et permet de suivre ses candidatures via une base SQLite locale.

A high-performance desktop application that automates the job search workflow for work-study contracts (*alternance*). It aggregates job postings across major job boards, extracts text directly from PDF resumes, calculates skill compatibility scores, and tracks application statuses entirely offline in a local database.

---

## 📸 Overview

```text
┌────────────────────────────────────────────────────────┐
│ Alternance Finder Desktop                              │
├─────────────────┬──────────────────────────────────────┤
│ [📄 Upload CV]  │ 🔍 Live Aggregated Offers (5 boards) │
│ Match: 87%      │ ──────────────────────────────────── │
│ Skills: Rust,   │ • Full-Stack Developer - Paris       │
│ React, SQL      │   Match Score: [████████░░] 87%      │
│                 │   Status: [ Applied 📌 ]             │
└─────────────────┴──────────────────────────────────────┘
```
*(Add demo screenshot or GIF here: `docs/demo.png`)*

---

## ⚡ Key Features

- **Multi-Platform Web Scrapers**: Concurrently fetches job postings from Welcome to the Jungle, HelloWork, LinkedIn, Indeed, and Jobijoba using custom scrapers written in Rust (`reqwest` + `scraper`).
- **PDF Resume Parser**: Automatically parses and extracts plain text from uploaded PDF resumes using `pdf-extract`.
- **Skill Compatibility Scoring**: Compares resume technical keywords against job descriptions to calculate an instant compatibility match percentage.
- **Local Application Tracking**: Persists user preferences, search filters, saved listings, and application states locally using embedded SQLite (`rusqlite`).
- **Native Desktop Performance**: Ultra-fast startup and tiny memory footprint powered by Tauri v2 and Vite.

---

## 🛠 Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Desktop Core** | Rust 2021, Tauri v2 |
| **Frontend UI** | React 19, Vite, Modern CSS |
| **Concurrency & Async** | Tokio |
| **Web Scraping & HTTP** | `reqwest`, `scraper`, `regex`, `urlencoding` |
| **Data Extraction** | `pdf-extract` |
| **Database** | Embedded SQLite (`rusqlite`) |

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
