use regex::Regex;

use crate::config::SearchConfig;

#[derive(Debug)]
pub struct SearchOptions {
    pub conserve_nodes: bool,
    pub pe_radius: f32,
    pub thread_radius: f32,
}

impl SearchOptions {
    pub fn default(search_config: SearchConfig) -> Self {
        Self {
            conserve_nodes: search_config.conserve_nodes,
            pe_radius: search_config.pe_radius,
            thread_radius: search_config.thread_radius,
        }
    }

    pub fn parse(s: &str, search_config: SearchConfig) -> Result<Self, String> {
        let pe_radius_matcher = FloatOption::new("pe_radius");
        let thread_radius_matcher = FloatOption::new("thread_radius");

        let mut conserve_nodes = search_config.conserve_nodes;
        let mut pe_radius = search_config.pe_radius;
        let mut thread_radius = search_config.thread_radius;

        for opt in s.split(',') {
            if opt == "conserve_nodes" {
                conserve_nodes = true;
            } else if pe_radius_matcher.is_match(opt) {
                pe_radius = pe_radius_matcher.get_value(opt);
            } else if thread_radius_matcher.is_match(opt) {
                thread_radius = thread_radius_matcher.get_value(opt);
            } else {
                return Err(format!("unknown search option: {}", opt));
            }
        }

        Ok(Self {
            conserve_nodes,
            pe_radius,
            thread_radius,
        })
    }
}

struct FloatOption {
    regex: Regex,
}

impl FloatOption {
    fn new(option_name: &str) -> Self {
        Self {
            regex: Regex::new(&format!(
                "^{}={}$",
                regex::escape(option_name),
                r#"(?P<value>[0-9]*\.?[0-9]+)"#
            ))
            .unwrap(),
        }
    }

    fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    fn get_value(&self, text: &str) -> f32 {
        self.regex
            .captures(text)
            .and_then(|c| c.name("value").map(|m| m.as_str()))
            .unwrap()
            .parse()
            .unwrap()
    }
}
