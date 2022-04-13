use regex::Regex;

const DEFAULT_SEARCH_CONSERVE_NODES: bool = false;
const DEFAULT_SEARCH_PE_RADIUS: f32 = 0.25;
const DEFAULT_SEARCH_THREAD_RADIUS: f32 = 0.5;

#[derive(Debug)]
pub struct SearchOptions {
    pub conserve_nodes: bool,
    pub pe_radius: f32,
    pub thread_radius: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            conserve_nodes: DEFAULT_SEARCH_CONSERVE_NODES,
            pe_radius: DEFAULT_SEARCH_PE_RADIUS,
            thread_radius: DEFAULT_SEARCH_THREAD_RADIUS,
        }
    }
}

impl SearchOptions {
    pub fn parse(s: &str) -> Result<Self, String> {
        let pe_radius_matcher = FloatOption::new("pe_radius");
        let thread_radius_matcher = FloatOption::new("thread_radius");

        let mut conserve_nodes = DEFAULT_SEARCH_CONSERVE_NODES;
        let mut pe_radius = DEFAULT_SEARCH_PE_RADIUS;
        let mut thread_radius = DEFAULT_SEARCH_THREAD_RADIUS;

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
