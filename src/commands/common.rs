use crate::{
    RADYKO_CONCURRENCY,
    app::{
        config::RadykoConfig, program_resolver::resolve_selector,
        program_selector::ProgramSelector, state::AppState,
    },
    model::Program,
};
use futures::{StreamExt, TryStreamExt};
use std::{cmp::Reverse, sync::Arc};
use tracing::{info, warn};

pub fn collect_program_selectors(config: &RadykoConfig) -> anyhow::Result<Vec<ProgramSelector>> {
    let mut selectors = Vec::new();
    if config.keywords.is_none() && config.rules.is_none() {
        warn!("keywords and rules config is empty");
        return Ok(selectors);
    }

    match config.keywords.clone() {
        Some(keywords) => selectors.extend(ProgramSelector::from_keywords(keywords)),
        None => info!("keywords not found."),
    }
    match config.rules.clone() {
        Some(rules) => selectors.extend(ProgramSelector::from_rules(rules)?),
        None => info!("rules not found."),
    }
    Ok(selectors)
}

pub async fn resolve_programs(
    app_state: Arc<AppState>,
    program_selectors: Vec<ProgramSelector>,
) -> anyhow::Result<Vec<Program>> {
    let mut programs = futures::stream::iter(program_selectors)
        .map(|selector| {
            let app_state = Arc::clone(&app_state);
            async move { resolve_selector(&app_state.radiko_client, selector).await }
        })
        .buffer_unordered(RADYKO_CONCURRENCY)
        .try_fold(Vec::new(), |mut result, programs| async move {
            result.extend(programs);
            Ok(result)
        })
        .await?;
    programs.sort_by_key(|p| Reverse(p.start_time));

    Ok(programs)
}
