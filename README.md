# Alternance Finder

Alternance Finder is a desktop application for job search automation. It is built with Rust, Tauri, and React. It scrapes job boards, extracts text from PDF CVs to calculate skill compatibility scores, and tracks applications locally.

## Features

- **Web Scraper**: Scrapes Welcome to the Jungle, HelloWork, Jobijoba, LinkedIn, and Indeed using scrapers written in Rust.
- **CV Parser**: Extracts text from PDF resumes using the `pdf-extract` library.
- **Skill Matcher**: Compares CV text with job descriptions to compute compatibility scores.
- **Database**: Stores configurations, job offers, and application states in a local SQLite database.
- **Background Loop**: Monitors job listings and triggers alerts when new matches occur.
- **Email Drafts**: Creates email templates in macOS Mail using AppleScript.

## Tech Stack

- **Framework**: Tauri v2
- **Frontend**: React 19, Vite, CSS
- **Database**: SQLite (via `rusqlite`)
- **HTTP Client**: `reqwest`
- **HTML Parser**: `scraper`
- **PDF Reader**: `pdf-extract`
- **Scripting**: AppleScript

## Installation & Development

### Prerequisites

- Node.js
- Rust

### Steps

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```
