use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use serde_json::Value;
use chrono::{Datelike, Local, NaiveDate};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    pub title: String,
    pub company: String,
    pub location: String,
    pub url: String,
    pub source: String,
    pub age_label: String,
}

fn parse_age_days(label: &str) -> Option<i32> {
    let label = label.to_lowercase();
    let label = label.trim();

    // 1. Relative cases (French & English)
    if label.contains("aujourd") || label.contains("instant") || label.contains("today") || label.contains("just now") || label.contains("now") {
        return Some(0);
    }
    if label.contains("hier") || label.contains("yesterday") {
        return Some(1);
    }

    // 2. Standard ISO Date YYYY-MM-DD
    let re_ymd = Regex::new(r"\b(\d{4})-(\d{2})-(\d{2})\b").ok()?;
    if let Some(caps) = re_ymd.captures(label) {
        let year: i32 = caps.get(1)?.as_str().parse().ok()?;
        let month: u32 = caps.get(2)?.as_str().parse().ok()?;
        let day: u32 = caps.get(3)?.as_str().parse().ok()?;
        let now = Local::now().date_naive();
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            let duration = now.signed_duration_since(date);
            return Some(duration.num_days() as i32);
        }
    }

    // 3. "il y a X jour(s)/semaine(s)..." (French relative)
    let re_fr = Regex::new(r"il y a\s+(\d+)\s*(heure|h\b|jour|semaine|mois|an)").ok()?;
    if let Some(caps) = re_fr.captures(label) {
        let n: i32 = caps.get(1)?.as_str().parse().ok()?;
        let unit = caps.get(2)?.as_str();
        if unit.starts_with("heure") || unit == "h" {
            return Some(0);
        } else if unit.starts_with("jour") {
            return Some(n);
        } else if unit.starts_with("semaine") {
            return Some(n * 7);
        } else if unit.starts_with("mois") {
            return Some(n * 30);
        } else if unit.starts_with("an") {
            return Some(n * 365);
        }
    }

    // 4. "X days/weeks/months/years ago" (English relative)
    let re_en = Regex::new(r"(\d+)\s*(day|week|month|year)s?\s+ago").ok()?;
    if let Some(caps) = re_en.captures(label) {
        let n: i32 = caps.get(1)?.as_str().parse().ok()?;
        let unit = caps.get(2)?.as_str();
        if unit.starts_with("day") {
            return Some(n);
        } else if unit.starts_with("week") {
            return Some(n * 7);
        } else if unit.starts_with("month") {
            return Some(n * 30);
        } else if unit.starts_with("year") {
            return Some(n * 365);
        }
    }

    // 5. Slash dates like "DD/MM" or "DD/MM/YYYY" or "DD/MM/YY"
    let re_slash = Regex::new(r"\b(\d{1,2})/(\d{1,2})(?:/(\d{2,4}))?\b").ok()?;
    if let Some(caps) = re_slash.captures(label) {
        let day: u32 = caps.get(1)?.as_str().parse().ok()?;
        let month: u32 = caps.get(2)?.as_str().parse().ok()?;
        
        let now = Local::now().date_naive();
        let year = if let Some(year_cap) = caps.get(3) {
            let y_str = year_cap.as_str();
            if y_str.len() == 2 {
                2000 + y_str.parse::<i32>().ok()?
            } else {
                y_str.parse::<i32>().ok()?
            }
        } else {
            now.year()
        };

        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            let duration = now.signed_duration_since(date);
            return Some(duration.num_days() as i32);
        }
    }

    // 6. Textual French dates like "11 juin", "20 juin", "1er mai"
    let months = [
        ("janvier", 1), ("février", 2), ("mars", 3), ("avril", 4),
        ("mai", 5), ("juin", 6), ("juillet", 7), ("août", 8),
        ("septembre", 9), ("octobre", 10), ("novembre", 11), ("décembre", 12)
    ];

    for (m_name, m_val) in months.iter() {
        if label.contains(m_name) {
            let re_text = Regex::new(&format!(r"(\d+)\s*(?:er)?\s*{}", m_name)).ok()?;
            if let Some(caps) = re_text.captures(label) {
                let day: u32 = caps.get(1)?.as_str().parse().ok()?;
                let now = Local::now().date_naive();
                if let Some(date) = NaiveDate::from_ymd_opt(now.year(), *m_val, day) {
                    let duration = now.signed_duration_since(date);
                    let mut diff = duration.num_days() as i32;
                    // If diff is negative, it might be from the previous year
                    if diff < 0 {
                        if let Some(prev_date) = NaiveDate::from_ymd_opt(now.year() - 1, *m_val, day) {
                            diff = now.signed_duration_since(prev_date).num_days() as i32;
                        }
                    }
                    return Some(diff);
                }
            }
        }
    }

    None
}

fn format_absolute_date(age_days: Option<i32>, fallback_label: &str) -> String {
    if let Some(days) = age_days {
        let now = Local::now().date_naive();
        let announced_date = now - chrono::Duration::days(days as i64);
        announced_date.format("%d/%m/%Y").to_string()
    } else {
        let clean_label = fallback_label.trim();
        let re_date = Regex::new(r"\b\d{1,2}/\d{1,2}/\d{2,4}\b").unwrap();
        if re_date.is_match(clean_label) {
            clean_label.to_string()
        } else if !clean_label.is_empty() && clean_label != "recent" {
            if let Some(days) = parse_age_days(clean_label) {
                let now = Local::now().date_naive();
                let announced_date = now - chrono::Duration::days(days as i64);
                announced_date.format("%d/%m/%Y").to_string()
            } else {
                clean_label.to_string()
            }
        } else {
            Local::now().date_naive().format("%d/%m/%Y").to_string()
        }
    }
}

fn find_text_by_pattern(element: &scraper::ElementRef, substring: &str) -> Option<String> {
    for text in element.text() {
        let t = text.trim();
        if t.to_lowercase().contains(substring) {
            return Some(t.to_string());
        }
    }
    None
}

fn find_text_by_pattern_multiple(element: &scraper::ElementRef, substrings: &[&str]) -> Option<String> {
    for text in element.text() {
        let t = text.trim();
        let lower_t = t.to_lowercase();
        for sub in substrings {
            if lower_t.contains(sub) {
                return Some(t.to_string());
            }
        }
    }
    None
}

pub async fn scrape_wttj(keywords: &[String], location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let client = reqwest::Client::new();
    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    for kw in keywords {
        let mut payload = serde_json::json!({
            "query": kw,
            "hitsPerPage": 20,
            "aroundLatLng": "48.8566,2.3522",
            "aroundRadius": 30000
        });
        
        if contract_type == "alternance" {
            payload.as_object_mut().unwrap().insert("filters".to_string(), serde_json::Value::String("contract_type:apprenticeship".to_string()));
        } else if contract_type == "fulltime" {
            payload.as_object_mut().unwrap().insert("filters".to_string(), serde_json::Value::String("contract_type:full_time OR contract_type:temporary".to_string()));
        }

        let resp = client
            .post("https://csekhvms53-dsn.algolia.net/1/indexes/wttj_jobs_production_fr/query")
            .header("X-Algolia-Application-Id", "CSEKHVMS53")
            .header("X-Algolia-API-Key", "4bd8f6215d0cc52b26430765769e65a0")
            .header("Content-Type", "application/json")
            .header("Referer", "https://www.welcometothejungle.com/")
            .json(&payload)
            .timeout(Duration::from_secs(20))
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("WTTJ request failed for '{}': {}", kw, e);
                continue;
            }
        };

        let body: Value = match resp.json().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("WTTJ JSON parsing failed: {}", e);
                continue;
            }
        };

        let hits = match body.get("hits").and_then(|h| h.as_array()) {
            Some(h) => h,
            None => continue,
        };

        for hit in hits {
            let mut published_time = hit.get("published_at_timestamp")
                .and_then(|v| v.as_i64())
                .or_else(|| hit.get("created_at_timestamp").and_then(|v| v.as_i64()));

            if published_time.is_none() {
                let date_str = hit.get("published_at")
                    .and_then(|v| v.as_str())
                    .or_else(|| hit.get("created_at").and_then(|v| v.as_str()));
                if let Some(ds) = date_str {
                    if let Some(days) = parse_age_days(ds) {
                        published_time = Some(now_sec - (days as i64 * 86400));
                    }
                }
            }

            let age_days = if let Some(pub_time) = published_time {
                let diff_sec = now_sec - pub_time;
                Some(diff_sec / 86400)
            } else {
                None
            };

            if let Some(age) = age_days {
                if age > max_age as i64 {
                    continue;
                }
            }

            let name = hit.get("name").and_then(|v| v.as_str()).unwrap_or(kw);
            let org = hit.get("organization");
            let comp_name = org.and_then(|o| o.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            let slug_org = org.and_then(|o| o.get("slug")).and_then(|v| v.as_str()).unwrap_or("");
            let slug_job = hit.get("slug").and_then(|v| v.as_str()).unwrap_or("");

            let job_city = hit.get("offices")
                .and_then(|o| o.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("city"))
                .and_then(|v| v.as_str())
                .unwrap_or(location);

            let job_url = format!(
                "https://www.welcometothejungle.com/fr/companies/{}/jobs/{}",
                slug_org, slug_job
            );

            let age_label = format_absolute_date(age_days.map(|a| a as i32), "recent");

            jobs.push(Job {
                title: name.to_string(),
                company: comp_name.to_string(),
                location: job_city.to_string(),
                url: job_url,
                source: "Welcome to the Jungle".to_string(),
                age_label,
            });
        }
    }
    jobs
}

pub async fn scrape_hellowork(keywords: &[String], location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    for kw in keywords {
        let contract_param = if contract_type == "alternance" {
            "&c=Alternance"
        } else if contract_type == "fulltime" {
            "&c=CDI&c=CDD"
        } else {
            ""
        };
        let url = format!(
            "https://www.hellowork.com/fr-fr/emploi/recherche.html?k={}&l={}&d={}j{}",
            urlencoding::encode(kw),
            urlencoding::encode(location),
            max_age,
            contract_param
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Hellowork request failed: {}", e);
                continue;
            }
        };

        let html_content = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Hellowork body text extraction failed: {}", e);
                continue;
            }
        };

        let document = Html::parse_document(&html_content);
        let card_selector = Selector::parse("[data-id-storage-target='item'], li[id^='offer-']").unwrap();
        let link_selector = Selector::parse("a[href*='/emplois/']").unwrap();
        let title_selector = Selector::parse("h3, p[class*='tw-typo-l']").unwrap();
        let company_selector = Selector::parse("[class*='tw-typo-s']").unwrap();

        for card in document.select(&card_selector) {
            let link_el = card.select(&link_selector).next();
            if link_el.is_none() {
                continue;
            }
            let link_el = link_el.unwrap();
            let mut href = link_el.value().attr("href").unwrap_or("").to_string();
            if href.starts_with('/') {
                href = format!("https://www.hellowork.com{}", href);
            }

            let title_el = card.select(&title_selector).next();
            let title = match title_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => kw.to_string(),
            };

            let company_el = card.select(&company_selector).next();
            let company = match company_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => String::new(),
            };

            let date_label = find_text_by_pattern(&card, "il y a");
            let age_days = date_label.as_ref().and_then(|lbl| parse_age_days(lbl));

            if let Some(age) = age_days {
                if age > max_age {
                    continue;
                }
            }

            let age_label = format_absolute_date(age_days, date_label.as_deref().unwrap_or("recent"));

            jobs.push(Job {
                title,
                company,
                location: location.to_string(),
                url: href,
                source: "Hellowork".to_string(),
                age_label,
            });
        }
    }
    jobs
}

pub async fn scrape_jobijoba(keywords: &[String], location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    for kw in keywords {
        let path = if contract_type == "alternance" {
            "alternance"
        } else {
            "emploi"
        };
        let slug = kw.replace(' ', "+");
        let url = format!(
            "https://www.jobijoba.com/fr/{}/{}/{}",
            path,
            urlencoding::encode(&slug),
            urlencoding::encode(location)
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => {
                if r.status() == reqwest::StatusCode::NOT_FOUND {
                    continue;
                }
                r
            }
            Err(e) => {
                eprintln!("Jobijoba request failed: {}", e);
                continue;
            }
        };

        let html_content = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Jobijoba body text extraction failed: {}", e);
                continue;
            }
        };

        let document = Html::parse_document(&html_content);
        let card_selector = Selector::parse("article, div[class*='offer'], li[class*='result']").unwrap();
        let link_selector = Selector::parse("a.permalink-link, a[href*='/annonce/'], a[href]").unwrap();
        let title_selector = Selector::parse(".offer-header-title, h3, [class*='title']").unwrap();

        for card in document.select(&card_selector) {
            let link_el = card.select(&link_selector).next();
            if link_el.is_none() {
                continue;
            }
            let link_el = link_el.unwrap();
            let mut href = link_el.value().attr("href").unwrap_or("").to_string();
            if href.starts_with('/') {
                href = format!("https://www.jobijoba.com{}", href);
            }
            if !href.contains("jobijoba.com") && !href.starts_with("http") {
                continue;
            }

            let title_el = card.select(&title_selector).next();
            let title = match title_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => kw.to_string(),
            };
            let title = if title.is_empty() { kw.to_string() } else { title };
            let title = if title.len() > 120 { title[..120].to_string() } else { title };

            let mut company = String::new();
            let company_icon_selector = Selector::parse(".icon-apartment").unwrap();
            let feature_selector = Selector::parse(".feature").unwrap();
            for feature in card.select(&feature_selector) {
                if feature.select(&company_icon_selector).next().is_some() {
                    let text = feature.text().collect::<Vec<_>>().join(" ");
                    company = text.trim().to_string();
                    break;
                }
            }

            let date_label = find_text_by_pattern_multiple(
                &card,
                &["il y a", "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"]
            );
            let age_days = date_label.as_ref().and_then(|lbl| parse_age_days(lbl));

            if let Some(age) = age_days {
                if age > max_age {
                    continue;
                }
            }

            let age_label = format_absolute_date(age_days, date_label.as_deref().unwrap_or("recent"));

            jobs.push(Job {
                title,
                company,
                location: location.to_string(),
                url: href,
                source: "Jobijoba".to_string(),
                age_label,
            });
        }
    }
    jobs
}

pub async fn scrape_linkedin(keywords: &[String], location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    for kw in keywords {
        let contract_param = if contract_type == "alternance" {
            "&f_JT=I"
        } else if contract_type == "fulltime" {
            "&f_JT=F"
        } else {
            ""
        };
        let url = format!(
            "https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search?keywords={}&location={}{}&start=0",
            urlencoding::encode(kw),
            urlencoding::encode(location),
            contract_param
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("LinkedIn request failed: {}", e);
                continue;
            }
        };

        let html_content = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("LinkedIn body text extraction failed: {}", e);
                continue;
            }
        };

        let document = Html::parse_document(&html_content);
        let card_selector = Selector::parse("li").unwrap();
        let link_selector = Selector::parse("a.base-card__full-link, a[href*='/jobs/view/']").unwrap();
        let title_selector = Selector::parse("h3.base-search-card__title, h3.base-card__title").unwrap();
        let company_selector = Selector::parse("h4.base-search-card__subtitle, a.base-card__subtitle-link").unwrap();
        let location_selector = Selector::parse("span.job-search-card__location, span.base-card__metadata-item").unwrap();
        let date_selector = Selector::parse("time").unwrap();

        for card in document.select(&card_selector) {
            let link_el = card.select(&link_selector).next();
            if link_el.is_none() {
                continue;
            }
            let link_el = link_el.unwrap();
            let mut href = link_el.value().attr("href").unwrap_or("").to_string();
            if let Some(pos) = href.find('?') {
                href = href[..pos].to_string();
            }

            let title_el = card.select(&title_selector).next();
            let title = match title_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => kw.to_string(),
            };

            let company_el = card.select(&company_selector).next();
            let company = match company_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => String::new(),
            };

            let loc_el = card.select(&location_selector).next();
            let loc = match loc_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => location.to_string(),
            };

            let date_el = card.select(&date_selector).next();
            let date_label = if let Some(el) = date_el {
                if let Some(dt) = el.value().attr("datetime") {
                    dt.to_string()
                } else {
                    el.text().collect::<Vec<_>>().join(" ").trim().to_string()
                }
            } else {
                "recent".to_string()
            };

            let age_days = parse_age_days(&date_label);
            if let Some(age) = age_days {
                if age > max_age {
                    continue;
                }
            }

            jobs.push(Job {
                title,
                company,
                location: loc,
                url: href,
                source: "LinkedIn".to_string(),
                age_label: format_absolute_date(age_days, &date_label),
            });
        }
    }
    jobs
}

pub async fn scrape_indeed(keywords: &[String], location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let mut jobs = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    for kw in keywords {
        let contract_param = if contract_type == "alternance" {
            "&jt=internship"
        } else if contract_type == "fulltime" {
            "&jt=permanent"
        } else {
            ""
        };
        let url = format!(
            "https://fr.indeed.com/jobs?q={}&l={}{}",
            urlencoding::encode(kw),
            urlencoding::encode(location),
            contract_param
        );

        let resp = client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .header("Referer", "https://fr.indeed.com/")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Indeed request failed: {}", e);
                continue;
            }
        };

        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            eprintln!("Indeed returned 403 Forbidden (Cloudflare block). Skipping Indeed.");
            continue;
        }

        let html_content = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Indeed body text extraction failed: {}", e);
                continue;
            }
        };

        let document = Html::parse_document(&html_content);
        let card_selector = Selector::parse("div.job_seen_beacon, td.resultContent").unwrap();
        let link_selector = Selector::parse("h2.jobTitle a, a[data-jk]").unwrap();
        let company_selector = Selector::parse("span[data-testid='company-name']").unwrap();
        let location_selector = Selector::parse("div[data-testid='text-location']").unwrap();

        for card in document.select(&card_selector) {
            let link_el = card.select(&link_selector).next();
            if link_el.is_none() {
                continue;
            }
            let link_el = link_el.unwrap();
            let jk = link_el.value().attr("data-jk").unwrap_or("");
            let href = if !jk.is_empty() {
                format!("https://fr.indeed.com/viewjob?jk={}", jk)
            } else {
                let mut path = link_el.value().attr("href").unwrap_or("").to_string();
                if path.starts_with('/') {
                    path = format!("https://fr.indeed.com{}", path);
                }
                path
            };

            let title = link_el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            let title = if title.is_empty() { kw.to_string() } else { title };

            let company_el = card.select(&company_selector).next();
            let company = match company_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => String::new(),
            };

            let loc_el = card.select(&location_selector).next();
            let loc = match loc_el {
                Some(el) => el.text().collect::<Vec<_>>().join(" ").trim().to_string(),
                None => location.to_string(),
            };

            let date_label = find_text_by_pattern(&card, "il y a").or_else(|| find_text_by_pattern(&card, "posted"));
            let age_days = date_label.as_ref().and_then(|lbl| parse_age_days(lbl));

            if let Some(age) = age_days {
                if age > max_age {
                    continue;
                }
            }

            let age_label = format_absolute_date(age_days, date_label.as_deref().unwrap_or("recent"));

            jobs.push(Job {
                title,
                company,
                location: loc,
                url: href,
                source: "Indeed".to_string(),
                age_label,
            });
        }
    }
    jobs
}

pub async fn scrape_all(keywords_str: &str, location: &str, max_age: i32, contract_type: &str) -> Vec<Job> {
    let keywords: Vec<String> = keywords_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if keywords.is_empty() {
        return Vec::new();
    }

    let kws = Arc::new(keywords);
    let loc = location.to_string();
    let ct = contract_type.to_string();

    let kws1 = Arc::clone(&kws);
    let loc1 = loc.clone();
    let ct1 = ct.clone();
    let handle_wttj = tokio::spawn(async move {
        scrape_wttj(&kws1, &loc1, max_age, &ct1).await
    });

    let kws2 = Arc::clone(&kws);
    let loc2 = loc.clone();
    let ct2 = ct.clone();
    let handle_hw = tokio::spawn(async move {
        scrape_hellowork(&kws2, &loc2, max_age, &ct2).await
    });

    let kws3 = Arc::clone(&kws);
    let loc3 = loc.clone();
    let ct3 = ct.clone();
    let handle_jj = tokio::spawn(async move {
        scrape_jobijoba(&kws3, &loc3, max_age, &ct3).await
    });

    let kws4 = Arc::clone(&kws);
    let loc4 = loc.clone();
    let ct4 = ct.clone();
    let handle_li = tokio::spawn(async move {
        scrape_linkedin(&kws4, &loc4, max_age, &ct4).await
    });

    let kws5 = Arc::clone(&kws);
    let loc5 = loc.clone();
    let ct5 = ct.clone();
    let handle_ind = tokio::spawn(async move {
        scrape_indeed(&kws5, &loc5, max_age, &ct5).await
    });

    let mut all_jobs = Vec::new();

    if let Ok(jobs) = handle_wttj.await {
        all_jobs.extend(jobs);
    }
    if let Ok(jobs) = handle_hw.await {
        all_jobs.extend(jobs);
    }
    if let Ok(jobs) = handle_jj.await {
        all_jobs.extend(jobs);
    }
    if let Ok(jobs) = handle_li.await {
        all_jobs.extend(jobs);
    }
    if let Ok(jobs) = handle_ind.await {
        all_jobs.extend(jobs);
    }

    all_jobs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_age_days() {
        assert_eq!(parse_age_days("Aujourd'hui"), Some(0));
        assert_eq!(parse_age_days("Instantané"), Some(0));
        assert_eq!(parse_age_days("Hier"), Some(1));
        assert_eq!(parse_age_days("il y a 3 heures"), Some(0));
        assert_eq!(parse_age_days("il y a 2 jours"), Some(2));
        assert_eq!(parse_age_days("il y a 1 semaine"), Some(7));
        assert_eq!(parse_age_days("il y a 2 semaines"), Some(14));
        assert_eq!(parse_age_days("il y a 1 mois"), Some(30));

        // ISO format YYYY-MM-DD
        let now = Local::now().date_naive();
        let target_date = now - chrono::Duration::days(10);
        let iso_str = target_date.format("%Y-%m-%d").to_string();
        assert_eq!(parse_age_days(&iso_str), Some(10));

        // Slash format DD/MM
        let slash_str = format!("{:02}/{:02}", target_date.day(), target_date.month());
        assert_eq!(parse_age_days(&slash_str), Some(10));

        // Textual French month names
        let months = [
            "janvier", "février", "mars", "avril", "mai", "juin",
            "juillet", "août", "septembre", "octobre", "novembre", "décembre"
        ];
        let m_name = months[(target_date.month() - 1) as usize];
        let textual_str = format!("{} {}", target_date.day(), m_name);
        assert_eq!(parse_age_days(&textual_str), Some(10));

        // English relative formats
        assert_eq!(parse_age_days("today"), Some(0));
        assert_eq!(parse_age_days("yesterday"), Some(1));
        assert_eq!(parse_age_days("3 days ago"), Some(3));
        assert_eq!(parse_age_days("2 weeks ago"), Some(14));
        assert_eq!(parse_age_days("1 month ago"), Some(30));
    }

    #[tokio::test]
    async fn test_scrapers() {
        let keywords = vec!["Python".to_string()];
        let location = "Paris";
        let max_age = 30;

        println!("--- Testing scrape_wttj ---");
        let wttj_jobs = scrape_wttj(&keywords, location, max_age, "alternance").await;
        println!("WTTJ found {} jobs", wttj_jobs.len());
        for j in wttj_jobs.iter().take(3) {
            println!("  - {} at {} (url: {})", j.title, j.company, j.url);
        }

        println!("--- Testing scrape_hellowork ---");
        let hw_jobs = scrape_hellowork(&keywords, location, max_age, "alternance").await;
        println!("Hellowork found {} jobs", hw_jobs.len());
        for j in hw_jobs.iter().take(3) {
            println!("  - {} at {} (url: {})", j.title, j.company, j.url);
        }

        println!("--- Testing scrape_jobijoba ---");
        let jj_jobs = scrape_jobijoba(&keywords, location, max_age, "alternance").await;
        println!("Jobijoba found {} jobs", jj_jobs.len());
        for j in jj_jobs.iter().take(3) {
            println!("  - {} at {} (url: {})", j.title, j.company, j.url);
        }

        println!("--- Testing scrape_linkedin ---");
        let li_jobs = scrape_linkedin(&keywords, location, max_age, "alternance").await;
        println!("LinkedIn found {} jobs", li_jobs.len());
        for j in li_jobs.iter().take(3) {
            println!("  - {} at {} (url: {})", j.title, j.company, j.url);
        }

        println!("--- Testing scrape_indeed ---");
        let ind_jobs = scrape_indeed(&keywords, location, max_age, "alternance").await;
        println!("Indeed found {} jobs", ind_jobs.len());
        for j in ind_jobs.iter().take(3) {
            println!("  - {} at {} (url: {})", j.title, j.company, j.url);
        }
    }
}
