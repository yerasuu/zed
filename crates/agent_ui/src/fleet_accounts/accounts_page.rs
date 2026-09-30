use std::{path::PathBuf, sync::Arc, time::Duration};

use agent_servers::CLAUDE_AGENT_ID;
use chrono::{DateTime, Local, Utc};
use fs::Fs;
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, SharedString, Subscription, Task,
    Window,
};
use project::{
    AgentId,
    agent_server_store::{
        AllAgentServersSettings, CustomAgentServerSettings, agent_accounts::split_account,
    },
};
use serde::Deserialize;
use settings::{Settings as _, SettingsStore};
use ui::{Chip, Indicator, ProgressBar, prelude::*};
use workspace::{
    Workspace,
    item::{Item, ItemEvent},
};

use super::AddClaudeAccount;
use crate::{
    Agent, AgentPanel, NewExternalAgentThread,
    agent_connection_store::{AgentConnectionStatus, AgentConnectionStore},
};

/// Claude Code rewrites its usage cache in `.claude.json` while it runs, so the page rereads
/// the account files on this interval instead of only on settings changes.
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

pub struct AgentAccountsPage {
    fs: Arc<dyn Fs>,
    connection_store: Option<Entity<AgentConnectionStore>>,
    accounts: Vec<Account>,
    focus_handle: FocusHandle,
    _refresh_task: Task<()>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone)]
struct AccountSource {
    agent_id: AgentId,
    /// `None` for the default `claude-acp` entry.
    name: Option<SharedString>,
    config_dir: PathBuf,
    claude_json_path: PathBuf,
}

struct Account {
    source: AccountSource,
    details: AccountDetails,
}

#[derive(Default)]
struct AccountDetails {
    logged_in: bool,
    email: Option<String>,
    organization: Option<String>,
    role: Option<String>,
    subscription: Option<String>,
    usage: Option<Usage>,
}

struct Usage {
    fetched_at: DateTime<Utc>,
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
}

// Only non-secret fields are declared, so tokens in these files are never deserialized.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeConfigFile {
    oauth_account: Option<OauthAccount>,
    cached_usage_utilization: Option<CachedUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthAccount {
    account_uuid: Option<String>,
    email_address: Option<String>,
    organization_name: Option<String>,
    organization_role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedUsage {
    fetched_at_ms: Option<i64>,
    account_uuid: Option<String>,
    utilization: Option<Utilization>,
}

#[derive(Deserialize)]
struct Utilization {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
}

#[derive(Deserialize)]
struct UsageWindow {
    utilization: Option<f32>,
    resets_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialsFile {
    claude_ai_oauth: Option<OauthCredentials>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthCredentials {
    subscription_type: Option<String>,
}

impl AgentAccountsPage {
    pub fn new(
        workspace: &Workspace,
        fs: Arc<dyn Fs>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        let connection_store = workspace
            .panel::<AgentPanel>(cx)
            .map(|panel| panel.read(cx).connection_store().clone());

        cx.new(|cx: &mut Context<Self>| {
            let mut subscriptions = vec![cx.observe_global::<SettingsStore>(|this, cx| {
                this.refresh(cx);
            })];
            if let Some(connection_store) = &connection_store {
                subscriptions.push(cx.observe(connection_store, |_, _, cx| cx.notify()));
            }

            let mut this = Self {
                fs,
                connection_store,
                accounts: Vec::new(),
                focus_handle: cx.focus_handle(),
                _refresh_task: Task::ready(()),
                _subscriptions: subscriptions,
            };
            this.refresh(cx);
            window.focus(&this.focus_handle, cx);
            this
        })
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let sources = configured_accounts(cx);
        let fs = self.fs.clone();
        self._refresh_task = cx.spawn(async move |this, cx| {
            loop {
                let mut accounts = Vec::with_capacity(sources.len());
                for source in &sources {
                    accounts.push(Account {
                        details: load_details(fs.as_ref(), source).await,
                        source: source.clone(),
                    });
                }
                let updated = this.update(cx, |this, cx| {
                    this.accounts = accounts;
                    cx.notify();
                });
                if updated.is_err() {
                    return;
                }
                cx.background_executor().timer(REFRESH_INTERVAL).await;
            }
        });
    }

    fn connection_status(&self, agent_id: &AgentId, cx: &App) -> AgentConnectionStatus {
        self.connection_store
            .as_ref()
            .map_or(AgentConnectionStatus::Disconnected, |store| {
                store.read(cx).connection_status(
                    &Agent::Custom {
                        id: agent_id.clone(),
                    },
                    cx,
                )
            })
    }

    fn render_account(
        &self,
        index: usize,
        account: &Account,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let details = &account.details;
        let title: SharedString = match &account.source.name {
            Some(name) => format!("Claude Agent ({name})").into(),
            None => "Claude Agent".into(),
        };
        let (status_color, status_label) =
            match self.connection_status(&account.source.agent_id, cx) {
                AgentConnectionStatus::Connected => (Color::Success, "Running"),
                AgentConnectionStatus::Connecting => (Color::Warning, "Starting"),
                AgentConnectionStatus::Disconnected => (Color::Muted, "Not running"),
            };

        let identity = [
            details.email.as_deref(),
            details.organization.as_deref(),
            details.role.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");

        let agent_id = account.source.agent_id.clone();
        let new_thread_button = Button::new(("account-new-thread", index), "New Thread")
            .style(ButtonStyle::Outlined)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focus_handle.dispatch_action(
                    &NewExternalAgentThread {
                        agent: agent_id.clone(),
                    },
                    window,
                    cx,
                );
            }));

        v_flex()
            .p_3()
            .gap_2()
            .bg(cx.theme().colors().elevated_surface_background.opacity(0.5))
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .rounded_md()
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Indicator::dot().color(status_color))
                            .child(Label::new(title))
                            .child(
                                Label::new(status_label)
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            )
                            .when_some(details.subscription.clone(), |this, subscription| {
                                this.child(Chip::new(subscription))
                            }),
                    )
                    .child(new_thread_button),
            )
            .child(if details.logged_in {
                Label::new(if identity.is_empty() {
                    "Logged in".to_string()
                } else {
                    identity
                })
                .into_any_element()
            } else {
                Label::new("Not logged in. Start a thread to log in.")
                    .color(Color::Warning)
                    .into_any_element()
            })
            .child(
                Label::new(account.source.config_dir.display().to_string())
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .when_some(details.usage.as_ref(), |this, usage| {
                this.child(render_usage_window(
                    ("five-hour", index),
                    "5-hour",
                    usage.five_hour.as_ref(),
                    cx,
                ))
                .child(render_usage_window(
                    ("seven-day", index),
                    "7-day",
                    usage.seven_day.as_ref(),
                    cx,
                ))
                .child(
                    Label::new(format!(
                        "Usage as of {}, from Claude Code's cache",
                        format_ago(usage.fetched_at)
                    ))
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
                )
            })
    }
}

fn render_usage_window(
    id: (&'static str, usize),
    label: &'static str,
    window: Option<&UsageWindow>,
    cx: &App,
) -> impl IntoElement {
    let utilization = window.and_then(|window| window.utilization);
    let summary = match (utilization, window.and_then(|window| window.resets_at)) {
        (Some(utilization), Some(resets_at)) => format!(
            "{utilization:.0}% · resets {}",
            resets_at.with_timezone(&Local).format("%a %H:%M")
        ),
        (Some(utilization), None) => format!("{utilization:.0}%"),
        (None, _) => "No data".to_string(),
    };

    h_flex()
        .gap_2()
        .child(div().w_16().child(Label::new(label).size(LabelSize::Small)))
        .child(
            div()
                .w_48()
                .child(ProgressBar::new(id, utilization.unwrap_or(0.), 100., cx)),
        )
        .child(
            Label::new(summary)
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
}

fn format_ago(time: DateTime<Utc>) -> String {
    let minutes = (Utc::now() - time).num_minutes();
    match minutes {
        ..1 => "just now".to_string(),
        1..60 => format!("{minutes} min ago"),
        60..1440 => format!("{} h ago", minutes / 60),
        _ => format!("{} days ago", minutes / 1440),
    }
}

/// The default `claude-acp` entry followed by every `claude-acp@<name>` account.
fn configured_accounts(cx: &App) -> Vec<AccountSource> {
    let settings = AllAgentServersSettings::get_global(cx);
    let home_dir = util::paths::home_dir();
    let config_dir_override = |agent_id: &str| match settings.get(agent_id) {
        Some(CustomAgentServerSettings::Registry { env, .. }) => {
            env.get("CLAUDE_CONFIG_DIR").map(PathBuf::from)
        }
        Some(CustomAgentServerSettings::Custom { command, .. }) => command
            .env
            .as_ref()
            .and_then(|env| env.get("CLAUDE_CONFIG_DIR"))
            .map(PathBuf::from),
        None => None,
    };
    let source = |agent_id: &str, name: Option<&str>| {
        // Without CLAUDE_CONFIG_DIR, Claude Code keeps `.claude.json` in the home dir rather
        // than inside `~/.claude`.
        let (config_dir, claude_json_path) = match config_dir_override(agent_id) {
            Some(config_dir) => {
                let claude_json_path = config_dir.join(".claude.json");
                (config_dir, claude_json_path)
            }
            None => (home_dir.join(".claude"), home_dir.join(".claude.json")),
        };
        AccountSource {
            agent_id: AgentId::new(agent_id.to_string()),
            name: name.map(|name| SharedString::from(name.to_string())),
            config_dir,
            claude_json_path,
        }
    };

    let mut accounts = settings
        .keys()
        .filter_map(|agent_id| {
            let (agent, name) = split_account(agent_id)?;
            (agent == CLAUDE_AGENT_ID).then(|| source(agent_id, Some(name)))
        })
        .collect::<Vec<_>>();
    accounts.sort_by(|a, b| a.name.cmp(&b.name));
    accounts.insert(0, source(CLAUDE_AGENT_ID, None));
    accounts
}

async fn load_details(fs: &dyn Fs, source: &AccountSource) -> AccountDetails {
    let credentials =
        read_json::<CredentialsFile>(fs, &source.config_dir.join(".credentials.json")).await;
    let config = read_json::<ClaudeConfigFile>(fs, &source.claude_json_path).await;
    account_details(credentials, config)
}

fn account_details(
    credentials: Option<CredentialsFile>,
    config: Option<ClaudeConfigFile>,
) -> AccountDetails {
    let Some(credentials) = credentials.and_then(|file| file.claude_ai_oauth) else {
        return AccountDetails::default();
    };
    let (oauth_account, cached_usage) = match config {
        Some(config) => (config.oauth_account, config.cached_usage_utilization),
        None => (None, None),
    };
    let account_uuid = oauth_account
        .as_ref()
        .and_then(|account| account.account_uuid.clone());
    // The cache survives a re-login with a different account, so only trust it when it
    // belongs to the account that is logged in now.
    let usage = cached_usage
        .filter(|usage| usage.account_uuid == account_uuid)
        .and_then(|usage| {
            let utilization = usage.utilization?;
            Some(Usage {
                fetched_at: DateTime::from_timestamp_millis(usage.fetched_at_ms?)?,
                five_hour: utilization.five_hour,
                seven_day: utilization.seven_day,
            })
        });

    AccountDetails {
        logged_in: true,
        email: oauth_account
            .as_ref()
            .and_then(|account| account.email_address.clone()),
        organization: oauth_account
            .as_ref()
            .and_then(|account| account.organization_name.clone()),
        role: oauth_account.and_then(|account| account.organization_role),
        subscription: credentials.subscription_type,
        usage,
    }
}

async fn read_json<T: for<'de> Deserialize<'de>>(fs: &dyn Fs, path: &std::path::Path) -> Option<T> {
    let text = fs.load(path).await.ok()?;
    serde_json::from_str(&text)
        .inspect_err(|error| log::warn!("failed to parse {}: {error}", path.display()))
        .ok()
}

impl Render for AgentAccountsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let accounts = self
            .accounts
            .iter()
            .enumerate()
            .map(|(index, account)| self.render_account(index, account, cx).into_any_element())
            .collect::<Vec<_>>();

        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .p_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().colors().border_variant)
                    .child(Headline::new("Agent Accounts").size(HeadlineSize::Large))
                    .child(
                        Button::new("add-claude-account", "Add Claude Account")
                            .style(ButtonStyle::Outlined)
                            .start_icon(Icon::new(IconName::Plus).size(IconSize::Small))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.focus_handle
                                    .dispatch_action(&AddClaudeAccount, window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .id("agent-accounts")
                    .p_4()
                    .gap_3()
                    .size_full()
                    .overflow_y_scroll()
                    .children(accounts),
            )
    }
}

impl EventEmitter<ItemEvent> for AgentAccountsPage {}

impl Focusable for AgentAccountsPage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for AgentAccountsPage {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "Agent Accounts".into()
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed copies of what Claude Code writes, including a secret the parser must ignore.
    const CREDENTIALS: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "secret",
            "refreshToken": "secret",
            "expiresAt": 1790782282828,
            "subscriptionType": "team",
            "rateLimitTier": "default_raven"
        }
    }"#;
    const CONFIG: &str = r#"{
        "numStartups": 3,
        "oauthAccount": {
            "accountUuid": "8f6c2bfb",
            "emailAddress": "me@example.com",
            "organizationName": "Example",
            "organizationRole": "primary_owner",
            "seatTier": "team_standard"
        },
        "cachedUsageUtilization": {
            "fetchedAtMs": 1790769217595,
            "accountUuid": "8f6c2bfb",
            "utilization": {
                "five_hour": { "utilization": 0, "resets_at": "2026-09-30T16:49:59.516115+00:00" },
                "seven_day": { "utilization": 8, "resets_at": "2026-10-06T23:59:59.516138+00:00" },
                "seven_day_opus": null
            }
        }
    }"#;

    fn details(credentials: Option<&str>, config: &str) -> AccountDetails {
        account_details(
            credentials.map(|text| serde_json::from_str(text).unwrap()),
            Some(serde_json::from_str(config).unwrap()),
        )
    }

    #[test]
    fn test_logged_in_account() {
        let details = details(Some(CREDENTIALS), CONFIG);
        assert!(details.logged_in);
        assert_eq!(details.email.as_deref(), Some("me@example.com"));
        assert_eq!(details.organization.as_deref(), Some("Example"));
        assert_eq!(details.role.as_deref(), Some("primary_owner"));
        assert_eq!(details.subscription.as_deref(), Some("team"));

        let usage = details.usage.unwrap();
        let seven_day = usage.seven_day.unwrap();
        assert_eq!(seven_day.utilization, Some(8.));
        assert_eq!(
            seven_day.resets_at.unwrap().to_rfc3339(),
            "2026-10-06T23:59:59.516138+00:00"
        );
    }

    #[test]
    fn test_account_without_credentials_is_logged_out() {
        let details = details(None, CONFIG);
        assert!(!details.logged_in);
        assert!(details.email.is_none());
        assert!(details.usage.is_none());
    }

    #[test]
    fn test_usage_cache_from_another_account_is_ignored() {
        let config = CONFIG.replacen(
            r#""accountUuid": "8f6c2bfb",
            "utilization""#,
            r#""accountUuid": "someone-else",
            "utilization""#,
            1,
        );
        let details = details(Some(CREDENTIALS), &config);
        assert!(details.logged_in);
        assert!(details.usage.is_none());
    }
}
