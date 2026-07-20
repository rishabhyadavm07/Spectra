use crate::dto::{EnvironmentOutput, OrphanedSecret, VariableInput, VariableOutput};
use spectra_core::model::{new_id, Environment, VariableValue};
use spectra_core::secrets::keychain_account;
use spectra_core::{AppContext, ApiResult};
use std::collections::HashMap;

/// Sentinel value the frontend sends for a secret variable's `value` field
/// when the user didn't change it (since the real value is never sent back
/// to the frontend to be echoed). Any other value for a `secret: true`
/// variable is treated as a real update.
pub const UNCHANGED_SECRET_SENTINEL: &str = "••••••••";

pub async fn list_environments(ctx: &AppContext, workspace_id: String) -> ApiResult<Vec<EnvironmentOutput>> {
    let envs = ctx.storage.list_environments(&workspace_id).await?;
    Ok(envs.iter().map(to_output).collect())
}

/// Scans every environment in a workspace for secret variables whose
/// Keychain entry can't be found, and reports each one. This is the
/// detectable symptom of copying/restoring `~/.spectra` onto a different
/// machine (or a different user account) than the one that created the
/// secrets: the JSON round-trips fine, but the Keychain entries it
/// references never travel with it, so a secret that "exists" in the
/// environment file silently resolves to an empty string at send time
/// without this check. Read-only — never creates/deletes Keychain entries.
pub async fn check_secrets_health(ctx: &AppContext, workspace_id: String) -> ApiResult<Vec<OrphanedSecret>> {
    let mut orphaned = Vec::new();
    for env in ctx.storage.list_environments(&workspace_id).await? {
        for (name, value) in &env.variables {
            if let VariableValue::Secret { keychain_account } = value {
                if ctx.secrets.get(keychain_account)?.is_none() {
                    orphaned.push(OrphanedSecret {
                        environment_id: env.id.clone(),
                        environment_name: env.name.clone(),
                        variable_name: name.clone(),
                    });
                }
            }
        }
    }
    Ok(orphaned)
}

fn to_output(env: &Environment) -> EnvironmentOutput {
    EnvironmentOutput {
        id: env.id.clone(),
        workspace_id: env.workspace_id.clone(),
        name: env.name.clone(),
        variables: to_output_variables(&env.variables),
    }
}

/// Converts internal variables (which may hold Keychain references for
/// secrets) into the masked DTO shape safe to send to the frontend.
pub fn to_output_variables(variables: &HashMap<String, VariableValue>) -> HashMap<String, VariableOutput> {
    variables
        .iter()
        .map(|(k, v)| {
            let out = match v {
                VariableValue::Plain { value } => VariableOutput { value: value.clone(), secret: false },
                VariableValue::Secret { .. } => {
                    VariableOutput { value: UNCHANGED_SECRET_SENTINEL.to_string(), secret: true }
                }
            };
            (k.clone(), out)
        })
        .collect()
}

/// Converts frontend input variables into internal storage, writing any new
/// secret plaintext to Keychain. `existing` is the previous variable set (if
/// any) so an unchanged secret (still showing the sentinel) keeps its
/// existing Keychain account rather than being treated as a fresh secret.
fn to_internal_variables(
    ctx: &AppContext,
    workspace_id: &str,
    environment_id: &str,
    existing: Option<&HashMap<String, VariableValue>>,
    inputs: HashMap<String, VariableInput>,
) -> ApiResult<HashMap<String, VariableValue>> {
    let mut out = HashMap::with_capacity(inputs.len());
    for (name, input) in inputs {
        if input.secret {
            let account = keychain_account(workspace_id, environment_id, &name);
            if input.value != UNCHANGED_SECRET_SENTINEL {
                ctx.secrets.set(&account, &input.value)?;
            } else if !matches!(existing.and_then(|e| e.get(&name)), Some(VariableValue::Secret { .. })) {
                // Sentinel sent but there's no existing secret under this
                // name (e.g. a plain var was flipped to secret without
                // typing a new value) — nothing to persist to Keychain yet.
            }
            out.insert(name, VariableValue::Secret { keychain_account: account });
        } else {
            out.insert(name, VariableValue::Plain { value: input.value });
        }
    }
    Ok(out)
}

/// Deletes Keychain entries for any secret variables that existed before but
/// are absent from (or no longer secret in) the new variable set.
fn cleanup_removed_secrets(
    ctx: &AppContext,
    previous: &HashMap<String, VariableValue>,
    next: &HashMap<String, VariableValue>,
) -> ApiResult<()> {
    for (name, value) in previous {
        if let VariableValue::Secret { keychain_account } = value {
            let still_secret = matches!(next.get(name), Some(VariableValue::Secret { .. }));
            if !still_secret {
                ctx.secrets.delete(keychain_account)?;
            }
        }
    }
    Ok(())
}

pub async fn create_environment(
    ctx: &AppContext,
    workspace_id: String,
    name: String,
    variables: HashMap<String, VariableInput>,
) -> ApiResult<EnvironmentOutput> {
    let id = new_id();
    let variables = to_internal_variables(ctx, &workspace_id, &id, None, variables)?;
    let env = Environment { id, workspace_id, name, variables };
    ctx.storage.save_environment(&env).await?;
    Ok(to_output(&env))
}

pub async fn update_environment(
    ctx: &AppContext,
    workspace_id: String,
    id: String,
    name: String,
    variables: HashMap<String, VariableInput>,
) -> ApiResult<EnvironmentOutput> {
    let previous = ctx.storage.get_environment(&workspace_id, &id).await.ok();
    let previous_vars = previous.as_ref().map(|e| &e.variables);
    let variables = to_internal_variables(ctx, &workspace_id, &id, previous_vars, variables)?;
    if let Some(prev) = &previous {
        cleanup_removed_secrets(ctx, &prev.variables, &variables)?;
    }
    let env = Environment { id, workspace_id, name, variables };
    ctx.storage.save_environment(&env).await?;
    Ok(to_output(&env))
}

pub async fn delete_environment(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<()> {
    if let Ok(env) = ctx.storage.get_environment(&workspace_id, &id).await {
        for value in env.variables.values() {
            if let VariableValue::Secret { keychain_account } = value {
                ctx.secrets.delete(keychain_account)?;
            }
        }
    }
    ctx.storage.delete_environment(&workspace_id, &id).await?;
    let mut ws = ctx.storage.get_workspace(&workspace_id).await?;
    if ws.active_environment_id.as_deref() == Some(id.as_str()) {
        ws.active_environment_id = None;
        ctx.storage.save_workspace(&ws).await?;
    }
    Ok(())
}
