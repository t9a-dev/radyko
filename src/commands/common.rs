use crate::{
    app::{
        config::RadykoConfig, program_resolver::resolve_selector,
        program_selector::ProgramSelector, state::AppState,
    },
    model::Program,
};
use std::{cmp::Reverse, sync::Arc};
use tokio_stream::StreamExt;
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
    let mut resolve_program_handles = tokio_stream::iter(
        program_selectors
            .into_iter()
            .map(|selector| {
                let app_state = Arc::clone(&app_state);
                tokio::spawn(async move {
                    resolve_selector(&app_state.radiko_client, selector)
                        .await
                        .unwrap()
                })
            })
            .collect::<Vec<_>>(),
    );
    let mut programs = Vec::new();
    // join_allで複数の非同期処理の完了を待つと遅くなるのでtokio_streamを利用している
    // https://github.com/tokio-rs/tokio/issues/2401
    while let Some(resolved_programs) = resolve_program_handles.next().await {
        programs.extend(resolved_programs.await.unwrap());
    }
    programs.sort_by_key(|p| Reverse(p.start_time));
    Ok(programs)
}
