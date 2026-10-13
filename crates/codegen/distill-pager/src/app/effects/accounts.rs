//! Scoped account IO. Error messages never expose provider responses or credentials.
use crate::app::actions::AccountsTask;
use crate::views::accounts_modal::{AccountsData, AccountsRequest};
use distill_shell::accounts::{AccountStore, GrokLoginConfig, Provider};

pub(super) async fn run(
    task: &AccountsTask,
    feedback: tokio::sync::mpsc::UnboundedSender<distill_shell::accounts::AccountLoginFeedback>,
) -> Result<AccountsData, String> {
    let request = task.request.clone();
    let grok = tokio::task::spawn_blocking(|| {
        distill_shell::config::load_agent_config_disk_only()
            .ok()
            .map(|config| GrokLoginConfig::from_config(&config))
    })
    .await
    .map_err(|_| "Could not load account configuration.".to_owned())?;
    let store = AccountStore::default();
    match &request {
        AccountsRequest::Login(id) => {
            store
                .login_tui(id, grok.clone(), feedback)
                .await
                .map_err(|_| {
                    "Sign-in failed. Retry Login; your existing connection was preserved."
                        .to_owned()
                })?
        }
        AccountsRequest::Logout(id) => store
            .logout(id, grok.clone())
            .await
            .map_err(|_| "Could not disconnect this account. Retry Disconnect.".to_owned())?,
        AccountsRequest::Refresh(id) => {
            store
                .refresh_usage(id, grok.clone())
                .await
                .map_err(|_| "Could not refresh this account's quota. Retry Quota.".to_owned())?;
        }
        _ => {}
    }
    tokio::task::spawn_blocking(move || {
        match &request {
            AccountsRequest::Add {
                id,
                provider,
                label,
            } => {
                store
                    .add(id, *provider, label)
                    .map_err(|_| "Could not add account. Try a shorter label.".to_owned())?;
            }
            AccountsRequest::Rename { id, label } => {
                store
                    .rename(id, label)
                    .map_err(|_| "Could not rename account. Try a shorter label.".to_owned())?;
            }
            AccountsRequest::Remove(id) => store.remove(id).map_err(|_| {
                "Could not remove account. Remove its route options and Save first.".to_owned()
            })?,
            AccountsRequest::Save(routing) => store.save_routing(routing).map_err(|_| {
                "Could not save routing. Check each account and model, then retry Save.".to_owned()
            })?,
            _ => {}
        }
        let mut accounts = vec![];
        for account in store
            .list()
            .map_err(|_| "Could not load accounts. Press r to retry.".to_owned())?
        {
            let config = (account.provider == Provider::Grok)
                .then_some(grok.as_ref())
                .flatten();
            accounts.push(
                store
                    .status(&account.id, config)
                    .map_err(|_| "Could not read account status. Press r to retry.".to_owned())?,
            );
        }
        let routing = store
            .load_routing()
            .map_err(|_| "Could not load routing. Press r to retry.".to_owned())?;
        Ok(AccountsData {
            accounts,
            routing,
            catalog: None,
        })
    })
    .await
    .map_err(|_| "Account operation stopped unexpectedly. Retry the action.".to_owned())?
}
