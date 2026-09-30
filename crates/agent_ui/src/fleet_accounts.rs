//! fleet: adds Claude Agent accounts from inside Zed. Each account is an `agent_servers` entry
//! `claude-acp@<name>` whose `CLAUDE_CONFIG_DIR` points at its own directory under Zed's config
//! dir (`agent_accounts/claude-acp/<name>`), so it keeps its own login while sharing settings
//! and conversation history with `~/.claude`. Kept in its own file so upstream merges don't
//! touch it; see `FLEET.md`.

use std::{path::PathBuf, sync::Arc, time::Duration};

use agent_servers::CLAUDE_AGENT_ID;
use anyhow::{Context as _, anyhow};
use collections::HashMap;
use fs::Fs;
use futures::{FutureExt as _, channel::oneshot};
use gpui::{App, Context, Window, actions};
use project::{
    AgentId,
    agent_server_store::{AgentServersUpdated, AllAgentServersSettings},
};
use settings::Settings as _;
use workspace::{Workspace, notifications::NotifyTaskExt as _};

use crate::NewExternalAgentThread;

mod accounts_page;

use accounts_page::AgentAccountsPage;

actions!(
    agent,
    [
        /// Adds another Claude Agent account. It keeps its own login and shares settings and
        /// conversation history with the default account.
        AddClaudeAccount,
        /// Shows every Claude Agent account with its login, plan and usage.
        OpenAgentAccounts
    ]
);

/// Account-independent entries of `~/.claude` linked into every account. `.credentials.json`
/// and `.claude.json` hold the account's login, so they are deliberately absent.
const SHARED_ITEMS: &[&str] = &[
    "settings.json",
    "keybindings.json",
    "CLAUDE.md",
    "skills",
    "commands",
    "agents",
    "projects",
    "history.jsonl",
];

/// Covers a cold start, where the ACP registry must be fetched before the account's agent can
/// be registered.
const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn init(fs: Arc<dyn Fs>, cx: &mut App) {
    cx.observe_new(move |workspace: &mut Workspace, _window, _cx| {
        let fs = fs.clone();
        workspace.register_action({
            let fs = fs.clone();
            move |workspace, _: &AddClaudeAccount, window, cx| {
                add_claude_account(workspace, fs.clone(), window, cx);
            }
        });
        workspace.register_action(move |workspace, _: &OpenAgentAccounts, window, cx| {
            open_accounts_page(workspace, fs.clone(), window, cx);
        });
    })
    .detach();
}

fn open_accounts_page(
    workspace: &mut Workspace,
    fs: Arc<dyn Fs>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let existing = workspace
        .active_pane()
        .read(cx)
        .items()
        .find_map(|item| item.downcast::<AgentAccountsPage>());
    if let Some(existing) = existing {
        existing.update(cx, |page, cx| page.refresh(cx));
        workspace.activate_item(&existing, true, true, window, cx);
    } else {
        let page = AgentAccountsPage::new(workspace, fs, window, cx);
        workspace.add_item_to_active_pane(Box::new(page), None, true, window, cx);
    }
}

fn add_claude_account(
    workspace: &mut Workspace,
    fs: Arc<dyn Fs>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let agent_server_store = workspace.project().read(cx).agent_server_store().clone();
    let configured_agents = AllAgentServersSettings::get_global(cx).clone();

    let task = cx.spawn_in(window, async move |workspace, cx| {
        let shared_dir = util::paths::home_dir().join(".claude");
        let accounts_dir = paths::config_dir()
            .join("agent_accounts")
            .join(CLAUDE_AGENT_ID);
        let (name, account_dir) =
            next_free_account(&configured_agents, &accounts_dir, fs.as_ref()).await;
        let agent_id = AgentId::new(format!("{CLAUDE_AGENT_ID}@{name}"));

        fs.create_dir(&account_dir)
            .await
            .with_context(|| format!("creating {}", account_dir.display()))?;
        for item in SHARED_ITEMS {
            let source = shared_dir.join(item);
            if fs.metadata(&source).await?.is_some() {
                fs.create_symlink(&account_dir.join(item), source)
                    .await
                    .with_context(|| format!("linking {item} into {}", account_dir.display()))?;
            }
        }

        // Subscribe before saving so the registration can't happen unobserved.
        let (registered_tx, registered_rx) = oneshot::channel();
        let _subscription = cx.update(|_, cx| {
            let agent_id = agent_id.clone();
            let mut registered_tx = Some(registered_tx);
            cx.subscribe(
                &agent_server_store,
                move |store, _: &AgentServersUpdated, cx| {
                    if store.read(cx).external_agents().any(|id| id == &agent_id)
                        && let Some(registered_tx) = registered_tx.take()
                    {
                        registered_tx.send(()).ok();
                    }
                },
            )
        })?;

        let account_dir_value = account_dir.to_string_lossy().into_owned();
        let saved = cx.update(|_, cx| {
            let agent_id = agent_id.to_string();
            settings::update_settings_file_with_completion(fs.clone(), cx, move |settings, _| {
                settings.agent_servers.get_or_insert_default().0.insert(
                    agent_id,
                    settings::CustomAgentServerSettings::Registry {
                        env: HashMap::from_iter([(
                            "CLAUDE_CONFIG_DIR".to_string(),
                            account_dir_value,
                        )]),
                        default_mode: None,
                        default_config_options: HashMap::default(),
                        favorite_config_option_values: HashMap::default(),
                    },
                );
            })
        })?;
        saved.await.context("saving the account to settings")??;

        let timeout = cx.background_executor().timer(REGISTRATION_TIMEOUT);
        futures::select_biased! {
            registered = registered_rx.fuse() => registered.context("agent registration")?,
            _ = timeout.fuse() => {
                return Err(anyhow!(
                    "Account `{name}` was saved, but Claude Agent could not be loaded from the \
                     ACP registry yet. It will appear in the new thread menu once it loads."
                ));
            }
        }

        workspace.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(NewExternalAgentThread { agent: agent_id }), cx);
        })?;
        anyhow::Ok(())
    });
    task.detach_and_notify_err(cx.weak_entity(), window, cx);
}

/// Accounts are numbered from 2, since the default `claude-acp` entry is the first account.
async fn next_free_account(
    configured_agents: &AllAgentServersSettings,
    accounts_dir: &std::path::Path,
    fs: &dyn Fs,
) -> (String, PathBuf) {
    let mut number = 2;
    loop {
        let name = format!("account-{number}");
        let account_dir = accounts_dir.join(&name);
        let configured = configured_agents.contains_key(&format!("{CLAUDE_AGENT_ID}@{name}"));
        if !configured && !fs.is_dir(&account_dir).await {
            return (name, account_dir);
        }
        number += 1;
    }
}
