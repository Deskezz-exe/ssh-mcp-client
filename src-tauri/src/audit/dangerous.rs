use std::sync::LazyLock;

use regex::RegexSet;

/// Patterns that block a command in `run_command` until confirmed via
/// `confirm_dangerous_command`. Intentionally coarse (false positives are
/// cheap — they just add a confirmation step; false negatives are not).
static DANGEROUS_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"\brm\b[^\n]*-[a-zA-Z]*r[a-zA-Z]*f|\brm\b[^\n]*-[a-zA-Z]*f[a-zA-Z]*r",
        r"\bdd\b\s+if=",
        r"\bmkfs(\.\w+)?\b",
        r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:",
        r"\bshutdown\b",
        r"\breboot\b",
        r"\bhalt\b",
        r"\bsystemctl\s+(stop|disable|mask)\b",
        r"\bservice\s+\S+\s+stop\b",
        r">\s*/dev/sd\w*",
        r">\s*/dev/nvme\w*",
        r"\bchmod\s+(-R\s+)?000\b",
        r"\bchmod\s+-R\s+777\s+/",
        r"\buserdel\b",
        r"\bdeluser\b",
        r"\bdrop\s+database\b",
        r"\btruncate\s+table\b",
        r"\bufw\s+disable\b",
        r"\biptables\s+-F\b",
        r"\bkill\s+-9\s+1\b",
        r"\b(apt|apt-get|yum|dnf)\s+(remove|purge)\s+-y\b",
        r">\s*/etc/passwd\b",
    ])
    .expect("dangerous command patterns must compile")
});

/// Returns a human-readable reason if `command` matches a known destructive
/// pattern, or `None` if it looks safe to run without extra confirmation.
pub fn is_dangerous(command: &str) -> Option<&'static str> {
    if DANGEROUS_PATTERNS.is_match(command) {
        Some("command matches a known destructive pattern (filesystem wipe, service/host shutdown, or similar)")
    } else {
        None
    }
}
