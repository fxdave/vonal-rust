use crate::{
    config::{ConfigBuilder, ConfigError},
    theme::list::{self, ListState},
};
use std::{
    error::Error,
    fmt::Display,
    io::{BufRead, BufReader, Write},
    process::{Command, Output, Stdio},
    thread,
    time::Duration,
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
const DEFAULT_COPY_PASSWORD_COMMAND: &str = r#"pass show -c "$PASSWORD_NAME""#;
const DEFAULT_TYPE_PASSWORD_COMMAND: &str = r#"pass show "$PASSWORD_NAME" \
| { IFS= read -r pass; printf %s "$pass"; } \
| if [ -n "$WAYLAND_DISPLAY" ]; then wtype -; else xdotool type --clearmodifiers --file -; fi
"#;
const DEFAULT_COPY_OTP_COMMAND: &str = r#"pass otp -c "$PASSWORD_NAME""#;
const DEFAULT_TYPE_OTP_COMMAND: &str = r#"pass otp "$PASSWORD_NAME" \
| tr -d '\n' \
| if [ -n "$WAYLAND_DISPLAY" ]; then wtype -; else xdotool type --clearmodifiers --file -; fi
"#;
const DEFAULT_GENERATE_PASSWORD_COMMAND: &str = r#"pass generate "$PASSWORD_NAME""#;
const DEFAULT_MANUAL_ADD_PASSWORD_COMMAND: &str = r#"pass insert -e "$PASSWORD_NAME""#;
const DEFAULT_DELETE_PASSWORD_COMMAND: &str = r#"pass delete -f "$PASSWORD_NAME""#;
/// Gives time for the window to hide and for the focus to return to the previous window
const TYPE_PASSWORD_DELAY: Duration = Duration::from_millis(200);

struct MessageState {
    message: String,
    query: String,
}

#[derive(Default)]
pub struct Pass {
    message: Option<MessageState>,
    config_command_list_passwords: String,
    config_command_copy_password: String,
    config_command_type_password: String,
    config_command_copy_otp: String,
    config_command_type_otp: String,
    config_command_generate_password: String,
    config_command_manual_add_password: String,
    config_command_delete_password: String,
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
            .filter(|name| keyword.is_empty() || name.contains(keyword))
            .map(ToString::to_string)
            .collect();
        Ok(passwords)
    }

    /// The name is passed as an environment variable, so it can't inject shell code.
    /// Old configs with `{name}` are still supported.
    fn create_command(&self, command: &str, name: &str) -> Command {
        let mut process = Command::new("bash");
        process
            .arg("-c")
            .arg(command.replace("{name}", r#""$PASSWORD_NAME""#))
            .env("PASSWORD_NAME", name);
        process
    }

    fn copy_secret(&self, command: &str, name: &str) -> Result<(), Box<dyn Error>> {
        let mut child = self
            .create_command(command, name)
            .stdout(Stdio::piped())
            .spawn()?;
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            if let Some(result) = reader.lines().next() {
                Command::new("notify-send").arg(result?).spawn()?;
                return Ok(());
            }
        }

        // nothing was printed, so it's probably failed
        let status = child.wait()?;
        if !status.success() {
            return Err(Box::new(PassError(format!("Command failed with {status}"))));
        }
        Ok(())
    }

    fn run_bash(&self, command: &str, name: &str) -> Result<(), Box<dyn Error>> {
        let output = self.create_command(command, name).output()?;
        check_output(&output)
    }

    fn type_secret(&self, command: &str, name: &str) {
        let mut command = self.create_command(command, name);
        thread::spawn(move || {
            thread::sleep(TYPE_PASSWORD_DELAY);
            let result: Result<(), Box<dyn Error>> = command
                .output()
                .map_err(Into::into)
                .and_then(|output| check_output(&output));
            if let Err(error) = result {
                Command::new("notify-send")
                    .arg(error.to_string())
                    .spawn()
                    .ok();
            }
        });
    }

    fn generate_password(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.run_bash(&self.config_command_generate_password, name)
    }

    fn add_password_manually(&self, name: &str, password: &str) -> Result<(), Box<dyn Error>> {
        let mut child = self
            .create_command(&self.config_command_manual_add_password, name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin.write_all(password.as_bytes())?;
        let output = child.wait_with_output()?;
        check_output(&output)
    }

    fn delete_password(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.run_bash(&self.config_command_delete_password, name)
    }

    fn render_copy_button(
        &self,
        rui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        command: &str,
        pw: &str,
        query: &mut String,
    ) {
        if rui.primary_action("Copy").activated {
            gl_window.set_visible(false);
            if let Err(e) = self.copy_secret(command, pw) {
                gl_window.set_visible(true);
                rui.label(&e.to_string());
            } else {
                *query = "".into();
                ListState::reset(rui.ui.ctx(), 0);
            }
        }
    }

    fn render_type_button(
        &self,
        rui: &mut crate::theme::list::RowUi,
        gl_window: &crate::windowing::GlutinWindowContext,
        command: &str,
        pw: &str,
        query: &mut String,
    ) {
        if rui.primary_action("Type").activated {
            gl_window.set_visible(false);
            self.type_secret(command, pw);
            *query = "".into();
            ListState::reset(rui.ui.ctx(), 0);
        }
    }

    fn is_existing_password(&self, password: &str) -> Result<bool, Box<dyn Error>> {
        Ok(self.list_passwords("")?.iter().any(|pw| pw == password))
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
            self.generate_password(password_name)?;
            self.add_message(query.to_owned(), "Password is added successfully!".into());
        }
        Ok(())
    }

    fn render_delete_button(
        &mut self,
        rui: &mut crate::theme::list::RowUi,
        password_name: &str,
        query: &str,
    ) -> Result<(), Box<dyn Error>> {
        if rui.primary_action("delete").activated {
            self.delete_password(password_name)?;
            self.add_message(query.to_owned(), "Password is deleted successfully!".into());
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
            "Add entry named \"{}\" with password: \"{}\"",
            password_name, password
        );
        if rui.primary_action(button_title).activated {
            if self.is_existing_password(password_name)? {
                self.add_message(query.to_owned(), "Please remove the password first.".into());
                return Ok(());
            }
            self.add_password_manually(password_name, password)?;
            self.add_message(query.to_owned(), "Password is added successfully!".into());
        }
        Ok(())
    }

    fn add_message(&mut self, query: String, message: String) {
        self.message = Some(MessageState { message, query })
    }

    fn render_message(&mut self, ui: &mut egui::Ui, query: &str) {
        let should_clear_message = if let Some(message) = &self.message {
            message.query != query
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
    fn search(&mut self, ui: &mut egui::Ui, ctx: &mut PluginContext<'_>) {
        if !ctx.query.starts_with(&self.config_prefix) {
            return;
        }

        self.render_message(ui, ctx.query);

        let keyword = ctx
            .query
            .strip_prefix(&self.config_prefix)
            .unwrap_or_default()
            .trim()
            .to_owned();

        if keyword == "add" || keyword.starts_with("add ") {
            let segments: Vec<&str> = keyword.split(' ').collect();
            ui.add(list::List::new().with_builder(|list_ui| {
                list_ui.passive_row(|ui| {
                    ui.label(" ");
                    ui.label("Examples:");
                    ui.label("add email");
                    ui.label("add email 1234");
                    ui.label(" ");
                });
                if segments.len() < 3 {
                    list_ui.row(|rui| {
                        ctx.verify_result(self.render_generate_button(
                            rui,
                            segments.get(1).unwrap_or(&""),
                            ctx.query,
                        ));
                    });
                }
                list_ui.row(|rui| {
                    ctx.verify_result(self.render_manual_add_button(
                        rui,
                        segments.get(1).unwrap_or(&""),
                        segments.get(2).unwrap_or(&""),
                        ctx.query,
                    ));
                });
            }));
            return ctx.break_flow();
        }

        let is_delete = keyword == "delete" || keyword.starts_with("delete ");
        let is_otp = keyword == "otp" || keyword.starts_with("otp ");
        let password_name = if is_delete {
            keyword
                .trim_start_matches("delete")
                .trim_start_matches(' ')
                .to_string()
        } else if is_otp {
            keyword
                .trim_start_matches("otp")
                .trim_start_matches(' ')
                .to_string()
        } else {
            keyword
        };

        match self.list_passwords(&password_name) {
            Ok(passwords) => {
                ui.add(
                    list::List::new()
                        .with_limit(self.config_list_length)
                        .with_builder(|list_ui| {
                            if is_delete {
                                list_ui.passive_row(|ui| {
                                    ui.label(" ");
                                    ui.label("Examples:");
                                    ui.label("delete email");
                                    ui.label(" ");
                                });
                            }
                            if is_otp {
                                list_ui.passive_row(|ui| {
                                    ui.label(" ");
                                    ui.label("Examples:");
                                    ui.label("otp github");
                                    ui.label(" ");
                                });
                            }
                            for pw in passwords {
                                list_ui.row(|rui| {
                                    rui.label(&pw);
                                    if is_delete {
                                        ctx.verify_result(
                                            self.render_delete_button(rui, &pw, ctx.query),
                                        );
                                    } else if is_otp {
                                        self.render_type_button(
                                            rui,
                                            ctx.gl_window,
                                            &self.config_command_type_otp,
                                            &pw,
                                            ctx.query,
                                        );
                                        rui.label("/");
                                        self.render_copy_button(
                                            rui,
                                            ctx.gl_window,
                                            &self.config_command_copy_otp,
                                            &pw,
                                            ctx.query,
                                        );
                                    } else {
                                        self.render_type_button(
                                            rui,
                                            ctx.gl_window,
                                            &self.config_command_type_password,
                                            &pw,
                                            ctx.query,
                                        );
                                        rui.label("/");
                                        self.render_copy_button(
                                            rui,
                                            ctx.gl_window,
                                            &self.config_command_copy_password,
                                            &pw,
                                            ctx.query,
                                        );
                                    }
                                });
                            }
                        }),
                );
            }
            Err(err) => {
                ui.label(err.to_string());
            }
        }
        ctx.break_flow()
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
            self.config_command_copy_otp =
                builder.get_or_create("command_copy_otp", DEFAULT_COPY_OTP_COMMAND.into())?;
            self.config_command_type_otp = builder.get_or_create(
                "command_type_otp",
                DEFAULT_TYPE_OTP_COMMAND.trim_start().into(),
            )?;
            self.config_command_generate_password = builder.get_or_create(
                "command_generate_password",
                DEFAULT_GENERATE_PASSWORD_COMMAND.trim_start().into(),
            )?;
            self.config_command_manual_add_password = builder.get_or_create(
                "command_manual_add_password_from_stdin",
                DEFAULT_MANUAL_ADD_PASSWORD_COMMAND.trim_start().into(),
            )?;
            self.config_command_delete_password = builder.get_or_create(
                "command_delete_password",
                DEFAULT_DELETE_PASSWORD_COMMAND.trim_start().into(),
            )?;
            Ok(())
        })?;

        Ok(builder)
    }
}

fn check_output(output: &Output) -> Result<(), Box<dyn Error>> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = if stderr.trim().is_empty() {
        format!("Command failed with {}", output.status)
    } else {
        stderr.trim().to_string()
    };
    Err(Box::new(PassError(message)))
}

#[derive(Debug)]
struct PassError(String);

impl Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for PassError {}
