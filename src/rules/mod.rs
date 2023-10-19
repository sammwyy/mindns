use std::path::PathBuf;

use crate::{config::RulesSettings, utils};

const A_DENY: u8 = 0;
const A_REPLACE: u8 = 1;
const A_APPEND: u8 = 2;

const M_EQUAL: u8 = 0;
const M_END: u8 = 1;
const M_START: u8 = 2;

#[derive(Debug, Clone)]
pub struct Rule {
    pub action: u8,
    pub mode: u8,
    pub reverse: bool,
    pub key: String,
    pub value: Option<String>,
}

pub fn parse_rule(raw: &str) -> Rule {
    let rule: Vec<&str> = raw.split(" ").collect();
    let action = match rule[0] {
        "deny" => A_DENY,
        "repl" => A_REPLACE,
        "apnd" => A_APPEND,
        _ => panic!("Invalid action {}", rule[0]),
    };
    let raw_key = rule[1].to_string().replace("!", "");
    let mode = if raw_key.starts_with("*") {
        M_END
    } else if raw_key.ends_with("*") {
        M_START
    } else {
        M_EQUAL
    };
    let reverse = rule[1].starts_with("!");
    let key = raw_key.replace("*", "");
    let value = if rule.len() > 3 {
        Some(rule[3].to_string())
    } else {
        None
    };

    Rule {
        action,
        mode,
        reverse,
        key,
        value,
    }
}

fn ignore_line(line: &str) -> bool {
    line.starts_with("#") || line.trim().is_empty()
}

pub fn parse_rules(file_path: PathBuf) -> Vec<Rule> {
    let rules = std::fs::read_to_string(file_path).unwrap();
    let rules: Vec<&str> = rules.split("\n").collect();
    let mut parsed_rules: Vec<Rule> = Vec::new();

    for line in rules {
        if ignore_line(line) {
            continue;
        }

        let rule = parse_rule(line);
        parsed_rules.push(rule);
    }

    parsed_rules
}

pub fn parse_rules_dir(dir_path: PathBuf) -> Vec<Rule> {
    let mut parsed_rules: Vec<Rule> = Vec::new();

    for entry in std::fs::read_dir(dir_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            let mut dir_rules = parse_rules_dir(path);
            parsed_rules.append(&mut dir_rules);
        } else if entry.file_name().to_str().unwrap().ends_with(".rules") {
            let mut file_rules = parse_rules(path);
            parsed_rules.append(&mut file_rules);
        }
    }

    parsed_rules
}

pub fn parse_rules_config(config: &Vec<RulesSettings>) -> Vec<Rule> {
    let mut parsed_rules: Vec<Rule> = Vec::new();

    for rule_file in config {
        let path = utils::get_path(&rule_file.path);

        if rule_file.load_as == "file" {
            let mut file_rules = parse_rules(path);
            parsed_rules.append(&mut file_rules);
        } else if rule_file.load_as == "dir" {
            let mut dir_rules = parse_rules_dir(path);
            parsed_rules.append(&mut dir_rules);
        }
    }

    parsed_rules
}

pub fn match_rule(rules: &Vec<Rule>, query: &str) -> bool {
    for rule in rules {
        let mut match_rule;

        match rule.mode {
            M_EQUAL => match query == rule.key {
                true => match_rule = true,
                false => continue,
            },
            M_END => match query.ends_with(&rule.key) {
                true => match_rule = true,
                false => continue,
            },
            M_START => match query.starts_with(&rule.key) {
                true => match_rule = true,
                false => continue,
            },
            _ => panic!("Invalid mode"),
        }

        if rule.reverse {
            match_rule = !match_rule;
        }

        if match_rule {
            return true;
        }
    }

    false
}
