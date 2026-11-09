use rusqlite::{Connection, Result as SqlResult};
use std::fs;
use std::sync::Mutex;
use tauri::Manager;
use tauri::Emitter;
use regex::Regex;

mod scrapers;
use scrapers::Job;

// 1. Tauri State struct to hold the DB connection safely across threads
struct AppState {
    db: Mutex<Connection>,
}

// 2. Database Initialization
fn init_db(app: &tauri::AppHandle) -> SqlResult<Connection> {
    let app_dir = app.path().app_data_dir().expect("Failed to get app data dir");
    fs::create_dir_all(&app_dir).expect("Failed to create app data directory");
    
    let db_path = app_dir.join("alternance.db");
    let conn = Connection::open(db_path)?;

    // Enable Write-Ahead Logging (WAL) and set busy timeout for concurrency
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;"
    )?;

    // Create the jobs table.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY,
            url TEXT UNIQUE NOT NULL,
            title TEXT NOT NULL,
            company TEXT NOT NULL,
            location TEXT NOT NULL,
            source TEXT NOT NULL,
            age_label TEXT NOT NULL,
            status TEXT DEFAULT 'new',
            date_added DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        (),
    )?;

    // Create the settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        (),
    )?;

    // Insert default settings if they don't exist (no hardcoded PII)
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_scrape', 'false')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('scrape_interval_hours', '4')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('recipient_email', '')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('keywords', 'développeur fullstack, développeur web')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('location', 'Paris')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('max_age', '2')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('contract_type', 'alternance')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('ignore_list', 'Studi CFA, Studi')", ())?;

    // Automatically purge any existing jobs matching Studi CFA / Studi
    let _ = conn.execute("DELETE FROM jobs WHERE LOWER(company) LIKE '%studi%' OR LOWER(title) LIKE '%studi%'", ());

    Ok(conn)
}

// 3. Command to run the Rust scraper manually and insert new jobs
#[tauri::command]
async fn run_scraper_worker(
    state: tauri::State<'_, AppState>,
    keywords: String,
    location: String,
    max_age: i32,
    contract_type: String,
) -> Result<Vec<Job>, String> {
    let scraped_jobs = scrapers::scrape_all(&keywords, &location, max_age, &contract_type).await;

    let ignore_list: Vec<String> = {
        let conn = state.db.lock().unwrap();
        conn.query_row("SELECT value FROM settings WHERE key = 'ignore_list'", [], |r| r.get(0))
            .unwrap_or_else(|_| "Studi CFA, Studi".to_string())
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let is_ignored = |title: &str, company: &str| -> bool {
        let t = title.to_lowercase();
        let c = company.to_lowercase();
        ignore_list.iter().any(|ign| c.contains(ign) || t.contains(ign))
    };

    let conn = state.db.lock().unwrap();
    let mut new_jobs = Vec::new();

    for job in scraped_jobs {
        if is_ignored(&job.title, &job.company) {
            continue;
        }

        let result = conn.execute(
            "INSERT INTO jobs (url, title, company, location, source, age_label) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &job.url,
                &job.title,
                &job.company,
                &job.location,
                &job.source,
                &job.age_label,
            ),
        );

        if result.is_ok() {
            new_jobs.push(job);
        }
    }

    Ok(new_jobs)
}

// 4. Command to fetch all historical jobs for the frontend
#[tauri::command]
fn get_all_jobs(state: tauri::State<'_, AppState>) -> Result<Vec<Job>, String> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT title, company, location, url, source, age_label FROM jobs ORDER BY date_added DESC")
        .map_err(|e| e.to_string())?;

    let job_iter = stmt
        .query_map([], |row| {
            Ok(Job {
                title: row.get(0)?,
                company: row.get(1)?,
                location: row.get(2)?,
                url: row.get(3)?,
                source: row.get(4)?,
                age_label: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut jobs = Vec::new();
    for job in job_iter {
        if let Ok(j) = job {
            jobs.push(j);
        }
    }

    Ok(jobs)
}

// 5. Command to clear all jobs from the database
#[tauri::command]
fn clear_all_jobs(state: tauri::State<'_, AppState>) -> Result<(), String> {
    println!("Tauri backend: clear_all_jobs command called");
    let conn = state.db.lock().unwrap();
    let rows_deleted = conn.execute("DELETE FROM jobs", ())
        .map_err(|e| {
            eprintln!("Tauri backend: Error clearing jobs: {}", e);
            e.to_string()
        })?;
    println!("Tauri backend: Successfully cleared jobs table. Rows deleted: {}", rows_deleted);
    Ok(())
}

#[tauri::command]
fn open_in_browser(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err("Security Block: Only HTTP and HTTPS protocols are permitted".to_string());
    }

    // Reject control characters and quote characters to prevent argument/command injection
    if trimmed.chars().any(|c| c.is_control() || c == '"' || c == '\'' || c == '`' || c == '\n' || c == '\r') {
        return Err("Security Block: Invalid characters in URL".to_string());
    }

    println!("Tauri backend: Opening validated URL in browser: {}", trimmed);
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("--")
            .arg(trimmed)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", trimmed])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(trimmed)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Unsupported operating system".to_string())
    }
}

// 7. Commands to manage SQLite Settings
#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })
        .map_err(|e| e.to_string())?;

    let mut map = serde_json::Map::new();
    for row in rows {
        if let Ok((k, v)) = row {
            map.insert(k, serde_json::Value::String(v));
        }
    }

    Ok(serde_json::Value::Object(map))
}

#[tauri::command]
fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: serde_json::Value,
) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    
    if let serde_json::Value::Object(map) = settings {
        for (k, v) in map {
            if let Some(v_str) = v.as_str() {
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                    (&k, &v_str),
                )
                .map_err(|e| e.to_string())?;

                if k == "ignore_list" {
                    for item in v_str.split(',') {
                        let term = item.trim().to_lowercase();
                        if !term.is_empty() {
                            let pattern = format!("%{}%", term);
                            let _ = conn.execute(
                                "DELETE FROM jobs WHERE LOWER(company) LIKE ?1 OR LOWER(title) LIKE ?1",
                                [&pattern],
                            );
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
fn draft_all_offers(state: tauri::State<'_, AppState>, recipient: String) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT title, company, location, url, source, age_label FROM jobs ORDER BY date_added DESC")
        .map_err(|e| e.to_string())?;

    let job_iter = stmt
        .query_map([], |row| {
            Ok(Job {
                title: row.get(0)?,
                company: row.get(1)?,
                location: row.get(2)?,
                url: row.get(3)?,
                source: row.get(4)?,
                age_label: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut jobs = Vec::new();
    for job in job_iter {
        if let Ok(j) = job {
            jobs.push(j);
        }
    }

    if jobs.is_empty() {
        return Err("No offers found to email. Search first!".to_string());
    }

    let subject = format!("Alternance Finder: Found Offers ({} offers)", jobs.len());
    let mut body = format!("Hi,\n\nHere are the job offers found by Alternance Finder:\n\n");
    for (i, job) in jobs.iter().enumerate() {
        body.push_str(&format!(
            "{}. {} at {}\n   Location: {}\n   Source: {}\n   Date Announced: {}\n   URL: {}\n\n",
            i + 1, job.title, job.company, job.location, job.source, job.age_label, job.url
        ));
    }
    body.push_str("Good luck!");

    create_mail_draft(&recipient, &subject, &body)
}

#[tauri::command]
fn delete_job(state: tauri::State<'_, AppState>, url: String) -> Result<(), String> {
    println!("Tauri backend: delete_job command called for url: {}", url);
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM jobs WHERE url = ?1", (&url,))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_jobs_by_urls(state: tauri::State<'_, AppState>, urls: Vec<String>) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    for url in urls {
        let _ = conn.execute("DELETE FROM jobs WHERE url = ?1", (&url,));
    }
    Ok(())
}

// Helper to create Apple Mail draft on macOS with strict input sanitization
fn create_mail_draft(recipient: &str, subject: &str, body: &str) -> Result<(), String> {
    let clean_recipient = recipient.trim();
    if clean_recipient.is_empty() {
        return Err("No recipient email configured. Please enter an email address.".to_string());
    }

    // Validate email format strictly to eliminate AppleScript injection vectors
    let email_re = Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$")
        .map_err(|e| e.to_string())?;
    if !email_re.is_match(clean_recipient) {
        return Err("Invalid recipient email format.".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let escaped_recipient = clean_recipient.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_body = body.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        let escaped_subject = subject.replace('\\', "\\\\").replace('"', "\\\"");
        
        let script = format!(
            r#"tell application "Mail"
                set newM to make new outgoing message with properties {{subject:"{}", content:"{}"}}
                tell newM
                    make new to recipient at end of to recipients with properties {{address:"{}"}}
                    save
                end tell
            end tell"#,
            escaped_subject, escaped_body, escaped_recipient
        );

        let output = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AppleScript failed: {}", stderr));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = recipient;
        let _ = subject;
        let _ = body;
        Err("Auto-drafting is only supported on macOS Mail app.".to_string())
    }
}

// Background scheduler running periodically when the app is active
async fn run_background_scheduler(app: tauri::AppHandle) {
    println!("Tauri background: Background scheduler started.");
    loop {
        // Sleep for 3 minutes before checking settings and schedules
        tokio::time::sleep(tokio::time::Duration::from_secs(180)).await;

        let state = match app.try_state::<AppState>() {
            Some(s) => s,
            None => continue,
        };

        let (auto_scrape, interval_hours, recipient_email) = {
            let conn = state.db.lock().unwrap();
            let auto_scrape: String = conn.query_row("SELECT value FROM settings WHERE key = 'auto_scrape'", [], |r| r.get(0)).unwrap_or_else(|_| "false".to_string());
            let interval_hours: i64 = conn.query_row("SELECT value FROM settings WHERE key = 'scrape_interval_hours'", [], |r| r.get(0))
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4);
            let recipient_email: String = conn.query_row("SELECT value FROM settings WHERE key = 'recipient_email'", [], |r| r.get(0)).unwrap_or_default();
            (auto_scrape == "true", interval_hours, recipient_email)
        };

        if !auto_scrape {
            continue;
        }

        let last_run: i64 = {
            let conn = state.db.lock().unwrap();
            conn.query_row("SELECT value FROM settings WHERE key = 'last_auto_scrape_timestamp'", [], |r| r.get(0))
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0)
        };

        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Run if interval has elapsed since last run
        if now_sec - last_run >= interval_hours * 3600 {
            println!("Tauri background: Starting auto-scrape cycle...");
            
            let (keywords, location, max_age, contract_type, ignore_list) = {
                let conn = state.db.lock().unwrap();
                let kw: String = conn.query_row("SELECT value FROM settings WHERE key = 'keywords'", [], |r| r.get(0)).unwrap_or_else(|_| "développeur fullstack, développeur web".to_string());
                let loc: String = conn.query_row("SELECT value FROM settings WHERE key = 'location'", [], |r| r.get(0)).unwrap_or_else(|_| "Paris".to_string());
                let age: i32 = conn.query_row("SELECT value FROM settings WHERE key = 'max_age'", [], |r| r.get(0))
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2);
                let ct: String = conn.query_row("SELECT value FROM settings WHERE key = 'contract_type'", [], |r| r.get(0)).unwrap_or_else(|_| "alternance".to_string());
                let ign: Vec<String> = conn.query_row("SELECT value FROM settings WHERE key = 'ignore_list'", [], |r| r.get(0))
                    .unwrap_or_else(|_| "Studi CFA, Studi".to_string())
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                (kw, loc, age, ct, ign)
            };

            let scraped_jobs = scrapers::scrape_all(&keywords, &location, max_age, &contract_type).await;
            let mut new_jobs = Vec::new();

            {
                let conn = state.db.lock().unwrap();
                let is_ignored = |title: &str, company: &str| -> bool {
                    let t = title.to_lowercase();
                    let c = company.to_lowercase();
                    ignore_list.iter().any(|ign| c.contains(ign) || t.contains(ign))
                };

                for job in scraped_jobs {
                    if is_ignored(&job.title, &job.company) {
                        continue;
                    }

                    let result = conn.execute(
                        "INSERT INTO jobs (url, title, company, location, source, age_label) 
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        (
                            &job.url,
                            &job.title,
                            &job.company,
                            &job.location,
                            &job.source,
                            &job.age_label,
                        ),
                    );
                    if result.is_ok() {
                        new_jobs.push(job);
                    }
                }
                
                // Update last run timestamp
                let _ = conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES ('last_auto_scrape_timestamp', ?1)", (now_sec.to_string(),));
            }

            if !new_jobs.is_empty() {
                println!("Tauri background: Found {} new jobs!", new_jobs.len());
                
                if !recipient_email.trim().is_empty() {
                    let subject = format!("New Job Offers Found! ({} new)", new_jobs.len());
                    let mut body = format!("Hi,\n\nHere are the new job offers found by Alternance Finder:\n\n");
                    for (i, job) in new_jobs.iter().enumerate() {
                        body.push_str(&format!(
                            "{}. {} at {}\n   Location: {}\n   Source: {}\n   Date Announced: {}\n   URL: {}\n\n",
                            i + 1, job.title, job.company, job.location, job.source, job.age_label, job.url
                        ));
                    }
                    body.push_str("Good luck!");

                    if let Err(e) = create_mail_draft(&recipient_email, &subject, &body) {
                        eprintln!("Tauri background: Failed to create mail draft: {}", e);
                    } else {
                        println!("Tauri background: Successfully drafted email to {}", recipient_email);
                    }
                } else {
                    println!("Tauri background: No recipient email configured, skipping auto email draft.");
                }

                // Emit event to frontend
                let _ = app.emit("new-jobs-auto-scraped", ());
            } else {
                println!("Tauri background: Auto-scrape finished, no new jobs found.");
            }
        }
    }
}

// 7b. Command to extract skills from a CV byte buffer using strict word-boundary regexes
#[tauri::command]
fn parse_cv_bytes(bytes: Vec<u8>) -> Result<Vec<String>, String> {
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract text from PDF: {}", e))?;
    
    let text_clean = format!(" {} ", text.replace('\r', " ").replace('\n', " ").replace('\t', " "));
    
    // Skill patterns with strict boundaries to prevent false substring matches
    // (e.g. "aujourd'hui" matching "ui", "digital" matching "git", "en vue de" matching "vue")
    let patterns: Vec<(&str, &str)> = vec![
        // Languages & Core Runtimes
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])react(?:native)?(?:\.js|js)?(?:$|[^a-zA-Z0-9_+#])", "React"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])next(?:\.js|js)(?:$|[^a-zA-Z0-9_+#])", "Next.js"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:vue(?:\.js|js)|vue\s*[23])(?:$|[^a-zA-Z0-9_+#])", "Vue.js"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])angular(?:$|[^a-zA-Z0-9_+#])", "Angular"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])svelte(?:kit)?(?:$|[^a-zA-Z0-9_+#])", "Svelte"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:javascript|\bjs\b)(?:$|[^a-zA-Z0-9_+#])", "JavaScript"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:typescript|\bts\b)(?:$|[^a-zA-Z0-9_+#])", "TypeScript"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])node(?:\.js|js)?(?:$|[^a-zA-Z0-9_+#])", "Node.js"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])express(?:\.js|js)?(?:$|[^a-zA-Z0-9_+#])", "Express"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])nest(?:\.js|js)(?:$|[^a-zA-Z0-9_+#])", "NestJS"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])python(?:$|[^a-zA-Z0-9_+#])", "Python"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])django(?:$|[^a-zA-Z0-9_+#])", "Django"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])fastapi(?:$|[^a-zA-Z0-9_+#])", "FastAPI"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])flask(?:$|[^a-zA-Z0-9_+#])", "Flask"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])java(?:$|[^a-zA-Z0-9_+#])", "Java"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])spring(?:\s*boot)?(?:$|[^a-zA-Z0-9_+#])", "Spring Boot"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])c\+\+(?:$|[^a-zA-Z0-9_+#])", "C++"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])c#(?:$|[^a-zA-Z0-9_+#])", "C#"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])\.net(?:$|[^a-zA-Z0-9_+#])", ".NET"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])php(?:$|[^a-zA-Z0-9_+#])", "PHP"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])laravel(?:$|[^a-zA-Z0-9_+#])", "Laravel"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])symfony(?:$|[^a-zA-Z0-9_+#])", "Symfony"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])rust(?:$|[^a-zA-Z0-9_+#])", "Rust"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:golang|go\s*lang)(?:$|[^a-zA-Z0-9_+#])", "Go"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])tailwind(?:css)?(?:$|[^a-zA-Z0-9_+#])", "Tailwind CSS"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:html5?|css3?)(?:$|[^a-zA-Z0-9_+#])", "HTML/CSS"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])graphql(?:$|[^a-zA-Z0-9_+#])", "GraphQL"),

        // Infrastructure, DB & DevOps
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])docker(?:$|[^a-zA-Z0-9_+#])", "Docker"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:kubernetes|k8s)(?:$|[^a-zA-Z0-9_+#])", "Kubernetes"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])aws(?:$|[^a-zA-Z0-9_+#])", "AWS"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:gcp|google\s*cloud)(?:$|[^a-zA-Z0-9_+#])", "Google Cloud"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])azure(?:$|[^a-zA-Z0-9_+#])", "Azure"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])sql(?:$|[^a-zA-Z0-9_+#])", "SQL"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])postgres(?:ql)?(?:$|[^a-zA-Z0-9_+#])", "PostgreSQL"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])mysql(?:$|[^a-zA-Z0-9_+#])", "MySQL"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])mongo(?:db)?(?:$|[^a-zA-Z0-9_+#])", "MongoDB"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])redis(?:$|[^a-zA-Z0-9_+#])", "Redis"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])git(?:$|[^a-zA-Z0-9_+#])", "Git"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:ci/cd|ci-cd|gitlab\s*ci|github\s*actions)(?:$|[^a-zA-Z0-9_+#])", "CI/CD"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])linux(?:$|[^a-zA-Z0-9_+#])", "Linux"),

        // Engineering Roles & Specializations
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:fullstack|full\s*stack)(?:$|[^a-zA-Z0-9_+#])", "Fullstack Developer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:frontend|front\s*end)(?:$|[^a-zA-Z0-9_+#])", "Frontend Developer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:backend|back\s*end)(?:$|[^a-zA-Z0-9_+#])", "Backend Developer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:devops|dev\s*ops)(?:$|[^a-zA-Z0-9_+#])", "DevOps"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:data\s*analyst|analyste\s*(?:de\s*)?donn[ée]es)(?:$|[^a-zA-Z0-9_+#])", "Data Analyst"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:data\s*engineer|ing[ée]nieur\s*(?:de\s*)?donn[ée]es)(?:$|[^a-zA-Z0-9_+#])", "Data Engineer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:data\s*scientist|data\s*science)(?:$|[^a-zA-Z0-9_+#])", "Data Scientist"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:machine\s*learning|\bml\b|ia\b|intelligence\s*artificielle)(?:$|[^a-zA-Z0-9_+#])", "Machine Learning"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:software\s*engineer|ing[ée]nieur\s*(?:d['’]études\s*et\s*)?d[ée]veloppement|ing[ée]nieur\s*logiciel)(?:$|[^a-zA-Z0-9_+#])", "Software Engineer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:cybers[ée]curit[ée]|cybersecurity|pentest(?:ing)?)(?:$|[^a-zA-Z0-9_+#])", "Cybersécurité"),

        // Design & Product
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:ui/ux|ui\s*design(?:er)?|user\s*interface)(?:$|[^a-zA-Z0-9_+#])", "UI Design"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:ui/ux|ux\s*design(?:er)?|user\s*experience)(?:$|[^a-zA-Z0-9_+#])", "UX Design"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])figma(?:$|[^a-zA-Z0-9_+#])", "Figma"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:product\s*owner|product\s*manager|\bpo\b|\bpm\b)(?:$|[^a-zA-Z0-9_+#])", "Product Owner"),

        // Business, Sales & Marketing
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:business\s*developer|account\s*manager|commercial\b|prospection\s*(?:b2b|b2c)?)(?:$|[^a-zA-Z0-9_+#])", "Business Developer"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:hubspot|salesforce|\bcrm\b)(?:$|[^a-zA-Z0-9_+#])", "CRM"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:marketing\s*digital|growth\s*hacking|community\s*manager)(?:$|[^a-zA-Z0-9_+#])", "Marketing"),
        (r"(?i)(?:^|[^a-zA-Z0-9_+#])(?:\bseo\b|\bsea\b|search\s*engine)(?:$|[^a-zA-Z0-9_+#])", "SEO"),
    ];
    
    let mut found_skills = std::collections::BTreeSet::new();
    for (regex_str, val) in patterns {
        if let Ok(re) = Regex::new(regex_str) {
            if re.is_match(&text_clean) {
                found_skills.insert(val.to_string());
            }
        }
    }
    
    Ok(found_skills.into_iter().collect())
}

// 8. App Initialization
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let db_conn = init_db(app.handle()).expect("Failed to initialize database");
            
            app.manage(AppState {
                db: Mutex::new(db_conn),
            });

            // Start background auto-scrape scheduler
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                run_background_scheduler(app_handle).await;
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            run_scraper_worker, 
            get_all_jobs, 
            clear_all_jobs, 
            open_in_browser,
            get_settings,
            save_settings,
            draft_all_offers,
            parse_cv_bytes,
            delete_job,
            delete_jobs_by_urls
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}