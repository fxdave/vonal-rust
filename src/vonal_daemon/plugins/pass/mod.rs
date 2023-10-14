use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::{CreateList, ListState},
};
use std::{
    error::Error,
    fmt::Display,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use super::{Plugin, PluginContext};

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
const DEFAULT_GENERATE_PASSWORD_COMMAND: &str = r#"pass generate {name}"#;
const DEFAULT_MANUAL_ADD_PASSWORD_COMMAND: &str = r#"pass insert -e {name}"#;

struct MessageState {
    message: String,
    query: String,
}

#[derive(Default)]
pub struct Pass {
    message: Option<MessageState>,
    list: ListState,
    config_command_list_passwords: String,
    config_command_copy_password: String,
    config_command_type_password: String,
    config_command_generate_password: String,
    config_command_manual_add_password: String,
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

    fn run_bash(&self, command: &str) -> Result<(), std::io::Error> {
        Command::new("bash")
            .stdout(Stdio::piped())
            .arg("-c")
            .arg(command)
            .spawn()?;
        Ok(())
    }

    fn type_password(&self, name: &str) -> Result<(), std::io::Error> {
        self.run_bash(&self.config_command_type_password.replace("{name}", &name))
    }

    fn generate_password(&self, name: &str) -> Result<(), std::io::Error> {
        self.run_bash(
            &self
                .config_command_generate_password
                .replace("{name}", &name),
        )
    }

    fn add_password_manually(&self, name: &str, password: &str) -> Result<(), std::io::Error> {
        let mut child = Command::new("bash")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .arg("-c")
            .arg(
                &self
                    .config_command_manual_add_password
                    .replace("{name}", &name),
            )
            .spawn()?;
        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin.write_all(password.as_bytes())?;
        child.wait_with_output()?;
        Ok(())
    }

    fn render_copy_button(
        &mut self,
        rui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        pw: &String,
        query: &mut String,
    ) {
        if rui.primary_action("Copy").activated {
            gl_window.window.set_visible(false);
            if let Err(e) = self.copy_password(pw) {
                gl_window.window.set_visible(true);
                rui.label(&e.to_string());
            } else {
                *query = "".into();
                self.list = Default::default();
            }
        }
    }

    fn render_type_button(
        &mut self,
        rui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        pw: &str,
        query: &mut String,
    ) {
        if rui.primary_action("Type").activated {
            gl_window.window.set_visible(false);
            if let Err(e) = self.type_password(&pw) {
                gl_window.window.set_visible(true);
                rui.label(&e.to_string());
            } else {
                *query = "".into();
                self.list = Default::default();
            }
        }
    }

    fn is_existing_password(&self, password: &str) -> Result<bool, Box<dyn Error>> {
        Ok(self
            .list_passwords("")?
            .iter()
            .find(|pw| *pw == password)
            .is_some())
    }

    fn render_generate_button(
        &mut self,
        rui: &mut crate::theme::list::RowUi,
        password_name: &str,
        query: &str,
    ) -> Result<(), Box<dyn Error>> {
        let button_title = &format!("Generate pass for \"{}\"", password_name);
        if rui.primary_action(button_title).activated {
            if self.is_existing_password(password_name)? {
                self.add_message(query.to_owned(), "Please remove the password first.".into());
                return Ok(());
            }
            self.generate_password(&password_name)?;
            self.add_message(query.to_owned(), "Password is added sucessfully!".into());
        }
        Ok(())
    }

    fn render_manual_add_button(
        &mut self,
        rui: &mut crate::theme::list::RowUi,
        password_name: &str,
        password: &str,
        query: &str,
    ) -> Result<(), Box<dyn Error>> {
        let button_title = &format!(
            "Add enty named \"{}\" with password: \"{}\"",
            password_name, password
        );
        if rui.primary_action(button_title).activated {
            if self.is_existing_password(password_name)? {
                self.add_message(query.to_owned(), "Please remove the password first.".into());
                return Ok(());
            }
            self.add_password_manually(&password_name, &password)?;
            self.add_message(query.to_owned(), "Password is added sucessfully!".into());
        }
        Ok(())
    }

    fn add_message(&mut self, query: String, message: String) {
        self.message = Some(MessageState { message, query })
    }

    fn render_message(&mut self, ui: &mut egui::Ui, query: &str) {
        let should_clear_message = if let Some(message) = &self.message {
            &message.query != query
        } else {
            false
        };

        if should_clear_message {
            self.message = None;
        }

        if let Some(message) = &self.message {
            ui.label(&message.message);
            ui.label("");
        }
    }
}

impl Plugin for Pass {
    fn search<'a>(&mut self, ui: &mut egui::Ui, ctx: &mut PluginContext<'a>) {
        if !ctx.query.starts_with(&self.config_prefix) {
            return;
        }

        self.render_message(ui, &ctx.query);

        let keyword = ctx
            .query
            .strip_prefix(&self.config_prefix)
            .unwrap_or_default()
            .trim()
            .to_owned();

        if keyword == "add" || keyword.starts_with("add ") {
            let segments: Vec<&str> = keyword.split(" ").collect();
            ui.list(ctx.egui_ctx, &mut self.list, |list_ui, ui| {
                list_ui.passive(|| {
                    ui.label(" ");
                    ui.label("Examples:");
                    ui.label("add email");
                    ui.label("add email 1234");
                    ui.label(" ");
                });
                if segments.len() < 3 {
                    list_ui.row(ui, |mut rui| {
                        ctx.verify_result(self.render_generate_button(
                            &mut rui,
                            segments.get(1).unwrap_or(&""),
                            &ctx.query,
                        ));
                    });
                }
                list_ui.row(ui, |mut rui| {
                    ctx.verify_result(self.render_manual_add_button(
                        &mut rui,
                        segments.get(1).unwrap_or(&""),
                        segments.get(2).unwrap_or(&""),
                        &ctx.query,
                    ));
                });
            });
            return ctx.break_flow();
        }

        match self.list_passwords(&keyword) {
            Ok(passwords) => {
                const NUMBER_OF_BUTTONS: usize = 2;
                self.list
                    .update(ui.ctx(), passwords.len(), |_| NUMBER_OF_BUTTONS);
                ui.list_limited(
                    self.config_list_length,
                    self.list,
                    |mut list_ui, ui: &mut egui::Ui| {
                        for pw in passwords {
                            list_ui.row(ui, |mut rui| {
                                rui.label(&pw);
                                self.render_type_button(&mut rui, ctx.gl_window, &pw, ctx.query);
                                rui.label("/");
                                self.render_copy_button(&mut rui, ctx.gl_window, &pw, ctx.query);
                            });
                        }
                    },
                );
            }
            Err(err) => {
                ui.label(err.to_string());
            }
        }
        return ctx.break_flow();
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
            self.config_command_generate_password = builder.get_or_create(
                "command_generate_password",
                DEFAULT_GENERATE_PASSWORD_COMMAND.trim_start().into(),
            )?;
            self.config_command_manual_add_password = builder.get_or_create(
                "command_manual_add_password_from_stdin",
                DEFAULT_MANUAL_ADD_PASSWORD_COMMAND.trim_start().into(),
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
