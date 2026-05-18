use crate::app::{config::RadykoConfig, program_selector::ProgramSelector};
use tracing::{info, warn};

// TODO: RadykoConfig構造体に実装できそう
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
