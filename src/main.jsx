import React, { useState, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

const SKILL_PRESETS = [
  {
    id: "backend",
    label: "Backend",
    keywords: ["développeur backend", "python", "node.js", "java"],
    skills: ["Backend Developer", "Python", "Node.js", "SQL", "PostgreSQL", "Docker", "Git"]
  },
  {
    id: "fullstack",
    label: "Fullstack",
    keywords: ["développeur fullstack", "développeur web", "react", "node.js"],
    skills: ["Fullstack Developer", "React", "Node.js", "TypeScript", "JavaScript", "SQL", "Git"]
  },
  {
    id: "frontend",
    label: "Frontend",
    keywords: ["développeur frontend", "react", "vue.js", "javascript"],
    skills: ["Frontend Developer", "React", "Vue.js", "TypeScript", "JavaScript", "Git"]
  },
  {
    id: "data",
    label: "Data",
    keywords: ["data analyst", "data engineer", "python", "sql"],
    skills: ["Data Analyst", "Python", "SQL", "PostgreSQL", "Machine Learning", "Git"]
  },
  {
    id: "sales",
    label: "Sales/BD",
    keywords: ["business developer", "commercial", "account manager"],
    skills: ["Commercial", "Business Developer", "CRM", "Sales", "Prospection", "Négociation"]
  },
  {
    id: "marketing",
    label: "Marketing",
    keywords: ["marketing digital", "communication", "growth marketing"],
    skills: ["Marketing", "Communication", "SEO", "Growth Hacking", "Community Manager"]
  }
];

function App() {
  const [keywordInput, setKeywordInput] = useState("");
  const [keywords, setKeywords] = useState([]);
  const [expandKeywords, setExpandKeywords] = useState(false);
  const [location, setLocation] = useState("Paris");
  const [maxAge, setMaxAge] = useState(2);
  const [recipientEmail, setRecipientEmail] = useState("igor.luna.it@gmail.com");
  const [jobs, setJobs] = useState([]);
  const [loading, setLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState("");
  const [showConfirmClear, setShowConfirmClear] = useState(false);
  
  // Automation settings
  const [autoScrape, setAutoScrape] = useState(false);
  const [scrapeInterval, setScrapeInterval] = useState("4");
  const [contractType, setContractType] = useState("alternance");
  const [isLoaded, setIsLoaded] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Modals & Popovers
  const [isSkillsSheetOpen, setIsSkillsSheetOpen] = useState(false);
  const [isExcludedPopoverOpen, setIsExcludedPopoverOpen] = useState(false);
  const [openMenuJobUrl, setOpenMenuJobUrl] = useState(null);

  // CV & Skills state
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
  const [lastExtractedSkills, setLastExtractedSkills] = useState(() => {
    try {
      const saved = localStorage.getItem("lastExtractedSkills");
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });
  const [selectedPresetId, setSelectedPresetId] = useState("fullstack");
  const [cvUploadLoading, setCvUploadLoading] = useState(false);
  const [sortBy, setSortBy] = useState("date");
  const [newSkillInput, setNewSkillInput] = useState("");
  const [hideZeroMatches, setHideZeroMatches] = useState(false);

  // Ignore / Excluded options state
  const [ignoreList, setIgnoreList] = useState(() => {
    try {
      const saved = localStorage.getItem("ignoreList");
      return saved ? JSON.parse(saved) : ["Studi CFA", "Studi"];
    } catch {
      return ["Studi CFA", "Studi"];
    }
  });
  const [ignoreInput, setIgnoreInput] = useState("");

  const cvFileInputRef = useRef(null);

  // Sync to localStorage
  useEffect(() => {
    localStorage.setItem("cvSkills", JSON.stringify(cvSkills));
  }, [cvSkills]);

  useEffect(() => {
    localStorage.setItem("cvFileName", cvFileName);
  }, [cvFileName]);

  useEffect(() => {
    localStorage.setItem("lastExtractedSkills", JSON.stringify(lastExtractedSkills));
  }, [lastExtractedSkills]);

  useEffect(() => {
    localStorage.setItem("ignoreList", JSON.stringify(ignoreList));
  }, [ignoreList]);

  // Global keydown handler for Esc & Return
  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.key === "Escape") {
        if (isSkillsSheetOpen) setIsSkillsSheetOpen(false);
        if (isExcludedPopoverOpen) setIsExcludedPopoverOpen(false);
        if (openMenuJobUrl) setOpenMenuJobUrl(null);
        if (showConfirmClear) setShowConfirmClear(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isSkillsSheetOpen, isExcludedPopoverOpen, openMenuJobUrl, showConfirmClear]);

  // Close open row menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e) => {
      if (openMenuJobUrl && !e.target.closest(".btn-dots-menu") && !e.target.closest(".row-context-menu")) {
        setOpenMenuJobUrl(null);
      }
    };
    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  }, [openMenuJobUrl]);

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
          if (settings.ignore_list) {
            const ignList = settings.ignore_list.split(",").map(k => k.trim()).filter(Boolean);
            if (ignList.length > 0) {
              setIgnoreList(ignList);
            }
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
        unlisten = await listen("new-jobs-auto-scraped", () => {
          fetchSavedJobs();
          setStatusMessage("New job offers found in background and saved!");
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
            contract_type: contractType,
            ignore_list: ignoreList.join(", ")
          }
        });
        await fetchSavedJobs();
      } catch (error) {
        console.error("Failed to save settings to DB:", error);
      } finally {
        setIsSaving(false);
      }
    }, 600);

    return () => clearTimeout(timer);
  }, [keywords, location, maxAge, recipientEmail, autoScrape, scrapeInterval, contractType, ignoreList, isLoaded]);

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

  const addIgnoreItem = (e) => {
    if (e.key === "Enter" && ignoreInput.trim()) {
      const val = ignoreInput.trim();
      if (!ignoreList.some(item => item.toLowerCase() === val.toLowerCase())) {
        setIgnoreList([...ignoreList, val]);
        setStatusMessage(`Added "${val}" to ignore list.`);
      }
      setIgnoreInput("");
    }
  };

  const removeIgnoreItem = (indexToRemove) => {
    const removed = ignoreList[indexToRemove];
    setIgnoreList(ignoreList.filter((_, index) => index !== indexToRemove));
    setStatusMessage(`Removed "${removed}" from ignore list.`);
  };

  const handleIgnoreCompany = (companyName) => {
    if (!companyName || companyName === "Entreprise partenaire") return;
    const trimmed = companyName.trim();
    if (!ignoreList.some(item => item.toLowerCase() === trimmed.toLowerCase())) {
      setIgnoreList([...ignoreList, trimmed]);
      setStatusMessage(`Added "${trimmed}" to ignore list.`);
    }
    setOpenMenuJobUrl(null);
  };

  const handleRunScraper = async () => {
    if (keywords.length === 0) {
      setStatusMessage("Please add at least one keyword.");
      return;
    }
    setLoading(true);
    setStatusMessage("Searching for new opportunities...");
    try {
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
      setStatusMessage("Apple Mail draft failed, opening default mail client...");
      
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
    } finally {
      setOpenMenuJobUrl(null);
    }
  };

  const sendEmail = (job) => {
    const subject = encodeURIComponent(`Job Opportunity: ${job.title} at ${job.company}`);
    const body = encodeURIComponent(`Hi,\n\nI found this job opportunity for you:\n\nTitle: ${job.title}\nCompany: ${job.company}\nLocation: ${job.location}\nSource: ${job.source}\nURL: ${job.url}\n\nGood luck!`);
    window.location.href = `mailto:${recipientEmail}?subject=${subject}&body=${body}`;
    setOpenMenuJobUrl(null);
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
      setLastExtractedSkills(extractedSkills);
      setSelectedPresetId(null);
      if (extractedSkills.length > 0) {
        setStatusMessage(`CV Parsed! Found ${extractedSkills.length} skills.`);
      } else {
        setStatusMessage("CV Parsed, but no matching skills found. Pick a preset or add manually.");
      }
    } catch (error) {
      console.error("Error parsing CV:", error);
      setStatusMessage("Failed to parse CV. Ensure it contains readable text.");
    } finally {
      setCvUploadLoading(false);
    }
  };

  const handleResetToCv = () => {
    if (lastExtractedSkills && lastExtractedSkills.length > 0) {
      setCvSkills([...lastExtractedSkills]);
      setStatusMessage(`Reset to CV extracted skills (${lastExtractedSkills.length} skills).`);
    }
  };

  const applySkillPreset = (preset) => {
    setCvSkills([...preset.skills]);
    setSelectedPresetId(preset.id);
    if (preset.keywords && preset.keywords.length > 0) {
      setKeywords([...preset.keywords]);
    }
    setStatusMessage(`Loaded '${preset.label}' preset.`);
  };

  const handleRemoveCvFile = (e) => {
    e.stopPropagation();
    setCvFileName("");
    setLastExtractedSkills([]);
    localStorage.removeItem("cvFileName");
    localStorage.removeItem("lastExtractedSkills");
    setStatusMessage("CV file removed.");
  };

  const handleSyncSkillsToKeywords = () => {
    if (cvSkills.length === 0) {
      setStatusMessage("No CV skills to sync. Upload a CV first!");
      return;
    }
    const newKeywords = Array.from(new Set([...keywords, ...cvSkills]));
    setKeywords(newKeywords);
    setStatusMessage("Search keywords updated with your skills!");
    setIsSkillsSheetOpen(false);
  };

  const handleReplaceKeywordsWithSkills = () => {
    if (cvSkills.length === 0) {
      setStatusMessage("No CV skills to sync. Upload a CV first!");
      return;
    }
    setKeywords([...cvSkills]);
    setStatusMessage(`Keywords replaced with ${cvSkills.length} skills!`);
    setIsSkillsSheetOpen(false);
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

  const addSkillBtn = () => {
    if (newSkillInput.trim()) {
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

  // Unified Orange "Reset" Button at sidebar bottom
  const handleUnifiedReset = () => {
    setKeywords(["développeur fullstack", "développeur web", "react", "node.js"]);
    const defaultPreset = SKILL_PRESETS.find(p => p.id === "fullstack");
    if (defaultPreset) {
      setCvSkills([...defaultPreset.skills]);
      setSelectedPresetId("fullstack");
    }
    setIgnoreList(["Studi CFA", "Studi"]);
    setStatusMessage("Search configuration reset to defaults.");
  };

  const escapeRegExp = (string) => {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  };

  const matchesWholeWordOrPhrase = (text, term) => {
    if (!term || !text) return false;
    const cleanTerm = term.toLowerCase().trim();
    if (!cleanTerm) return false;
    
    const escaped = escapeRegExp(cleanTerm);
    const regex = new RegExp(`(?:^|[^a-z0-9+#])${escaped}(?:$|[^a-z0-9+#])`, 'i');
    return regex.test(text);
  };

  const calculateMatchScore = (jobTitle, companyName) => {
    if (!jobTitle) return { score: 0, matching: [] };
    const textToSearch = `${jobTitle} ${companyName}`.toLowerCase();
    const matching = [];

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

    let keywordMatches = [];
    const genericTerms = ["alternance", "stage", "cdd", "cdi", "emploi", "job", "de", "en", "h/f", "f/h", "à", "dans", "le", "la", "les"];
    keywords.forEach(kw => {
      const kwLower = kw.toLowerCase().trim();
      if (kwLower && !genericTerms.includes(kwLower) && matchesWholeWordOrPhrase(textToSearch, kwLower)) {
        keywordMatches.push(kw);
      }
    });

    const allMatches = Array.from(new Set([...matching, ...keywordMatches]));

    if (allMatches.length === 0) {
      return { score: 0, matching: [] };
    }

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

  const isIgnored = (job) => {
    const compLower = (job.company || "").toLowerCase();
    const titleLower = (job.title || "").toLowerCase();
    return ignoreList.some(item => {
      const term = item.toLowerCase().trim();
      return term && (compLower.includes(term) || titleLower.includes(term));
    });
  };

  const getSortedAndScoredJobs = () => {
    let list = jobs
      .filter(job => !isIgnored(job))
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
      window.open(url, "_blank");
    }
  };

  const visibleKeywords = expandKeywords ? keywords : keywords.slice(0, 3);
  const hiddenCount = keywords.length - 3;
  const filteredJobs = getSortedAndScoredJobs();

  return (
    <div className="macos-window">
      {/* 52px macOS Title Bar */}
      <header className="macos-titlebar">
        <div className="titlebar-left">
          <span className="app-title">Alternance Finder</span>
          <span className="version-pill">v1.2</span>
        </div>
        <div className="titlebar-right">
          <button 
            onClick={handleShareAllOffers} 
            className="btn-mac-secondary" 
            disabled={jobs.length === 0 || loading}
          >
            Share all
          </button>
          {showConfirmClear ? (
            <div style={{ display: "inline-flex", gap: "6px", alignItems: "center" }}>
              <span style={{ fontSize: "12px", color: "var(--danger)" }}>Clear all?</span>
              <button onClick={handleClearOffers} className="btn-mac-secondary" style={{ color: "var(--danger)" }}>Yes</button>
              <button onClick={() => setShowConfirmClear(false)} className="btn-mac-secondary">No</button>
            </div>
          ) : (
            <button 
              onClick={() => setShowConfirmClear(true)} 
              className="btn-mac-secondary" 
              disabled={jobs.length === 0}
            >
              Clear offers
            </button>
          )}
        </div>
      </header>

      {/* Main Window Body: 300px Sidebar + Content */}
      <div className="macos-body">
        {/* Left Sidebar (300px) */}
        <aside className="macos-sidebar">
          {/* 1. Search field (30px) */}
          <div className="sidebar-search-box">
            <svg className="search-magnifier-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <circle cx="7" cy="7" r="4.5" />
              <line x1="10.5" y1="10.5" x2="14" y2="14" />
            </svg>
            <input 
              type="text" 
              value={keywordInput} 
              onKeyDown={addKeyword}
              onChange={(e) => setKeywordInput(e.target.value)} 
              placeholder="Job title or keyword"
              className="sidebar-search-input"
              aria-label="Job title or keyword search"
            />
          </div>

          {/* 2. Keyword chips (24px pills) */}
          {keywords.length > 0 && (
            <div className="keyword-chips-wrap">
              {visibleKeywords.map((kw, index) => (
                <span key={index} className="mac-chip">
                  <span>{kw}</span>
                  <button 
                    onClick={() => removeKeyword(index)} 
                    className="chip-remove-btn"
                    aria-label={`Remove keyword ${kw}`}
                  >
                    ×
                  </button>
                </span>
              ))}
              {hiddenCount > 0 && !expandKeywords && (
                <button 
                  onClick={() => setExpandKeywords(true)} 
                  className="chip-more-btn"
                >
                  +{hiddenCount} more
                </button>
              )}
              {expandKeywords && hiddenCount > 0 && (
                <button 
                  onClick={() => setExpandKeywords(false)} 
                  className="chip-more-btn"
                >
                  Show less
                </button>
              )}
            </div>
          )}

          {/* 3. Section Profile */}
          <div className="grouped-section">
            <span className="section-label">Profile</span>
            <div className="grouped-list">
              {/* CV Row */}
              <div 
                className="grouped-row grouped-row-clickable" 
                onClick={() => cvFileInputRef.current && cvFileInputRef.current.click()}
              >
                <input 
                  type="file" 
                  ref={cvFileInputRef}
                  accept=".pdf" 
                  onClick={(e) => { e.target.value = null; }}
                  onChange={handleCvUpload} 
                  style={{ display: "none" }} 
                />
                <span className="row-label">CV</span>
                <div className="row-content-right">
                  {cvUploadLoading ? (
                    <span>Analyzing...</span>
                  ) : cvFileName ? (
                    <>
                      <span title={cvFileName} style={{ maxWidth: "160px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {cvFileName}
                      </span>
                      <button 
                        onClick={handleRemoveCvFile} 
                        className="chip-remove-btn" 
                        aria-label="Remove CV file"
                        style={{ marginLeft: "4px" }}
                      >
                        ×
                      </button>
                    </>
                  ) : (
                    <span style={{ color: "var(--accent)" }}>Upload PDF</span>
                  )}
                  <span className="row-chevron">›</span>
                </div>
              </div>

              {/* Skills Row */}
              <div 
                className="grouped-row grouped-row-clickable"
                onClick={() => setIsSkillsSheetOpen(true)}
              >
                <span className="row-label">Skills</span>
                <div className="row-content-right">
                  <span>{cvSkills.length} active</span>
                  <span className="row-chevron">›</span>
                </div>
              </div>
            </div>
          </div>

          {/* 4. Section Where and when */}
          <div className="grouped-section">
            <span className="section-label">Where and when</span>
            <div className="grouped-list">
              {/* Location */}
              <div className="grouped-row">
                <span className="row-label">Location</span>
                <div className="row-content-right">
                  <input 
                    type="text" 
                    value={location} 
                    onChange={(e) => setLocation(e.target.value)} 
                    className="row-inline-input"
                    aria-label="Location"
                    placeholder="e.g. Paris"
                  />
                </div>
              </div>

              {/* Posted within */}
              <div className="grouped-row">
                <span className="row-label">Posted within</span>
                <div className="row-content-right">
                  <div className="mac-select-wrap">
                    <select 
                      value={maxAge} 
                      onChange={(e) => setMaxAge(parseInt(e.target.value) || 2)} 
                      className="mac-popup-select"
                      aria-label="Posted within days"
                    >
                      <option value={1}>1 day</option>
                      <option value={2}>2 days</option>
                      <option value={3}>3 days</option>
                      <option value={7}>7 days</option>
                      <option value={14}>14 days</option>
                      <option value={30}>30 days</option>
                    </select>
                    <span className="mac-select-arrow">↕</span>
                  </div>
                </div>
              </div>

              {/* Contract */}
              <div className="grouped-row">
                <span className="row-label">Contract</span>
                <div className="row-content-right">
                  <div className="mac-select-wrap">
                    <select 
                      value={contractType} 
                      onChange={(e) => setContractType(e.target.value)} 
                      className="mac-popup-select"
                      aria-label="Contract type"
                    >
                      <option value="alternance">Alternance</option>
                      <option value="fulltime">Full-time</option>
                      <option value="all">All</option>
                    </select>
                    <span className="mac-select-arrow">↕</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* 5. Section Filtering */}
          <div className="grouped-section">
            <span className="section-label">Filtering</span>
            <div className="grouped-list">
              {/* Excluded */}
              <div 
                className="grouped-row grouped-row-clickable"
                onClick={() => setIsExcludedPopoverOpen(true)}
              >
                <span className="row-label">Excluded</span>
                <div className="row-content-right">
                  <span>{ignoreList.length} active</span>
                  <span className="row-chevron">›</span>
                </div>
              </div>

              {/* Email */}
              <div className="grouped-row">
                <span className="row-label">Email</span>
                <div className="row-content-right">
                  <input 
                    type="email" 
                    value={recipientEmail} 
                    onChange={(e) => setRecipientEmail(e.target.value)} 
                    className="row-inline-input"
                    aria-label="Recipient Email"
                    placeholder="email@domain.com"
                  />
                </div>
              </div>
            </div>
          </div>

          {/* Automation Section (Preserved & Styled) */}
          <div className="grouped-section">
            <span className="section-label">Automation</span>
            <div className="grouped-list">
              <div className="grouped-row">
                <span className="row-label">Auto-scrape</span>
                <div className="row-content-right">
                  <label className="mac-switch" aria-label="Toggle auto scrape">
                    <input 
                      type="checkbox" 
                      checked={autoScrape} 
                      onChange={(e) => setAutoScrape(e.target.checked)} 
                    />
                    <span className="mac-slider"></span>
                  </label>
                </div>
              </div>
              <div className="grouped-row">
                <span className="row-label">Interval</span>
                <div className="row-content-right">
                  <div className="mac-select-wrap">
                    <select 
                      value={scrapeInterval} 
                      onChange={(e) => setScrapeInterval(e.target.value)} 
                      className="mac-popup-select"
                      disabled={!autoScrape}
                      aria-label="Auto scrape interval"
                    >
                      <option value="2">Every 2h</option>
                      <option value="4">Every 4h</option>
                      <option value="8">Every 8h</option>
                      <option value="12">Every 12h</option>
                      <option value="24">Every 24h</option>
                    </select>
                    <span className="mac-select-arrow">↕</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* 6. Sidebar Footer: Start Search + Orange Reset Button */}
          <div className="sidebar-footer">
            <button 
              onClick={handleRunScraper} 
              disabled={loading}
              className="btn-mac-primary sidebar-start-btn"
            >
              {loading ? "Searching..." : "Start Search"}
            </button>

            {statusMessage && (
              <span className="sidebar-status-banner">{statusMessage}</span>
            )}

            <button 
              onClick={handleUnifiedReset} 
              className="btn-text-orange sidebar-reset-btn"
              title="Reset configuration to defaults"
            >
              Reset
            </button>
          </div>
        </aside>

        {/* Content Area / Offers Feed */}
        <main className="macos-content">
          <div className="feed-header-bar">
            <h1 className="feed-header-title">
              Found offers <span className="feed-count-secondary">{filteredJobs.length}</span>
            </h1>

            <div className="feed-toolbar-controls">
              <label className="toggle-filter-label">
                <span>Hide 0% matches</span>
                <span className="mac-switch">
                  <input 
                    type="checkbox" 
                    checked={hideZeroMatches} 
                    onChange={(e) => setHideZeroMatches(e.target.checked)} 
                  />
                  <span className="mac-slider"></span>
                </span>
              </label>

              <div className="sort-select-group">
                <span>Sort:</span>
                <div className="mac-select-wrap">
                  <select 
                    value={sortBy} 
                    onChange={(e) => setSortBy(e.target.value)} 
                    className="mac-popup-select"
                    aria-label="Sort offers"
                  >
                    <option value="date">Date added</option>
                    <option value="match">Match score</option>
                  </select>
                  <span className="mac-select-arrow">↕</span>
                </div>
              </div>
            </div>
          </div>

          <div className="offers-list-container">
            {filteredJobs.length === 0 ? (
              <div className="empty-offers-state">
                <span>No offers found.</span>
                <span style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                  Try broadening your keywords or click "Start Search".
                </span>
              </div>
            ) : (
              filteredJobs.map((job, index) => {
                const hasMatch = job.matchScore > 0;
                let badgeClass = "badge-low";
                if (job.matchScore >= 70) badgeClass = "badge-high";
                else if (job.matchScore >= 40) badgeClass = "badge-med";

                const isMenuOpen = openMenuJobUrl === job.url;

                return (
                  <div key={index} className={`offer-row ${hasMatch ? 'offer-row-matched' : ''}`}>
                    <div className="offer-left-details">
                      <div className="offer-title-wrap">
                        <h2 className="offer-title">{job.title}</h2>
                        {hasMatch && (
                          <span className={`offer-badge-match ${badgeClass}`} title={`Matching: ${job.matchingSkills?.join(', ')}`}>
                            {job.matchScore}% match
                          </span>
                        )}
                      </div>
                      <div className="offer-meta-line">
                        {job.company || "Entreprise partenaire"} · {job.location || "France"} · {job.source}
                      </div>
                    </div>

                    <div className="offer-right-actions">
                      <span className="offer-date">{job.age_label}</span>
                      <button 
                        onClick={() => handleViewOffer(job.url)} 
                        className="btn-view-offer"
                      >
                        View
                      </button>

                      <div style={{ position: "relative" }}>
                        <button 
                          onClick={(e) => {
                            e.stopPropagation();
                            setOpenMenuJobUrl(isMenuOpen ? null : job.url);
                          }} 
                          className="btn-dots-menu"
                          aria-label="More actions"
                          aria-expanded={isMenuOpen}
                        >
                          ···
                        </button>

                        {isMenuOpen && (
                          <div className="row-context-menu" role="menu">
                            <button 
                              onClick={() => sendEmail(job)} 
                              className="menu-item"
                              role="menuitem"
                            >
                              Share via email
                            </button>
                            {job.company && job.company !== "Entreprise partenaire" && (
                              <button 
                                onClick={() => handleIgnoreCompany(job.company)} 
                                className="menu-item"
                                role="menuitem"
                              >
                                Ignore company
                              </button>
                            )}
                            <div className="menu-divider" />
                            <button 
                              onClick={() => handleDeleteJob(job.url)} 
                              className="menu-item menu-item-danger"
                              role="menuitem"
                            >
                              Erase
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </main>
      </div>

      {/* SKILLS SHEET (480px, Dimmed Backdrop) */}
      {isSkillsSheetOpen && (
        <div className="mac-modal-backdrop" onClick={() => setIsSkillsSheetOpen(false)}>
          <div className="mac-sheet-card" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
            <div className="sheet-header">
              <div className="sheet-title-group">
                <h2>Skills</h2>
                <div className="sheet-caption">
                  {cvSkills.length} active{cvFileName ? ", taken from your CV" : ", from active profile"}
                </div>
              </div>
              <button 
                onClick={() => setIsSkillsSheetOpen(false)} 
                className="sheet-close-btn"
                aria-label="Close sheet"
              >
                ×
              </button>
            </div>

            {/* Quick sets: 6-segment control */}
            <div className="grouped-section">
              <span className="section-label">Quick sets</span>
              <div className="mac-segment-control" role="tablist">
                {SKILL_PRESETS.map((preset) => (
                  <button 
                    key={preset.id}
                    onClick={() => applySkillPreset(preset)}
                    className={`segment-btn ${selectedPresetId === preset.id ? "segment-btn-active" : ""}`}
                    role="tab"
                    aria-selected={selectedPresetId === preset.id}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Active well with chips */}
            <div className="grouped-section">
              <span className="section-label">Active</span>
              <div className="active-well">
                {cvSkills.length === 0 ? (
                  <span style={{ color: "var(--text-secondary)", fontSize: "12px", padding: "4px" }}>
                    No skills added yet. Click a quick set above or add skills below.
                  </span>
                ) : (
                  cvSkills.map((skill, index) => (
                    <span key={index} className="mac-chip">
                      <span>{skill}</span>
                      <button 
                        onClick={() => removeSkill(skill)} 
                        className="chip-remove-btn"
                        aria-label={`Remove skill ${skill}`}
                      >
                        ×
                      </button>
                    </span>
                  ))
                )}
              </div>
            </div>

            {/* Add skill input + button */}
            <div className="sheet-add-group">
              <input 
                type="text" 
                value={newSkillInput} 
                onKeyDown={addSkill}
                onChange={(e) => setNewSkillInput(e.target.value)} 
                placeholder="Add skill (press Enter)"
                className="sheet-add-input"
                aria-label="Add skill"
              />
              <button onClick={addSkillBtn} className="btn-mac-secondary">
                Add
              </button>
            </div>

            {/* Footer */}
            <div className="sheet-footer">
              <div>
                {lastExtractedSkills.length > 0 && (
                  <button onClick={handleResetToCv} className="btn-text-orange">
                    Reset to CV
                  </button>
                )}
              </div>
              <div className="sheet-footer-right">
                <button 
                  onClick={handleReplaceKeywordsWithSkills} 
                  className="btn-mac-secondary"
                >
                  Replace keywords
                </button>
                <button 
                  onClick={handleSyncSkillsToKeywords} 
                  className="btn-mac-primary"
                >
                  Add to keywords
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* EXCLUDED POPOVER (Modal / Popover) */}
      {isExcludedPopoverOpen && (
        <div className="mac-modal-backdrop" onClick={() => setIsExcludedPopoverOpen(false)}>
          <div className="popover-card" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
            <div className="sheet-header">
              <div className="sheet-title-group">
                <h2 style={{ fontSize: "16px", margin: 0 }}>Excluded Companies & Schools</h2>
                <div className="sheet-caption">
                  {ignoreList.length} excluded from results
                </div>
              </div>
              <button 
                onClick={() => setIsExcludedPopoverOpen(false)} 
                className="sheet-close-btn"
                aria-label="Close popover"
              >
                ×
              </button>
            </div>

            <div className="active-well" style={{ minHeight: "80px" }}>
              {ignoreList.length === 0 ? (
                <span style={{ color: "var(--text-secondary)", fontSize: "12px", padding: "4px" }}>
                  No companies or schools excluded.
                </span>
              ) : (
                ignoreList.map((item, index) => (
                  <span key={index} className="mac-chip">
                    <span>{item}</span>
                    <button 
                      onClick={() => removeIgnoreItem(index)} 
                      className="chip-remove-btn"
                      aria-label={`Remove excluded ${item}`}
                    >
                      ×
                    </button>
                  </span>
                ))
              )}
            </div>

            <div className="sheet-add-group">
              <input 
                type="text" 
                value={ignoreInput} 
                onKeyDown={addIgnoreItem}
                onChange={(e) => setIgnoreInput(e.target.value)} 
                placeholder="e.g. Studi CFA (press Enter)"
                className="sheet-add-input"
                aria-label="Exclude company or school"
              />
              <button 
                onClick={() => {
                  if (ignoreInput.trim()) {
                    const val = ignoreInput.trim();
                    if (!ignoreList.some(item => item.toLowerCase() === val.toLowerCase())) {
                      setIgnoreList([...ignoreList, val]);
                    }
                    setIgnoreInput("");
                  }
                }} 
                className="btn-mac-secondary"
              >
                Add
              </button>
            </div>

            <div className="sheet-footer" style={{ borderTop: "1px solid var(--hairline)", paddingTop: "8px" }}>
              <button 
                onClick={() => { setIgnoreList(["Studi CFA", "Studi"]); setStatusMessage("Ignore list reset to default."); }} 
                className="btn-text-orange"
              >
                Reset Defaults
              </button>
              <button 
                onClick={() => setIsExcludedPopoverOpen(false)} 
                className="btn-mac-secondary"
              >
                Done
              </button>
            </div>
          </div>
        </div>
      )}
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
        <div style={{ padding: "2rem", color: "var(--danger)", fontFamily: "var(--font-family)" }}>
          <h2>Application Error</h2>
          <p>Alternance Finder encountered an error:</p>
          <pre style={{ background: "var(--grouped-surface)", color: "var(--danger)", padding: "1rem", borderRadius: "8px", overflow: "auto" }}>
            {this.state.error?.toString()}
          </pre>
          <button onClick={() => window.location.reload()} className="btn-mac-primary" style={{ marginTop: "1rem" }}>
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
