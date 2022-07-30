use std::process::Command;

use druid::im;

use crate::state::{AppAction, AppEntry};

use super::Plugin;

pub struct CalculatorPlugin {}

impl Plugin for CalculatorPlugin {
    fn load() -> Self {
        Self {}
    }

    fn search(&self, query: &str) -> im::Vector<AppEntry> {
        if query.starts_with('=') {
            let out = if let Ok(out) = Command::new("/usr/bin/python")
                .arg("-c")
                .arg(format!(
                    "from math import *\nprint({})",
                    query.strip_prefix("=").unwrap_or(query).trim()
                ))
                .output()
            {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                String::from("??")
            };
            im::vector![AppEntry {
                actions: im::vector![AppAction {
                    command: "TODO: copy".into(),
                    name: out
                }]
            }]
        } else {
            im::vector![]
        }
    }
}
