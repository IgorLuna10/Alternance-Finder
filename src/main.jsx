import React, { useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

function App() {
  const [keywordInput, setKeywordInput] = useState("");
  const [keywords, setKeywords] = useState([]);
  const [location, setLocation] = useState("Paris");
  const [maxAge, setMaxAge] = useState(2);
  const [recipientEmail, setRecipientEmail] = useState("igor.luna.it@gmail.com");
  const [jobs, setJobs] = useState([]);
  const [loading, setLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState("");
  const [showConfirmClear, setShowConfirmClear] = useState(false);
  
  // Automation settings state
  const [autoScrape, setAutoScrape] = useState(false);
  const [scrapeInterval, setScrapeInterval] = useState("4");
  const [contractType, setContractType] = useState("alternance");
  const [isLoaded, setIsLoaded] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // CV match settings state
  const [cvSkills, setCvSkills] = useState(() => {
    try {
      const saved = localStorage.getItem("cvSkills");
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });
  const [cvFileName, setCvFileName] = useState(() => {
    return localStorage.getItem("cvFileName") || "";
  });
  const [cvUploadLoading, setCvUploadLoading] = useState(false);
  const [sortBy, setSortBy] = useState("date");
  const [newSkillInput, setNewSkillInput] = useState("");
  const [hideZeroMatches, setHideZeroMatches] = useState(false);

  useEffect(() => {
    localStorage.setItem("cvSkills", JSON.stringify(cvSkills));
  }, [cvSkills]);

  useEffect(() => {
    localStorage.setItem("cvFileName", cvFileName);
  }, [cvFileName]);

  // Load settings on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await invoke("get_settings");
        if (settings) {
          if (settings.auto_scrape) setAutoScrape(settings.auto_scrape === "true");
          if (settings.scrape_interval_hours) setScrapeInterval(settings.scrape_interval_hours);
          if (settings.recipient_email) setRecipientEmail(settings.recipient_email);
          if (settings.location) setLocation(settings.location);
          if (settings.max_age) setMaxAge(parseInt(settings.max_age) || 2);
          if (settings.contract_type) setContractType(settings.contract_type);
          if (settings.keywords) {
            const kwList = settings.keywords.split(",").map(k => k.trim()).filter(Boolean);
            setKeywords(kwList);
          }
        }
      } catch (error) {
        console.error("Failed to load settings from DB:", error);
      } finally {
        setIsLoaded(true);
      }
    };

    loadSettings();
    fetchSavedJobs();
  }, []);

  // Listen to background scrape notifications
  useEffect(() => {
    let unlisten;
    const setupListener = async () => {
      try {
        unlisten = await listen("new-jobs-auto-scraped", (event) => {
          console.log("New jobs auto-scraped event received");
          fetchSavedJobs();
          setStatusMessage("New job offers found in background and saved/drafted!");
        });
      } catch (err) {
        console.error("Failed to set up event listener:", err);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // Save settings when they change
  useEffect(() => {
    if (!isLoaded) return;

    const timer = setTimeout(async () => {
      setIsSaving(true);
      try {
        await invoke("save_settings", {
          settings: {
            auto_scrape: autoScrape ? "true" : "false",
            scrape_interval_hours: scrapeInterval.toString(),
            recipient_email: recipientEmail,
            keywords: keywords.join(", "),
            location: location,
            max_age: maxAge.toString(),
            contract_type: contractType
          }
        });
      } catch (error) {
        console.error("Failed to save settings to DB:", error);
      } finally {
        setIsSaving(false);
      }
    }, 600);

    return () => clearTimeout(timer);
  }, [keywords, location, maxAge, recipientEmail, autoScrape, scrapeInterval, contractType, isLoaded]);

  const fetchSavedJobs = async () => {
    try {
      const savedJobs = await invoke("get_all_jobs");
      setJobs(savedJobs);
    } catch (error) {
      console.error("Failed to fetch jobs:", error);
    }
  };

  const addKeyword = (e) => {
    if (e.key === "Enter" && keywordInput.trim()) {
      if (!keywords.includes(keywordInput.trim())) {
        setKeywords([...keywords, keywordInput.trim()]);
      }
      setKeywordInput("");
    }
  };

  const removeKeyword = (indexToRemove) => {
    setKeywords(keywords.filter((_, index) => index !== indexToRemove));
  };

  const handleRunScraper = async () => {
    if (keywords.length === 0) {
      setStatusMessage("Please add at least one keyword.");
      return;
    }
    setLoading(true);
    setStatusMessage("Searching for new opportunities...");
    try {
      // Pass keywords as a comma-separated string for compatibility with current worker
      const keywordsString = keywords.join(", ");
      const newJobs = await invoke("run_scraper_worker", { 
        keywords: keywordsString, 
        location,
        maxAge: parseInt(maxAge),
        contractType
      });
      setStatusMessage(`Scan complete! Found ${newJobs.length} new offer(s).`);
      await fetchSavedJobs();
    } catch (error) {
      setStatusMessage(`Error: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleClearOffers = async () => {
    try {
      await invoke("clear_all_jobs");
      setStatusMessage("All offers cleared.");
      await fetchSavedJobs();
    } catch (error) {
      setStatusMessage(`Error: ${error}`);
    } finally {
      setShowConfirmClear(false);
    }
  };

  const handleShareAllOffers = async () => {
    if (jobs.length === 0) {
      setStatusMessage("No offers to share.");
      return;
    }
    setStatusMessage("Drafting email in Apple Mail...");
    try {
      await invoke("draft_all_offers", { recipient: recipientEmail });
      setStatusMessage(`Draft created in Apple Mail with ${jobs.length} offers!`);
    } catch (error) {
      console.error("Failed to draft email via AppleScript:", error);
      setStatusMessage("Apple Mail draft failed, attempting local client...");
      
      const subject = encodeURIComponent(`Alternance Finder: Found Offers (${jobs.length} offers)`);
      let bodyText = `Hi,\n\nHere are the job offers found by Alternance Finder:\n\n`;
      jobs.forEach((job, i) => {
        bodyText += `${i + 1}. ${job.title} at ${job.company}\n   Location: ${job.location}\n   Source: ${job.source}\n   Date Announced: ${job.age_label}\n   URL: ${job.url}\n\n`;
      });
      bodyText += `Good luck!`;
      
      const body = encodeURIComponent(bodyText);
      const mailtoUrl = `mailto:${recipientEmail}?subject=${subject}&body=${body}`;
      
      if (mailtoUrl.length > 2000) {
        setStatusMessage("Error: The list is too long for standard mailto. Please open Apple Mail app.");
      } else {
        window.location.href = mailtoUrl;
        setStatusMessage("Opened default mail client.");
      }
    }
  };

  const handleDeleteJob = async (url) => {
    try {
      await invoke("delete_job", { url });
      setJobs(prevJobs => prevJobs.filter(job => job.url !== url));
    } catch (error) {
      console.error("Failed to delete job offer:", error);
      setStatusMessage(`Error: ${error}`);
    }
  };

  const sendEmail = (job) => {
    const subject = encodeURIComponent(`Job Opportunity: ${job.title} at ${job.company}`);
    const body = encodeURIComponent(`Hi,\n\nI found this job opportunity for you:\n\nTitle: ${job.title}\nCompany: ${job.company}\nLocation: ${job.location}\nSource: ${job.source}\nURL: ${job.url}\n\nGood luck!`);
    window.location.href = `mailto:${recipientEmail}?subject=${subject}&body=${body}`;
  };

  const handleCvUpload = async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    setCvFileName(file.name);
    setCvUploadLoading(true);
    setStatusMessage("Extracting skills from CV...");

    try {
      const arrayBuffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(arrayBuffer));
      const extractedSkills = await invoke("parse_cv_bytes", { bytes });
      
      setCvSkills(extractedSkills);
      if (extractedSkills.length > 0) {
        setStatusMessage(`CV Parsed! Found ${extractedSkills.length} skills. Added to CV Profile.`);
      } else {
        setStatusMessage("CV Parsed, but no matching skills were found. You can add them manually.");
      }
    } catch (error) {
      console.error("Error parsing CV:", error);
      setStatusMessage("Failed to parse CV. Ensure it contains readable text.");
    } finally {
      setCvUploadLoading(false);
    }
  };

  const handleSyncSkillsToKeywords = () => {
    if (cvSkills.length === 0) {
      setStatusMessage("No CV skills to sync. Upload a CV first!");
      return;
    }
    const newKeywords = Array.from(new Set([...keywords, ...cvSkills]));
    setKeywords(newKeywords);
    setStatusMessage("Search keywords updated from your CV skills!");
  };

  const addSkill = (e) => {
    if (e.key === "Enter" && newSkillInput.trim()) {
      const formatted = newSkillInput.trim();
      const capitalized = formatted.charAt(0).toUpperCase() + formatted.slice(1);
      if (!cvSkills.includes(capitalized)) {
        setCvSkills([...cvSkills, capitalized]);
      }
      setNewSkillInput("");
    }
  };

  const removeSkill = (skillToRemove) => {
    setCvSkills(cvSkills.filter(s => s !== skillToRemove));
  };

  const escapeRegExp = (string) => {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  };

  const matchesWholeWordOrPhrase = (text, term) => {
    if (!term || !text) return false;
    const cleanTerm = term.toLowerCase().trim();
    if (!cleanTerm) return false;
    
    const escaped = escapeRegExp(cleanTerm);
    // Matches as whole word, respecting symbols like C++ or C#
    const regex = new RegExp(`(?:^|[^a-z0-9+#])${escaped}(?:$|[^a-z0-9+#])`, 'i');
    return regex.test(text);
  };

  const calculateMatchScore = (jobTitle, companyName) => {
    if (!jobTitle) return { score: 0, matching: [] };
    const textToSearch = `${jobTitle} ${companyName}`.toLowerCase();
    const matching = [];

    // 1. Match against user's CV Skills / Roles with word boundary check
    if (cvSkills && cvSkills.length > 0) {
      cvSkills.forEach(skill => {
        const skillLower = skill.toLowerCase().trim();
        if (!skillLower) return;

        if (skillLower === "node.js" || skillLower === "nodejs" || skillLower === "node") {
          if (matchesWholeWordOrPhrase(textToSearch, "node") || matchesWholeWordOrPhrase(textToSearch, "nodejs") || matchesWholeWordOrPhrase(textToSearch, "node.js")) {
            matching.push(skill);
          }
        } else if (skillLower === "fullstack" || skillLower === "full stack") {
          if (textToSearch.includes("fullstack") || textToSearch.includes("full stack")) {
            matching.push(skill);
          }
        } else {
          if (matchesWholeWordOrPhrase(textToSearch, skillLower)) {
            matching.push(skill);
          }
        }
      });
    }

    // 2. Match against active search keywords (excluding generic French stop words & contract terms)
    let keywordMatches = [];
    const genericTerms = ["alternance", "stage", "cdd", "cdi", "emploi", "job", "de", "en", "h/f", "f/h", "à", "dans", "le", "la", "les"];
    keywords.forEach(kw => {
      const kwLower = kw.toLowerCase().trim();
      if (kwLower && !genericTerms.includes(kwLower) && matchesWholeWordOrPhrase(textToSearch, kwLower)) {
        keywordMatches.push(kw);
      }
    });

    // Combine unique matching items
    const allMatches = Array.from(new Set([...matching, ...keywordMatches]));

    // If there are no matches with either CV skills or search keywords, score is 0
    if (allMatches.length === 0) {
      return { score: 0, matching: [] };
    }

    // Calculate score based on degree of match
    const baseScore = 50;
    const bonusPerMatch = 20;
    const score = Math.min(100, baseScore + (allMatches.length - 1) * bonusPerMatch);
    
    return { score, matching: allMatches };
  };

  const isMatchingContractType = (title, currentContractType) => {
    if (!currentContractType || currentContractType === "all") return true;
    const titleLower = (title || "").toLowerCase();
    
    const isAlternanceTitle = titleLower.includes("alternan") || 
      titleLower.includes("apprenti") || 
      titleLower.includes("contrat pro") || 
      titleLower.includes("professionnalisation");
      
    const isExplicitFulltime = (titleLower.includes("cdi") || 
      titleLower.includes("cdd") || 
      titleLower.includes("full-time") || 
      titleLower.includes("fulltime") || 
      titleLower.includes("temps plein")) && !isAlternanceTitle;
      
    if (currentContractType === "alternance") {
      if (isExplicitFulltime) return false;
      return true;
    } else if (currentContractType === "fulltime") {
      if (isAlternanceTitle || titleLower.includes("stage") || titleLower.includes("internship")) return false;
      return true;
    }
    return true;
  };

  const getSortedAndScoredJobs = () => {
    let list = jobs
      .filter(job => isMatchingContractType(job.title, contractType))
      .map(job => {
        const { score, matching } = calculateMatchScore(job.title, job.company);
        return { ...job, matchScore: score, matchingSkills: matching };
      });

    if (hideZeroMatches) {
      list = list.filter(j => j.matchScore > 0);
    }

    if (sortBy === "match") {
      return [...list].sort((a, b) => b.matchScore - a.matchScore);
    }
    
    return list;
  };

  const handleViewOffer = async (url) => {
    try {
      await invoke("open_in_browser", { url });
    } catch (error) {
      console.error("Failed to open URL in browser:", error);
      window.open(url, "_blank"); // Fallback
    }
  };

  return (
    <div className="app-container">
      <header className="app-header">
        <h1>Alternance Finder <span style={{ fontSize: "0.8rem", padding: "0.2rem 0.5rem", borderRadius: "12px", background: "rgba(99, 102, 241, 0.2)", color: "#818cf8", verticalAlign: "middle", marginLeft: "0.5rem" }}>v1.0.0</span></h1>
        <p>Automate your job search with ease.</p>
      </header>

      <main className="app-grid">
        <section className="controls-panel">
          <h2>Search Configuration</h2>
          
          <div className="form-group">
            <label>Keywords (Press Enter to add)</label>
            <input 
              type="text" 
              value={keywordInput} 
              onKeyDown={addKeyword}
              onChange={(e) => setKeywordInput(e.target.value)} 
              placeholder="e.g. software engineer"
            />
            <div className="keyword-tags">
              {keywords.map((kw, index) => (
                <span key={index} className="keyword-tag">
                  {kw}
                  <button onClick={() => removeKeyword(index)}>×</button>
                </span>
              ))}
            </div>
          </div>
 
          <div className="form-group cv-profiler-section">
            <label>CV Profile Matcher</label>
            <div className="cv-upload-box">
              <input 
                type="file" 
                id="cv-file-input" 
                accept=".pdf" 
                onChange={handleCvUpload} 
                style={{ display: 'none' }} 
              />
              <label htmlFor="cv-file-input" className="cv-upload-label">
                {cvUploadLoading ? "Analyzing CV..." : cvFileName ? `📄 ${cvFileName}` : "Upload ATS CV (PDF)"}
              </label>
              {cvFileName && (
                <button 
                  onClick={() => { setCvFileName(""); setCvSkills([]); }} 
                  className="btn-clear-cv" 
                  title="Remove CV"
                >
                  ×
                </button>
              )}
            </div>
            {cvSkills.length > 0 && (
              <div className="cv-skills-container">
                <span className="cv-skills-title">Extracted Skills:</span>
                <div className="keyword-tags cv-skills-tags">
                  {cvSkills.map((skill, index) => (
                    <span key={index} className="keyword-tag cv-skill-tag">
                      {skill}
                      <button onClick={() => removeSkill(skill)}>×</button>
                    </span>
                  ))}
                </div>
                <div className="add-skill-group">
                  <input 
                    type="text" 
                    value={newSkillInput} 
                    onKeyDown={addSkill}
                    onChange={(e) => setNewSkillInput(e.target.value)} 
                    placeholder="Add skill (press Enter)"
                    className="input-add-skill"
                  />
                </div>
                <button onClick={handleSyncSkillsToKeywords} className="btn-sync-keywords">
                  Sync to Search Keywords
                </button>
              </div>
            )}
          </div>

          <div className="form-group">
            <label>Location</label>
            <input 
              type="text" 
              value={location} 
              onChange={(e) => setLocation(e.target.value)} 
            />
          </div>

          <div className="form-group">
            <label>Max Age (days)</label>
            <input 
              type="number" 
              value={maxAge} 
              onChange={(e) => setMaxAge(e.target.value)} 
            />
          </div>

          <div className="form-group">
            <label>Contract Type</label>
            <select 
              value={contractType} 
              onChange={(e) => setContractType(e.target.value)}
              className="select-interval"
            >
              <option value="alternance">Apprenticeship / Alternance</option>
              <option value="fulltime">Full-time (CDI / CDD)</option>
              <option value="all">All Contracts</option>
            </select>
          </div>

          <div className="form-group">
            <label>Recipient Email</label>
            <input 
              type="email" 
              value={recipientEmail} 
              onChange={(e) => setRecipientEmail(e.target.value)} 
            />
            <span className="input-hint">Used for manually sharing and for background auto-updates.</span>
          </div>

          <hr className="panel-divider" />

          <button 
            onClick={handleRunScraper} 
            disabled={loading}
            className="btn-trigger"
          >
            {loading ? "Searching..." : "Start Search"}
          </button>

          {statusMessage && <p className="status-banner">{statusMessage}</p>}
        </section>

        <section className="feed-panel">
          <div className="feed-header">
            <div className="feed-header-title">
              <h2>Found Offers ({getSortedAndScoredJobs().length}{hideZeroMatches && jobs.length !== getSortedAndScoredJobs().length ? ` of ${jobs.length}` : ""})</h2>
            </div>
            
            {jobs.length > 0 && (
              <div className="feed-header-toolbar">
                <div className="sort-controls">
                  <label className="filter-checkbox-label">
                    <input 
                      type="checkbox" 
                      checked={hideZeroMatches} 
                      onChange={(e) => setHideZeroMatches(e.target.checked)} 
                    />
                    <span>Hide 0% Matches</span>
                  </label>

                  <div className="sort-group">
                    <span className="sort-label">Sort:</span>
                    <select 
                      value={sortBy} 
                      onChange={(e) => setSortBy(e.target.value)} 
                      className="sort-select"
                    >
                      <option value="match">Match Score</option>
                      <option value="date">Date Added</option>
                    </select>
                  </div>
                </div>

                <div className="feed-actions">
                  <button onClick={handleShareAllOffers} className="btn-share-all" disabled={loading}>
                    Share All via Email
                  </button>
                  {showConfirmClear ? (
                    <div className="clear-confirm-group">
                      <span className="confirm-text">Confirm clear?</span>
                      <button onClick={handleClearOffers} className="btn-clear-confirm-yes">Yes</button>
                      <button onClick={() => setShowConfirmClear(false)} className="btn-clear-confirm-no">No</button>
                    </div>
                  ) : (
                    <button onClick={() => setShowConfirmClear(true)} className="btn-clear">
                      Clear Offers
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>
          <div className="job-list">
            {jobs.length === 0 ? (
              <p className="empty-state">No offers found yet. Start the search!</p>
            ) : (
              getSortedAndScoredJobs().map((job, index) => {
                const hasMatch = job.matchScore > 0;
                let badgeClass = "badge-match-low";
                if (job.matchScore >= 70) badgeClass = "badge-match-high";
                else if (job.matchScore >= 40) badgeClass = "badge-match-med";

                return (
                  <div key={index} className={`job-card ${hasMatch ? 'job-card-matched' : ''}`}>
                    <div className="job-card-header">
                      <h3>{job.title}</h3>
                      <div className="job-card-badges">
                        {hasMatch && (
                          <span className={`badge-match ${badgeClass}`} title={`Matching skills: ${job.matchingSkills.join(', ')}`}>
                            {job.matchScore}% Match
                          </span>
                        )}
                        <span className="badge-source">{job.source}</span>
                      </div>
                    </div>
                    <p className="job-meta">{job.company} — {job.location}</p>
                    
                    {hasMatch && job.matchingSkills && job.matchingSkills.length > 0 && (
                      <div className="matching-skills-list">
                        <span className="match-label">Matches:</span>
                        {job.matchingSkills.map((sk, sIdx) => (
                          <span key={sIdx} className="matching-skill-item">{sk}</span>
                        ))}
                      </div>
                    )}

                    <div className="job-card-footer">
                      <span className="badge-age">{job.age_label}</span>
                      <div className="actions">
                        <button onClick={() => sendEmail(job)} className="btn-email">
                          Share via Email
                        </button>
                        <button onClick={() => handleDeleteJob(job.url)} className="btn-delete">
                          Erase
                        </button>
                        <button onClick={() => handleViewOffer(job.url)} className="btn-apply">
                          View Offer
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </section>
      </main>
    </div>
  );
}

class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: "2rem", color: "#f87171", fontFamily: "sans-serif" }}>
          <h2>Application Error</h2>
          <p>Alternance Finder v1.0.0 encountered an error:</p>
          <pre style={{ background: "#1e1e1e", color: "#f87171", padding: "1rem", borderRadius: "8px", overflow: "auto" }}>
            {this.state.error?.toString()}
          </pre>
          <button onClick={() => window.location.reload()} style={{ padding: "0.5rem 1rem", marginTop: "1rem", cursor: "pointer", borderRadius: "6px", background: "#6366f1", color: "#fff", border: "none" }}>
            Reload App
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

const container = document.getElementById("root");
const root = createRoot(container);
root.render(
  <ErrorBoundary>
    <App />
  </ErrorBoundary>
);