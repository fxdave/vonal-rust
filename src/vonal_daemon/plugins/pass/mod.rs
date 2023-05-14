use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::{CreateList, ListState},
};
use std::{
    error::Error,
    fmt::Display,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use super::{Plugin, PluginFlowControl};

#[derive(Default)]
pub struct Pass {
    list: ListState,
    config_command_list_passwords: String,
    config_command_copy_password: String,
    config_command_type_password: String,
    config_prefix: String,
    config_list_length: usize,
}

impl Pass {
    pub fn new() -> Self {
        Default::default()
    }

    fn list_passwords(&self, keyword: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let call = Command::new("bash")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("-c")
            .arg(&self.config_command_list_passwords)
            .spawn()?
            .wait_with_output()?;

        let stderr = String::from_utf8_lossy(&call.stderr);
        if !stderr.is_empty() {
            return Err(Box::new(PassError(stderr.to_string())));
        }

        let stdout = String::from_utf8_lossy(&call.stdout);
        let passwords = stdout
            .lines()
            .filter(|name| name.contains(keyword))
            .map(ToString::to_string)
            .collect();
        Ok(passwords)
    }

    fn copy_password(&self, pw: &str) -> Result<(), std::io::Error> {
        let stdout = Command::new("sh")
            .stdout(Stdio::piped())
            .arg("-c")
            .arg(self.config_command_copy_password.replace("{name}", &pw))
            .spawn()?
            .stdout;
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            if let Some(result) = reader.lines().next() {
                Command::new("notify-send").arg(result?).spawn()?;
            }
        }
        Ok(())
    }

    fn type_password(&self, pw: &str) -> Result<(), std::io::Error> {
        Command::new("bash")
            .stdout(Stdio::piped())
            .arg("-c")
            .arg(self.config_command_type_password.replace("{name}", &pw))
            .spawn()?;
        Ok(())
    }

    fn render_copy_button(
        &mut self,
        ui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        pw: &String,
        query: &mut String,
    ) {
        if ui.primary_action("Copy").activated {
            gl_window.window.set_visible(false);
            if let Err(e) = self.copy_password(pw) {
                gl_window.window.set_visible(true);
                ui.label(&e.to_string());
            } else {
                *query = "".into();
                self.list = Default::default();
            }
        }
    }

    fn render_type_button(
        &mut self,
        ui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        pw: &str,
        query: &mut String,
    ) {
        if ui.primary_action("Type").activated {
            gl_window.window.set_visible(false);
            if let Err(e) = self.type_password(&pw) {
                gl_window.window.set_visible(true);
                ui.label(&e.to_string());
            } else {
                *query = "".into();
                self.list = Default::default();
            }
        }
    }
}

const DEFAULT_PREFIX: &str = "pass";
const DEFAULT_LIST_LENGTH: usize = 10;
const DEFAULT_LIST_PASSWORDS_COMMAND: &str = r#"
shopt -s nullglob globstar
prefix=${PASSWORD_STORE_DIR-~/.password-store}
password_files=( "$prefix"/**/*.gpg )
password_files=( "${password_files[@]#"$prefix"/}" )
password_files=( "${password_files[@]%.gpg}" )
printf '%s\n' "${password_files[@]}"
"#;
const DEFAULT_COPY_PASSWORD_COMMAND: &str = r#"pass show -c {name}"#;
const DEFAULT_TYPE_PASSWORD_COMMAND: &str = r#"pass show {name} \
| { IFS= read -r pass; printf %s "$pass"; } \
| xdotool type --clearmodifiers --file -
"#;

impl Plugin for Pass {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut egui::Ui,
        gl_window: &crate::windowing::GlutinWindowContext,
    ) -> PluginFlowControl {
        if !query.starts_with(&self.config_prefix) {
            return PluginFlowControl::Continue;
        }

        let keyword = query
            .strip_prefix(&self.config_prefix)
            .unwrap_or_default()
            .trim();
        match self.list_passwords(keyword) {
            Ok(passwords) => {
                const NUMBER_OF_BUTTONS: usize = 2;
                self.list
                    .update(ui.ctx(), passwords.len(), |_| NUMBER_OF_BUTTONS);
                ui.list_limited(self.config_list_length, self.list, |mut ui| {
                    for pw in passwords {
                        ui.row(|mut rui| {
                            rui.label(&pw);
                            self.render_type_button(&mut rui, gl_window, &pw, query);
                            rui.label("/");
                            self.render_copy_button(&mut rui, gl_window, &pw, query);
                        })
                    }
                });
            }
            Err(err) => {
                ui.label(err.to_string());
            }
        }
        PluginFlowControl::Break
    }

    fn configure(&mut self, mut builder: ConfigBuilder) -> Result<ConfigBuilder, ConfigError> {
        builder.group("pass_plugin", |builder| {
            self.config_prefix = builder.get_or_create("prefix", DEFAULT_PREFIX.into())?;
            self.config_list_length =
                builder.get_or_create("config_list_length", DEFAULT_LIST_LENGTH)?;
            self.config_command_list_passwords = builder.get_or_create(
                "command_list_password",
                DEFAULT_LIST_PASSWORDS_COMMAND.trim_start().into(),
            )?;
            self.config_command_copy_password = builder.get_or_create(
                "command_copy_password",
                DEFAULT_COPY_PASSWORD_COMMAND.into(),
            )?;
            self.config_command_type_password = builder.get_or_create(
                "command_type_password",
                DEFAULT_TYPE_PASSWORD_COMMAND.trim_start().into(),
            )?;
            Ok(())
        })?;

        Ok(builder)
    }
}

#[derive(Debug)]
struct PassError(String);

impl Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for PassError {}
