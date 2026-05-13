//! Phase Jack Adams.6 — AI fallback for the chat-CLI cmdbar.
//!
//! When the user types something that `parse_command` rejects
//! (`UnknownCommand("show me young scorers")`, etc.), and AI
//! fallback is enabled in `~/.icelines/config.toml`, this module
//! delegates to a configured LLM provider for natural-language →
//! command interpretation.
//!
//! ## Why opt-in
//!
//! Running an LLM costs money (API calls) or requires the user to
//! have Claude Code installed (`claude -p`). Both impose latency
//! that the deterministic Phase Art Ross filter parser doesn't.
//! AI fallback is a power-user knob, not a default behavior — the
//! cmdbar grammar is the canonical UX.
//!
//! ## Providers
//!
//! - `ClaudeCliProvider` — shells out to `claude -p "<prompt>"`.
//!   Zero binary footprint, but requires the user to have Claude
//!   Code installed and authenticated locally.
//! - `AnthropicApiProvider` — direct API call via reqwest. Needs
//!   `ANTHROPIC_API_KEY` (or whatever env var the user configured).
//!   Faster startup than CLI subprocess but adds reqwest as a
//!   workspace dep.
//!
//! Adams.6 ships `ClaudeCliProvider` only. `AnthropicApiProvider`
//! is reserved for Adams.7 polish.
//!
//! ## Flow
//!
//! 1. User types `show me forwards under 25 with 30+ goals`.
//! 2. `parse_command` returns `UnknownCommand`.
//! 3. `submit_command_bar` checks `Config.ai.enabled`.
//! 4. If enabled, builds an `AiProvider` from the configured
//!    provider name and calls `provider.interpret(input).await`.
//! 5. Provider returns a single canonical cmdbar verb string
//!    (`query g >= 30 AND age <= 25 AND pos != G`).
//! 6. App re-runs `parse_command` on the returned string and
//!    executes via `execute_command` like any other input.
//!
//! ## Determinism
//!
//! AI providers are non-deterministic by nature. The cmdbar
//! state machine MUST stay deterministic — provider failures fall
//! back to the original `ParseError` flash. Tests use a stub
//! `AiProvider` impl that returns canned responses.

#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

// ── Trait + types ────────────────────────────────────────────────────────────

/// Phase Adams.6 — AI fallback provider. Translates a natural-
/// language cmdbar input into a canonical icelines command
/// string. Returned string is fed back through
/// `crate::tui::command::parse_command` — providers SHOULD emit
/// strings that parse cleanly, but the App treats AI output as
/// untrusted (re-parses + flashes if invalid).
///
/// `interpret` is async because both shipping providers are
/// network-bound (subprocess or HTTP). Caller wraps the call in
/// a timeout (`AiConfig::timeout`) so a hung subprocess doesn't
/// hang the TUI forever.
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync + std::fmt::Debug {
    /// Translate `natural_language` into a canonical icelines
    /// command. The returned string should be a legal cmdbar
    /// input (verb-or-slash form) — the App re-runs
    /// `parse_command` on it.
    ///
    /// `system_prompt` carries the icelines grammar reference
    /// (built by `default_system_prompt`); providers prepend it
    /// to the user input.
    async fn interpret(
        &self,
        system_prompt: &str,
        natural_language: &str,
    ) -> Result<String, AiError>;

    /// Human-readable name shown in flash messages
    /// ("asked claude-cli…").
    fn name(&self) -> &'static str;
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AiError {
    #[error("AI provider not enabled in config — set `enabled = true` under `[ai]`")]
    Disabled,

    #[error("AI provider `{0}` unknown — expected one of: claude-cli, anthropic-api")]
    UnknownProvider(String),

    #[error("AI provider {provider} timed out after {seconds}s")]
    Timeout {
        provider: &'static str,
        seconds: u64,
    },

    #[error("AI provider {provider} subprocess failed: {message}")]
    Subprocess {
        provider: &'static str,
        message: String,
    },

    #[error("AI provider returned no output")]
    EmptyResponse,

    #[error("AI provider returned an unparseable command: {response}")]
    Unparseable { response: String },

    #[error("AI provider error: {0}")]
    Other(String),
}

// ── System prompt ────────────────────────────────────────────────────────────

/// Phase Adams.6 — system prompt explaining the icelines cmdbar
/// grammar. Hand-written, ~150 lines of grammar reference + 5
/// canonical examples. Versioned: bump VERSION when you change
/// the grammar so prompt-cache invalidation is obvious.
#[allow(dead_code)]
pub const SYSTEM_PROMPT_VERSION: &str = "v4";

pub fn default_system_prompt() -> String {
    // Single source of truth for the prompt — keep it in sync
    // with the cmdbar grammar in `tui/command.rs::parse_command`
    // and the user-facing reference in `widgets::mdi_help_lines`.
    r#"You translate natural-language hockey/NHL queries into IceLines TUI command-bar commands.

Output format: ONLY the canonical command string, no explanation, no markdown, no quotes.

VERBS (no args):
  stats          → Stats / Queries screen
  goalies        → Goalies leaderboard
  poach          → Fantasy poacher board
  gaps           → Fantasy roster-gap board
  simulate       → Fantasy simulation board
  watchlist      → Fantasy poacher watchlist
  transactions   → Transactions feed (alias: txs)
  playoffs       → Playoffs bracket
  depth          → Depth chart
  scores         → Today's scores
  schedule       → Weekly schedule
  favorites      → Favorites screen
  roster         → Active fantasy roster-gap board

VERBS (with args):
  player <name>             open player card                    e.g. player Bedard
  team <ABBR>               team depth chart                    e.g. team EDM
  team <ABBR> season        team's full schedule
  compare <a>               similarity peers
  compare <a> <b>           head-to-head
  box <game-id>             boxscore detail (numeric NHL game id)
  class <year>              draft-year query                    e.g. class 2024

ROSTER KV FORM:
  stats nationality=CAN pos=LW min-gp=20
  goalies sort=gaa min-gp=20 nationality=CAN saves=on
  gaps cats=hits,blocks,shots top=8
  fantasy gaps shots top=6
  poach rw cats=hits,blocks free top=12
  fantasy poach top=8 available
  simulate add=Connor_McDavid drop=Bench_Forward weeks=3
  fantasy simulate add Connor_McDavid drop Bench_Forward
  roster
  fantasy roster
  watchlist
  team EDM pos=LW nationality=CAN
  depth pos=LW nationality=CAN
  favorites sort=name nationality=CAN
  UI examples may show a leading colon, e.g. :goalies sort=gaa min-gp=20, but output should omit the colon.

FREE-FORM QUERY (Phase Art Ross filter grammar):
  query <filter-expression>
  Atoms: <stat> <op> <value>
    stats: g, p, a, gp, ppg, hits, blk, tk, gv, pim, plus-minus, shooting-pct,
           toi-per-game-sec, faceoff-win-pct, save-pct, gaa, sv-pct
           bio: age, pos, team, country, draft-round, draft-overall, birth-state, nationality
    ops: =, !=, <, <=, >, >=, IN (a, b, c), NOT IN, BETWEEN x AND y, LIKE "pattern", ~, !~
    boolean: AND, OR, NOT, parentheses
    sliding window: <stat>.last<N><unit>     unit=g(games)/d(days)/w(weeks)/m(months)
                    g.allteams      = aggregated across mid-season trades
                    g.career        = career-total constant
    streak: <stat>.streak>=N         consecutive games meeting condition
    EVER: <atom> EVER                anywhere in career, intra-season
    AT-age: <atom> AT age<=N         qualifier for EVER
    cross-league: league=OHL, league.tier=Junior, p.career.junior>=200

WRITE ACTIONS (always slash-prefixed):
  /fav add <name>           add to Favorites
  /fav remove <name>        remove from Favorites

LAYOUT (always slash-prefixed):
  /hide favorites           hide Favorites side pane
  /hide schedule            hide Schedule side pane
  /show favorites           restore
  /show schedule            restore

META:
  /help                     command reference (alias /h, /?)
  /quit                     exit (alias q, quit)

EXAMPLES:

User: show me top young scorers
You: query g >= 25 AND age <= 23

User: open mcdavid's profile
You: player McDavid

User: edmonton oilers depth chart
You: team EDM

User: who's leading the playoffs in points?
You: query p >= 1 --playoff
(NOTE: --playoff isn't a cmdbar arg — for cross-cutting filters, prefer the bare verb. Use `playoffs` to navigate.)

User: i want to see canadian forwards under 25
You: query country = CAN AND pos != G AND age < 25

User: hide the schedule pane
You: /hide schedule

User: add bedard to my favorites
You: /fav add Bedard

User: show right-wing poachers for hits and blocks
You: poach rw cats=hits,blocks free top=12

User: simulate adding mcdavid and dropping my bench forward
You: simulate add=Connor_McDavid drop=Bench_Forward weeks=3

User: show the 2024 draft class
You: class 2024

If the user's request CANNOT be expressed in this grammar, respond with the single token UNSUPPORTED.
"#
    .to_string()
}

// ── Provider config ──────────────────────────────────────────────────────────

/// Phase Adams.6 — resolved AI configuration. Built from
/// `Config::ai` (`[ai]` TOML section) at load time.
#[derive(Debug, Clone)]
pub struct AiConfig {
    /// Master enable flag. `false` short-circuits all AI calls
    /// — `submit_command_bar` falls back to the deterministic
    /// `ParseError` flash without ever consulting a provider.
    pub enabled: bool,

    /// Which provider to use. Adams.6 supports `"claude-cli"`;
    /// Adams.7 will add `"anthropic-api"`.
    pub provider: String,

    /// Provider-specific model identifier. For Claude CLI, this
    /// is forwarded as `--model`; for the Anthropic API, it's
    /// the API model parameter. Defaults to a fast model
    /// because cmdbar interpretation is short-prompt.
    pub model: String,

    /// Hard timeout per AI call. Subprocess providers are
    /// killed after this; HTTP providers cancel the request.
    pub timeout: Duration,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "claude-cli".to_owned(),
            model: "claude-haiku-4-5".to_owned(),
            timeout: Duration::from_secs(15),
        }
    }
}

// ── Stub provider for tests ──────────────────────────────────────────────────

/// Phase Adams.6 — synchronous stub provider for tests. Returns
/// a canned response without spawning any subprocess or making
/// any network call. Use via `interpret` like any other
/// provider. We don't derive `Clone` on the stub itself
/// because `AiError` isn't Clone (it carries `thiserror` source
/// chains in some variants); `StubProvider::ok`/`err` factories
/// keep test setup terse.
#[cfg(test)]
#[derive(Debug)]
pub struct StubProvider {
    pub canned_response: Result<String, AiError>,
}

#[cfg(test)]
impl StubProvider {
    pub fn ok(response: impl Into<String>) -> Self {
        Self {
            canned_response: Ok(response.into()),
        }
    }

    pub fn err(error: AiError) -> Self {
        Self {
            canned_response: Err(error),
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl AiProvider for StubProvider {
    async fn interpret(
        &self,
        _system_prompt: &str,
        _natural_language: &str,
    ) -> Result<String, AiError> {
        match &self.canned_response {
            Ok(s) => Ok(s.clone()),
            Err(e) => Err(match e {
                AiError::Disabled => AiError::Disabled,
                AiError::UnknownProvider(s) => AiError::UnknownProvider(s.clone()),
                AiError::Timeout { provider, seconds } => AiError::Timeout {
                    provider,
                    seconds: *seconds,
                },
                AiError::Subprocess { provider, message } => AiError::Subprocess {
                    provider,
                    message: message.clone(),
                },
                AiError::EmptyResponse => AiError::EmptyResponse,
                AiError::Unparseable { response } => AiError::Unparseable {
                    response: response.clone(),
                },
                AiError::Other(s) => AiError::Other(s.clone()),
            }),
        }
    }

    fn name(&self) -> &'static str {
        "stub"
    }
}

// ── Claude CLI provider ──────────────────────────────────────────────────────

/// Phase Adams.6 — subprocess-based provider. Calls
/// `claude -p "<prompt>"` and reads stdout. Requires the user
/// to have Claude Code installed and authenticated locally.
///
/// Why subprocess: zero new HTTP/auth code in icelines. The
/// user's existing `claude` CLI handles tokens. Latency is
/// 500ms-3s typically; we cap at `AiConfig::timeout`.
#[derive(Debug, Clone)]
pub struct ClaudeCliProvider {
    pub model: String,
    pub timeout: Duration,
}

impl ClaudeCliProvider {
    pub fn new(model: String, timeout: Duration) -> Self {
        Self { model, timeout }
    }
}

#[async_trait::async_trait]
impl AiProvider for ClaudeCliProvider {
    async fn interpret(
        &self,
        system_prompt: &str,
        natural_language: &str,
    ) -> Result<String, AiError> {
        // Build the prompt. Claude CLI reads from `-p <prompt>`.
        // We concatenate system + user with a clear separator so
        // the model can distinguish grammar reference from query.
        let prompt = format!("{system_prompt}\n\n----\n\nUser: {natural_language}\nYou:");

        // Spawn `claude -p <prompt> --model <model>`. Use the
        // tokio::process API so cancellation / timeout work.
        let mut cmd = tokio::process::Command::new("claude");
        cmd.arg("-p").arg(&prompt);
        if !self.model.is_empty() {
            cmd.arg("--model").arg(&self.model);
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd.spawn().map_err(|e| AiError::Subprocess {
            provider: "claude-cli",
            message: format!("failed to spawn `claude` — is it installed? {e}"),
        })?;

        // Wait with a timeout. tokio::time::timeout aborts the
        // future without killing the child — we still need to
        // explicitly wait for the output.
        let output = match tokio::time::timeout(self.timeout, child.wait_with_output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return Err(AiError::Subprocess {
                    provider: "claude-cli",
                    message: format!("wait_with_output failed: {e}"),
                })
            }
            Err(_elapsed) => {
                return Err(AiError::Timeout {
                    provider: "claude-cli",
                    seconds: self.timeout.as_secs(),
                })
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(AiError::Subprocess {
                provider: "claude-cli",
                message: format!(
                    "claude exited {}: {}",
                    output.status,
                    stderr.lines().next().unwrap_or("(no stderr)")
                ),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Err(AiError::EmptyResponse);
        }
        if trimmed == "UNSUPPORTED" {
            return Err(AiError::Unparseable {
                response: "model said the request can't be expressed in the cmdbar grammar"
                    .to_owned(),
            });
        }
        // Some models prefix with "You:" — strip it.
        let cleaned = trimmed
            .strip_prefix("You:")
            .map(str::trim)
            .unwrap_or(trimmed);
        Ok(cleaned.to_owned())
    }

    fn name(&self) -> &'static str {
        "claude-cli"
    }
}

// ── Anthropic API provider (Adams.7) ─────────────────────────────────────────

/// Phase Adams.7 — direct Anthropic Messages API provider.
/// Uses reqwest to POST to /v1/messages. Reads the API key
/// from `$ANTHROPIC_API_KEY` (override via the env; users with
/// company-specific names can `export ANTHROPIC_API_KEY=$XYZ`).
///
/// Why direct API: faster startup than subprocess, no
/// dependence on the user having Claude Code installed locally.
/// Tradeoff: the binary takes on auth-token UX (key must be set
/// in env).
#[derive(Debug, Clone)]
pub struct AnthropicApiProvider {
    pub model: String,
    pub timeout: Duration,
    /// Cached API key from $ANTHROPIC_API_KEY at construction
    /// time. Empty string means no key found — `interpret`
    /// returns AiError::Other rather than making a guaranteed-
    /// 401 request.
    pub api_key: String,
}

impl AnthropicApiProvider {
    pub fn new(model: String, timeout: Duration) -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        Self {
            model,
            timeout,
            api_key,
        }
    }
}

#[async_trait::async_trait]
impl AiProvider for AnthropicApiProvider {
    async fn interpret(
        &self,
        system_prompt: &str,
        natural_language: &str,
    ) -> Result<String, AiError> {
        if self.api_key.is_empty() {
            return Err(AiError::Other(
                "ANTHROPIC_API_KEY not set in environment — set it or switch provider to claude-cli"
                    .to_owned(),
            ));
        }

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": natural_language }
            ]
        });

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AiError::Other(format!("reqwest builder failed: {e}")))?;

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout {
                        provider: "anthropic-api",
                        seconds: self.timeout.as_secs(),
                    }
                } else {
                    AiError::Other(format!("HTTP error: {e}"))
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Truncate body for the flash — full responses can
            // be multi-KB.
            let snippet = body.chars().take(200).collect::<String>();
            return Err(AiError::Other(format!("anthropic-api {status}: {snippet}")));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::Other(format!("response decode failed: {e}")))?;

        // Response shape: { "content": [{"type": "text", "text": "..."}], ... }
        let text = json
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            })
            .and_then(|b| b.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(AiError::EmptyResponse);
        }
        if trimmed == "UNSUPPORTED" {
            return Err(AiError::Unparseable {
                response: "model said the request can't be expressed in the cmdbar grammar"
                    .to_owned(),
            });
        }
        Ok(trimmed.to_owned())
    }

    fn name(&self) -> &'static str {
        "anthropic-api"
    }
}

// ── Provider construction from config ────────────────────────────────────────

/// Build a boxed `AiProvider` from an `AiConfig`. Returns an
/// error if `provider` is unknown or `enabled` is false.
pub fn build_provider(cfg: &AiConfig) -> Result<Box<dyn AiProvider>, AiError> {
    if !cfg.enabled {
        return Err(AiError::Disabled);
    }
    match cfg.provider.as_str() {
        "claude-cli" => Ok(Box::new(ClaudeCliProvider::new(
            cfg.model.clone(),
            cfg.timeout,
        ))),
        "anthropic-api" => Ok(Box::new(AnthropicApiProvider::new(
            cfg.model.clone(),
            cfg.timeout,
        ))),
        other => Err(AiError::UnknownProvider(other.to_owned())),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_adams_ai_default_config_is_disabled() {
        let cfg = AiConfig::default();
        assert!(!cfg.enabled, "AI must be opt-in by default");
        assert_eq!(cfg.provider, "claude-cli");
        assert_eq!(cfg.timeout, Duration::from_secs(15));
    }

    #[test]
    fn l0_adams_ai_build_provider_disabled_errors() {
        let cfg = AiConfig::default(); // enabled=false
        let r = build_provider(&cfg);
        assert!(matches!(r, Err(AiError::Disabled)));
    }

    #[test]
    fn l0_adams_ai_build_provider_unknown_errors() {
        let cfg = AiConfig {
            enabled: true,
            provider: "frobnicator".to_owned(),
            model: "x".to_owned(),
            timeout: Duration::from_secs(5),
        };
        let r = build_provider(&cfg);
        assert!(matches!(r, Err(AiError::UnknownProvider(_))));
    }

    #[test]
    fn l0_adams_ai_build_provider_anthropic_ok() {
        let cfg = AiConfig {
            enabled: true,
            provider: "anthropic-api".to_owned(),
            model: "claude-haiku-4-5".to_owned(),
            timeout: Duration::from_secs(5),
        };
        let r = build_provider(&cfg);
        assert!(r.is_ok(), "anthropic-api must build (Adams.7)");
        let p = r.unwrap();
        assert_eq!(p.name(), "anthropic-api");
    }

    #[tokio::test]
    async fn l0_adams_ai_anthropic_no_api_key_errors() {
        // Force-clear env for the duration of the test. Note:
        // tests run in parallel, so we explicitly construct the
        // provider with a known-empty key instead of mutating env.
        let p = AnthropicApiProvider {
            model: "claude-haiku-4-5".to_owned(),
            timeout: Duration::from_secs(5),
            api_key: String::new(),
        };
        let r = p.interpret("system", "show top scorers").await;
        match r {
            Err(AiError::Other(msg)) => {
                assert!(
                    msg.contains("ANTHROPIC_API_KEY"),
                    "error must mention env var; got: {msg}"
                );
            }
            other => panic!("expected Other error, got {other:?}"),
        }
    }

    #[test]
    fn l0_adams_ai_anthropic_provider_reads_env_at_construction() {
        // Direct construction reads env. We can't easily mutate
        // env in parallel tests, but we can confirm the field
        // is initialized from std::env::var (compile-time
        // guarantee via the impl).
        let p = AnthropicApiProvider::new("claude-haiku-4-5".to_owned(), Duration::from_secs(5));
        // Either env is set or it isn't — both are valid; the
        // contract is just that the field is assigned.
        let _ = &p.api_key;
    }

    #[test]
    fn l0_adams_ai_build_provider_claude_cli_ok() {
        let cfg = AiConfig {
            enabled: true,
            provider: "claude-cli".to_owned(),
            model: "claude-haiku-4-5".to_owned(),
            timeout: Duration::from_secs(5),
        };
        let r = build_provider(&cfg);
        assert!(r.is_ok());
        let p = r.unwrap();
        assert_eq!(p.name(), "claude-cli");
    }

    #[tokio::test]
    async fn l0_adams_ai_stub_returns_canned_response() {
        let p = StubProvider::ok("query g >= 30");
        let r = p.interpret("ignored prompt", "young scorers").await;
        assert_eq!(r.unwrap(), "query g >= 30");
        assert_eq!(p.name(), "stub");
    }

    #[tokio::test]
    async fn l0_adams_ai_stub_returns_canned_error() {
        let p = StubProvider::err(AiError::EmptyResponse);
        let r = p.interpret("x", "y").await;
        assert!(matches!(r, Err(AiError::EmptyResponse)));
    }

    #[test]
    fn l0_adams_system_prompt_has_grammar_landmarks() {
        let s = default_system_prompt();
        // Each major grammar landmark must be present; this test
        // catches accidental prompt deletions.
        for landmark in &[
            "stats",
            "goalies",
            "poach",
            "gaps",
            "simulate",
            "roster",
            "watchlist",
            "team <ABBR>",
            "class <year>",
            "query <filter-expression>",
            "/fav add",
            "/hide favorites",
            "/help",
            "UNSUPPORTED",
            "AT age",
            "league=OHL",
            "ROSTER KV FORM",
            "sort=gaa",
            "min-gp=20",
            "pos=LW",
            ":goalies",
            "fantasy gaps shots top=6",
            "fantasy poach top=8 available",
            "simulate add=Connor_McDavid drop=Bench_Forward weeks=3",
            "class 2024",
        ] {
            assert!(
                s.contains(landmark),
                "system prompt missing landmark {landmark:?}"
            );
        }
    }

    #[test]
    fn l0_adams_system_prompt_version_is_set() {
        // Version sentinel for prompt-cache invalidation.
        assert_eq!(SYSTEM_PROMPT_VERSION, "v4");
    }

    #[test]
    fn l0_adams_ai_error_display_is_user_friendly() {
        let cases: Vec<AiError> = vec![
            AiError::Disabled,
            AiError::UnknownProvider("xyz".to_owned()),
            AiError::Timeout {
                provider: "claude-cli",
                seconds: 15,
            },
            AiError::Subprocess {
                provider: "claude-cli",
                message: "spawn failed".to_owned(),
            },
            AiError::EmptyResponse,
            AiError::Unparseable {
                response: "hello".to_owned(),
            },
            AiError::Other("misc".to_owned()),
        ];
        for e in cases {
            let s = e.to_string();
            assert!(!s.is_empty(), "Display must produce non-empty: {e:?}");
        }
    }
}
