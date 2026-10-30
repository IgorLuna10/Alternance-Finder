use rusqlite::{Connection, Result as SqlResult};
use std::fs;
use std::sync::Mutex;
use tauri::Manager;
use tauri::Emitter;

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

    // Insert default settings if they don't exist
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_scrape', 'false')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('scrape_interval_hours', '4')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('recipient_email', 'igor.luna.it@gmail.com')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('keywords', 'développeur fullstack, développeur web, react, node.js')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('location', 'Paris')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('max_age', '2')", ())?;
    conn.execute("INSERT OR IGNORE INTO settings (key, value) VALUES ('contract_type', 'alternance')", ())?;

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

    let conn = state.db.lock().unwrap();
    let mut new_jobs = Vec::new();

    for job in scraped_jobs {
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
    println!("Tauri backend: opening URL in browser: {}", url);
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", "start", &url]).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = url;
        Err("Unsupported OS".to_string())
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

// Helper to create Apple Mail draft on macOS
fn create_mail_draft(recipient: &str, subject: &str, body: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
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
            escaped_subject, escaped_body, recipient
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
            let recipient_email: String = conn.query_row("SELECT value FROM settings WHERE key = 'recipient_email'", [], |r| r.get(0)).unwrap_or_else(|_| "igor.luna.it@gmail.com".to_string());
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
            
            let (keywords, location, max_age, contract_type) = {
                let conn = state.db.lock().unwrap();
                let kw: String = conn.query_row("SELECT value FROM settings WHERE key = 'keywords'", [], |r| r.get(0)).unwrap_or_else(|_| "développeur fullstack, développeur web, react, node.js".to_string());
                let loc: String = conn.query_row("SELECT value FROM settings WHERE key = 'location'", [], |r| r.get(0)).unwrap_or_else(|_| "Paris".to_string());
                let age: i32 = conn.query_row("SELECT value FROM settings WHERE key = 'max_age'", [], |r| r.get(0))
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2);
                let ct: String = conn.query_row("SELECT value FROM settings WHERE key = 'contract_type'", [], |r| r.get(0)).unwrap_or_else(|_| "alternance".to_string());
                (kw, loc, age, ct)
            };

            let scraped_jobs = scrapers::scrape_all(&keywords, &location, max_age, &contract_type).await;
            let mut new_jobs = Vec::new();

            {
                let conn = state.db.lock().unwrap();
                for job in scraped_jobs {
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
                println!("Tauri background: Found {} new jobs! Auto-drafting email...", new_jobs.len());
                
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

                // Emit event to frontend
                let _ = app.emit("new-jobs-auto-scraped", ());
            } else {
                println!("Tauri background: Auto-scrape finished, no new jobs found.");
            }
        }
    }
}

// 7b. Command to extract skills from a CV byte buffer
#[tauri::command]
fn parse_cv_bytes(bytes: Vec<u8>) -> Result<Vec<String>, String> {
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract text from PDF: {}", e))?;
    
    let text_normalized = text.to_lowercase()
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ");
    
    let skills_mapping = vec![
        // Tech & Software
        ("react", "React"),
        ("nextjs", "Next.js"),
        ("next.js", "Next.js"),
        ("vue", "Vue.js"),
        ("angular", "Angular"),
        ("javascript", "JavaScript"),
        ("typescript", "TypeScript"),
        ("node", "Node.js"),
        ("python", "Python"),
        ("rust", "Rust"),
        ("docker", "Docker"),
        ("kubernetes", "Kubernetes"),
        ("aws", "AWS"),
        ("gcp", "Google Cloud"),
        ("azure", "Azure"),
        ("sql", "SQL"),
        ("postgres", "PostgreSQL"),
        ("mongodb", "MongoDB"),
        ("devops", "DevOps"),
        ("git", "Git"),
        ("c++", "C++"),
        ("java", "Java"),
        ("php", "PHP"),
        ("backend", "Backend Developer"),
        ("frontend", "Frontend Developer"),
        ("fullstack", "Fullstack Developer"),
        ("full stack", "Fullstack Developer"),
        ("développeur", "Développeur"),
        ("developpeur", "Développeur"),
        ("software engineer", "Software Engineer"),
        ("data", "Data Analyst"),
        ("cybersecurite", "Cybersécurité"),
        
        // Sales & Business Development
        ("commercial", "Commercial"),
        ("business developer", "Business Developer"),
        ("business development", "Business Development"),
        ("account manager", "Account Manager"),
        ("account executive", "Account Executive"),
        ("sales", "Sales"),
        ("prospection", "Prospection"),
        ("négociation", "Négociation"),
        ("negociation", "Négociation"),
        ("crm", "CRM"),
        ("hubspot", "HubSpot"),
        ("salesforce", "Salesforce"),
        
        // Marketing & Communication
        ("marketing", "Marketing"),
        ("growth", "Growth Hacking"),
        ("community manager", "Community Manager"),
        ("seo", "SEO"),
        ("sem", "SEM"),
        ("communication", "Communication"),
        
        // Design & Product
        ("ux", "UX Design"),
        ("ui", "UI Design"),
        ("figma", "Figma"),
        ("product manager", "Product Manager"),
        ("product owner", "Product Owner"),
        
        // Finance & Management
        ("comptabilité", "Comptabilité"),
        ("comptable", "Comptable"),
        ("finance", "Finance"),
        ("gestion", "Gestion de Projet"),
        ("management", "Management"),
        ("ressources humaines", "Ressources Humaines"),
        ("recrutement", "Recrutement")
    ];
    
    let mut found_skills = std::collections::BTreeSet::new();
    for (key, val) in skills_mapping {
        if text_normalized.contains(key) {
            found_skills.insert(val.to_string());
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