use aho_corasick::AhoCorasick;
use moka::future::Cache;
use regex::{Regex, RegexSet};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ModerationCache {
    pub bad_words: Cache<String, String>,
    /// Value: Regex, description, moderation_action
    pub regex_rules: Cache<i32, Arc<(Regex, String, String)>>,
    pub settings: Cache<String, String>,
    pub bad_words_matcher: Arc<RwLock<Option<Arc<BadWordsMatcher>>>>,
    pub regex_set_bundle: Arc<RwLock<Option<Arc<RegexSetBundle>>>>,
}

impl ModerationCache {
    pub fn new() -> Self {
        Self {
            bad_words: Cache::builder().max_capacity(50_000).build(),
            regex_rules: Cache::builder().max_capacity(10_000).build(),
            settings: Cache::builder().max_capacity(1_000).build(),
            bad_words_matcher: Arc::new(RwLock::new(None)),
            regex_set_bundle: Arc::new(RwLock::new(None)),
        }
    }

    // word, moderation_action
    pub async fn load_bad_words(&self, words: Vec<(String, String)>) {
        debug!(
            "Loading bad words into cache | Words Loaded: {}",
            words.len()
        );

        self.bad_words.invalidate_all();
        let mut patterns: Vec<String> = Vec::with_capacity(words.len());
        let mut actions: Vec<String> = Vec::with_capacity(words.len());
        for (word, action) in words {
            let normalized = word.to_lowercase();
            self.bad_words
                .insert(normalized.clone(), action.clone())
                .await;
            patterns.push(normalized);
            actions.push(action);
        }

        if patterns.is_empty() {
            *self.bad_words_matcher.write().unwrap() = None;
        } else {
            let ac = AhoCorasick::new(patterns.iter()).expect("failed to build Aho-Corasick");
            let matcher = BadWordsMatcher {
                ac,
                words: patterns,
                actions,
            };
            *self.bad_words_matcher.write().unwrap() = Some(Arc::new(matcher));
        }
    }

    pub async fn load_regex_rules(&self, items: Vec<(i32, Regex, String, String)>) {
        debug!(
            "Loading regex rules into cache | Rules Loaded: {}",
            items.len()
        );

        self.regex_rules.invalidate_all();
        let mut patterns: Vec<String> = Vec::with_capacity(items.len());
        let mut descriptions: Vec<String> = Vec::with_capacity(items.len());
        let mut actions: Vec<String> = Vec::with_capacity(items.len());
        for (id, re, desc, action) in items {
            self.regex_rules
                .insert(id, Arc::new((re, desc, action)))
                .await;
        }

        for (_id, arc_val) in self.regex_rules.iter() {
            let (re, desc, action) = &*arc_val;
            patterns.push(re.as_str().to_string());
            descriptions.push(desc.clone());
            actions.push(action.clone());
        }

        if patterns.is_empty() {
            *self.regex_set_bundle.write().unwrap() = None;
        } else {
            let set = RegexSet::new(&patterns).expect("failed to build RegexSet");
            let bundle = RegexSetBundle {
                set,
                descriptions,
                actions,
            };
            *self.regex_set_bundle.write().unwrap() = Some(Arc::new(bundle));
        }
    }

    pub async fn load_settings(&self, items: Vec<(String, String)>) {
        self.settings.invalidate_all();
        for (k, v) in items {
            debug!("Loading setting into cache | Setting Loaded: {} = {}", k, v);
            self.settings.insert(k, v).await;
        }
    }
}

#[derive(Clone)]
pub struct BadWordsMatcher {
    pub ac: AhoCorasick,
    pub words: Vec<String>,
    pub actions: Vec<String>,
}

#[derive(Clone)]
pub struct RegexSetBundle {
    pub set: RegexSet,
    pub descriptions: Vec<String>,
    pub actions: Vec<String>,
}
