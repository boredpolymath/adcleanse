use crate::error::Result;
use crate::scrubber::mutator::{PreferenceMutator, ScrubResult};
use crate::scrubber::rules::TopicRule;

#[tauri::command]
pub async fn scrub_topic(topic_id: String, topic_name: String) -> Result<ScrubResult> {
    log::info!("IPC: scrub_topic for {}", topic_name);
    let mutator = PreferenceMutator::new();
    mutator.remove_topic(&topic_id, &topic_name).await
}

#[tauri::command]
pub async fn batch_scrub_topics(topic_ids: Vec<String>) -> Result<Vec<ScrubResult>> {
    log::info!("IPC: batch_scrub_topics count: {}", topic_ids.len());
    let mutator = PreferenceMutator::new();
    let mut results = Vec::new();
    for id in topic_ids {
        results.push(mutator.remove_topic(&id, "Batch Topic").await?);
    }
    Ok(results)
}

#[tauri::command]
pub async fn opt_out_partner(partner_id: String, company_name: String) -> Result<ScrubResult> {
    log::info!("IPC: opt_out_partner for {}", company_name);
    let mutator = PreferenceMutator::new();
    mutator.opt_out_partner(&partner_id, &company_name).await
}

#[tauri::command]
pub async fn get_topic_rules() -> Result<Vec<TopicRule>> {
    Ok(vec![
        TopicRule {
            id: "rule_1".to_string(),
            pattern: "Gambling & Casinos".to_string(),
            is_regex: false,
            auto_scrub_enabled: true,
            created_at_epoch: chrono::Utc::now().timestamp() - 86400,
        },
        TopicRule {
            id: "rule_2".to_string(),
            pattern: "Weight Loss & Supplements".to_string(),
            is_regex: false,
            auto_scrub_enabled: true,
            created_at_epoch: chrono::Utc::now().timestamp() - 172800,
        },
    ])
}

#[tauri::command]
pub async fn add_topic_rule(pattern: String, is_regex: bool) -> Result<TopicRule> {
    let now = chrono::Utc::now().timestamp();
    Ok(TopicRule {
        id: format!("rule_{}", now),
        pattern,
        is_regex,
        auto_scrub_enabled: true,
        created_at_epoch: now,
    })
}
