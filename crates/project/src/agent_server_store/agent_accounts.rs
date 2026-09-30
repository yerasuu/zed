//! fleet: agent accounts. An `agent_servers` entry keyed `<agent>@<account>` runs registry
//! agent `<agent>` as a separate agent, so several accounts of the same agent can be used at
//! once. Kept in its own file so upstream merges don't touch it; see `FLEET.md`.

use collections::HashMap;
use gpui::SharedString;

/// Splits `<agent>@<account>` into its parts. Uses the last `@` so that the account name is
/// never mistaken for part of the agent id.
pub fn split_account(agent_id: &str) -> Option<(&str, &str)> {
    agent_id
        .rsplit_once('@')
        .filter(|(agent, account)| !agent.is_empty() && !account.is_empty())
}

/// The id used to pick agent-specific behavior (auth env, API keys) for `agent_id`, which is
/// the base agent for account entries.
pub fn base_agent_id(agent_id: &str) -> &str {
    split_account(agent_id).map_or(agent_id, |(agent, _)| agent)
}

/// Finds the registry agent backing a settings entry. An exact id match wins, so a registry
/// id that happens to contain `@` is never treated as an account.
pub fn lookup_registry_agent<'a, T>(
    registry_agents: &'a HashMap<String, T>,
    agent_id: &'a str,
) -> Option<(&'a T, Option<&'a str>)> {
    if let Some(agent) = registry_agents.get(agent_id) {
        return Some((agent, None));
    }
    let (agent, account) = split_account(agent_id)?;
    Some((registry_agents.get(agent)?, Some(account)))
}

pub fn display_name(agent_name: &SharedString, account: Option<&str>) -> SharedString {
    match account {
        Some(account) => format!("{agent_name} ({account})").into(),
        None => agent_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_account() {
        assert_eq!(
            split_account("claude-acp@work"),
            Some(("claude-acp", "work"))
        );
        assert_eq!(split_account("a@b@work"), Some(("a@b", "work")));
        assert_eq!(split_account("claude-acp"), None);
        assert_eq!(split_account("@work"), None);
        assert_eq!(split_account("claude-acp@"), None);
    }

    #[test]
    fn test_lookup_registry_agent() {
        let registry =
            HashMap::from_iter([("claude-acp".to_string(), 1), ("odd@id".to_string(), 2)]);

        assert_eq!(
            lookup_registry_agent(&registry, "claude-acp"),
            Some((&1, None))
        );
        assert_eq!(
            lookup_registry_agent(&registry, "claude-acp@work"),
            Some((&1, Some("work")))
        );
        assert_eq!(lookup_registry_agent(&registry, "odd@id"), Some((&2, None)));
        assert_eq!(lookup_registry_agent(&registry, "missing@work"), None);
    }

    #[test]
    fn test_display_name() {
        let name = SharedString::from("Claude Agent");
        assert_eq!(
            display_name(&name, Some("work")).as_ref(),
            "Claude Agent (work)"
        );
        assert_eq!(display_name(&name, None).as_ref(), "Claude Agent");
    }
}
