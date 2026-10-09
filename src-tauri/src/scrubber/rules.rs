use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicRule {
    pub id: String,
    pub pattern: String,
    pub is_regex: bool,
    pub auto_scrub_enabled: bool,
    pub created_at_epoch: i64,
}

pub struct RuleEnforcer {
    rules: Vec<TopicRule>,
}

impl RuleEnforcer {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: TopicRule) {
        self.rules.push(rule);
    }

    pub fn matches_blacklist(&self, topic_name: &str) -> bool {
        let lower = topic_name.to_lowercase();
        self.rules.iter().any(|r| {
            if !r.auto_scrub_enabled {
                return false;
            }

            if r.is_regex {
                if let Ok(re) = Regex::new(&r.pattern) {
                    re.is_match(topic_name)
                } else {
                    false
                }
            } else {
                lower.contains(&r.pattern.to_lowercase())
            }
        })
    }

    pub fn get_rules(&self) -> &[TopicRule] {
        &self.rules
    }
}

impl Default for RuleEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_matching() {
        let mut enforcer = RuleEnforcer::new();
        enforcer.add_rule(TopicRule {
            id: "r1".to_string(),
            pattern: "casino".to_string(),
            is_regex: false,
            auto_scrub_enabled: true,
            created_at_epoch: 1000,
        });
        
        enforcer.add_rule(TopicRule {
            id: "r2".to_string(),
            pattern: "^[0-9]+.*loan$".to_string(),
            is_regex: true,
            auto_scrub_enabled: true,
            created_at_epoch: 1000,
        });

        assert!(enforcer.matches_blacklist("Online Casino & Slots"));
        assert!(enforcer.matches_blacklist("CASINO BONUSES"));
        assert!(!enforcer.matches_blacklist("Mortgage Refinancing"));
        
        assert!(enforcer.matches_blacklist("1000 payday loan"));
        assert!(!enforcer.matches_blacklist("loan for a car"));
    }
}
