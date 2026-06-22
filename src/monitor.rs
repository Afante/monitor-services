use garde::rules::AsStr;
use reqwest::{self};
use regex::Regex;
use futures::stream::{FuturesUnordered, StreamExt};
use chrono::{Local, DateTime, Utc};
use std::cell::RefCell;
use std::ffi::OsStr;
use std::rc::Rc;
use std::process::Command;
use core::convert::From;
use std::string::ToString;
use std::convert::AsRef;

use crate::prog_settings::*;
use crate::log::*;
use crate::report_by_email::*;

#[derive(Debug)]
enum MonitorAction {
    Web,
    CustomCommand,
    Sleep(i64)
}

struct Error {
    msg: String
}

impl Error {
    #[allow(unused)]
    pub fn new<F: AsRef<str>>(msg: F) -> Self {
        Self {
            msg: msg.as_ref().to_owned()
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        self.msg.clone()
    }
}

impl MonitorAction {
    fn from_target(target: &MonitorTarget) -> Option<MonitorAction> {
        match target.kind.as_str() {
            "web" => Some(MonitorAction::Web),
            "custom" => Some(MonitorAction::CustomCommand),
            _ => None
        }
    }

    async fn run(&self, monitor_name: &str, target: &MonitorTarget) -> Result<Option<String>, String> {
        match *self {
            MonitorAction::Web => self.do_web(monitor_name, target).await,
            MonitorAction::CustomCommand => self.do_custom_command(monitor_name, target).await,
            MonitorAction::Sleep(milliseconds) => self.do_sleep(monitor_name, milliseconds).await
        }
    }

    async fn check_match<'a, F: AsyncFn() -> Result<&'a str, Error>>(&self, target: &MonitorTarget, text_func: F) -> Result<Option<String>, String> {
        let text_result = text_func().await;
        if let Some(regex) = target.expect_match.as_ref() {
            let regex = match Regex::new(regex.as_str()) {
                Ok(r) => r,
                Err(err) => return Err(err.to_string())
            };
            match text_result {
                Ok(text) => {
                    if !regex.is_match(text) {
                        return Err(format!("Content did not match against regex: {}", target.expect_match.as_ref().unwrap()))
                    }
                },
                Err(err) => {
                    return Err(err.to_string());
                }
            };
        }
        if let Some(regex) = target.expect_unmatch.as_ref() {
            let regex = match Regex::new(regex.as_str()) {
                Ok(r) => r,
                Err(err) => return Err(err.to_string())
            };
            match text_result {
                Ok(text) => {
                    if regex.is_match(text) {
                        return Err(format!("Content unexpectedly matched against regex: {}", target.expect_match.as_ref().unwrap()))
                    }
                },
                Err(err) => {
                    return Err(err.to_string());
                }
            };
        }
        Ok(None)
    }

    fn call_shell_command(&self, monitor_name: &str, cmd: &str) -> Result<Option<String>, String> {
        let words_result = shell_words::split(cmd);
        log_line_f(LogLevel::Trace, monitor_name, || format!("Call: {}", cmd));
        let args = match words_result {
            Ok(a) => a,
            Err(err) => return Err(err.to_string())
        };
        let output_result = Command::new(&args[0]).args(&args[1..]).output();
        let output = match output_result {
            Ok(o) => o,
            Err(err) => return Err(err.to_string())
        };
        let os_out = unsafe { 
            OsStr::from_encoded_bytes_unchecked(output.stdout.as_slice())
        }.to_string_lossy();
        let os_err = unsafe {
            OsStr::from_encoded_bytes_unchecked(output.stderr.as_slice())
        }.to_string_lossy();
        let cmd_out = os_out.as_str();
        let cmd_err = os_err.as_str();
        if output.status.success() {
            let cmd_out_err = format!("{}\n{}", cmd_out, cmd_err);
            Ok(Some(cmd_out_err))
        }
        else {
            Err(format!("Stdout:\n{}\n\nStderr:\n{}\n", cmd_out, cmd_err))
        }
    }

    async fn do_sleep(&self, _monitor_name: &str, wait_ms: i64) -> Result<Option<String>, String> {
        std::thread::sleep(std::time::Duration::from_millis(wait_ms as u64));
        Ok(None)
    }

    async fn do_web(&self, monitor_name: &str, target: &MonitorTarget) -> Result<Option<String>, String> {
        log_line_f(LogLevel::Debug, monitor_name, || format!("Send HTTP GET to {}", target.url));
        match reqwest::get(target.url.as_str()).await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                log_line_f(LogLevel::Debug, monitor_name, || format!("Got status code {}", status_code));
                if status_code != target.expect_status_code.unwrap_or(200) {
                    return Err(format!("Got status code [{}] != expected status code [{}].", status_code, target.expect_status_code.unwrap()))
                }
                match response.text().await {
                    Ok(body) => {
                        log_line_f(LogLevel::Debug, monitor_name, || format!("Got body: {}", body.as_str()));
                        match self.check_match(target, async || {Ok(body.as_str())}).await {
                            Ok(_) => Ok(Some("Web check succeeded.".to_string())),
                            Err(err) => return Err(err)
                        }
                    },
                    Err(err) => return Err(format!("Failed to get body bytes: {}", err.to_string()))
                }
                // 
            },
            Err(err) => Err(err.to_string())
        }
    }

    async fn do_custom_command(&self, monitor_name: &str, target: &MonitorTarget) -> Result<Option<String>, String> {
        if target.custom_check_cmd.len() <= 0 {
            Err("The custom_check_cmd is not defined for the target.".to_owned())
        }
        else {
            log_line_f(LogLevel::Debug, monitor_name, || format!("Exec: {}", target.custom_check_cmd.as_str()));
            let call_result = self.call_shell_command(monitor_name, target.custom_check_cmd.as_str());
            if let Err(_) = &call_result {
                return call_result;
            }
            let output = call_result.unwrap().unwrap();
            log_line_f(LogLevel::Debug, monitor_name, || format!("Stdout + Stderr:\n{}", output.as_str()));
            match self.check_match(target, async || Ok(output.as_str())).await {
                Ok(opt) => {
                    let result = match opt {
                        Some(msg) => Ok(Some(msg)),
                        None => Ok(Some("Custom command succeeded.".to_owned()))
                    };
                    result
                },
                Err(err) => Err(err)
            }
        }
    }
}

struct MonitorRuntime {
    monitor_name: String,
    target: MonitorTarget,
    action: MonitorAction,
    last_ran: DateTime<Local>
}

impl MonitorRuntime {
    fn new(monitor_name: &String, target: &MonitorTarget) -> Self {
        Self {
            monitor_name: monitor_name.clone(),
            target: target.clone(),
            action: MonitorAction::Sleep(0),
            last_ran: DateTime::<Local>::from(DateTime::<Utc>::MIN_UTC)
        }
    }
}

struct IdleTackler {
    idle_count: usize,
    idle_limit: usize,
    sleep_count: usize,
    sleep_limit: usize,
}

impl IdleTackler {
    fn new() -> Self {
        Self {
            idle_count: 0,
            idle_limit: 10,
            sleep_count: 0,
            sleep_limit: 50
        }
    }

    fn idle_limit(mut self, limit: usize) -> Self {
        self.idle_limit = limit;
        self
    }

    fn sleep_limit(mut self, limit: usize) -> Self {
        self.sleep_limit = limit;
        self
    }

    fn on_idle(&mut self) {
        if self.idle_count >= self.idle_limit {
            if self.sleep_count < self.sleep_limit {
                self.sleep_count += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(self.sleep_count as u64 * 10));
            self.idle_count = 0;
        }
        else {
            self.idle_count += 1;
        }
    }

    fn on_busy(&mut self) {
        self.idle_count = 0;
        self.sleep_count = 0;
    }
}

async fn send_report_by_email(line: &str, title_prefix: &str, monitor_name: &str, target: &MonitorTarget, common_settings: &CommonSettings) {
    let email_result = report_by_email(
        line,
        title_prefix,
        monitor_name, 
        target, 
        common_settings
    ).await;
    if let Err(err) = email_result {
        log_line_f(
            LogLevel::Error, 
            monitor_name,
            || format!(
                "Failed to send email: {}", 
                err.to_string()
            )
        );
    }
    else {
        log_line(
            LogLevel::Debug, 
            monitor_name,
            "Successfully sent email report"
        );
    }
}

pub async fn send_error_report(line: &str, monitor_name: &str, target: &MonitorTarget, common_settings: &CommonSettings) {
    send_report_by_email(line, "Monitor error on ", monitor_name, target, common_settings).await;
}

pub async fn send_warning_report(line: &str, monitor_name: &str, target: &MonitorTarget, common_settings: &CommonSettings) {
    send_report_by_email(line, "Monitor warning on ", monitor_name, target, common_settings).await;
}

pub async fn run_monitoring(prog_settings: &ProgramSettings) {
    let mut future_queue = FuturesUnordered::new();

    let call_monitor = async |runtime_cell: Rc<RefCell<MonitorRuntime>>| {
        let now = Local::now();
        {
            let mut runtime = runtime_cell.borrow_mut();
            let total_delay = (now.timestamp_millis() - runtime.last_ran.timestamp_millis()) as f64 / 1000.0;
            let target = &runtime.target;
            let run_interval = target.check_interval_secs.unwrap_or(prog_settings.common_settings.check_interval_secs) as f64;
            if run_interval <= total_delay {
                // Its time to run again.
                runtime.action = MonitorAction::from_target(target).unwrap();
                runtime.last_ran = Local::now();
            }
            else {
                runtime.action = MonitorAction::Sleep((run_interval - total_delay) as i64);
            }
            let result = runtime.action.run(runtime.monitor_name.as_str(), &runtime.target).await;
            match result {
                Ok(opt) => {
                    if let Some(msg) = opt {
                        log_line_f(LogLevel::Info, runtime.monitor_name.as_str(), || format!("Ok: {}", msg));
                    }
                },
                Err(msg) => {
                    let mut msg = msg;
                    let mut recovered = false;
                    if runtime.target.recovery_cmd.len() > 0 {
                        log_line_f(LogLevel::Debug, runtime.monitor_name.as_str(), || format!("Recovery Exec: {}", runtime.target.recovery_cmd.as_str()));
                        let cmd_result = runtime.action.call_shell_command(&runtime.monitor_name.as_str(), runtime.target.recovery_cmd.as_str());
                        recovered = match cmd_result {
                            Ok(opt_msg) => {
                                if let Some(msg) = opt_msg {
                                    log_line_f(LogLevel::Debug, runtime.monitor_name.as_str(), || msg.clone());
                                }
                                log_line(
                                    LogLevel::Warning, 
                                    runtime.monitor_name.as_str(),
                                    "Recovery command succeeded."
                                );
                                send_warning_report(
                                    "Failed but recovery command succeeded.", 
                                    runtime.monitor_name.as_str(), 
                                    &runtime.target, 
                                    &prog_settings.common_settings
                                ).await;
                                true
                            },
                            Err(err_msg) => {
                                msg = err_msg;
                                false
                            }
                        };
                    }
                    if !recovered {
                        let line = log_line_f(
                            LogLevel::Error, 
                            runtime.monitor_name.as_str(),
                            || format!("[{}]Fail: {}",runtime.monitor_name , msg)
                        );
                        send_error_report(
                            line.as_str(), 
                            runtime.monitor_name.as_str(), 
                            &runtime.target, 
                            &prog_settings.common_settings
                        ).await;
                    }
                }
            }
        };
        runtime_cell
    };

    for (monitor_name, target) in prog_settings.monitor_targets.iter() {
        let runtime = Rc::new(RefCell::new(MonitorRuntime::new(monitor_name, target)));
        future_queue.push(call_monitor(runtime.clone()));
    }

    let mut idle_tackler = IdleTackler::new().idle_limit(future_queue.len()).sleep_limit(10);
    while let Some(runtime_cell) = future_queue.next().await {
        let runtime = runtime_cell.borrow_mut();
        future_queue.push(call_monitor(runtime_cell.clone()));
        if let MonitorAction::Sleep(_) = &runtime.action {
            idle_tackler.on_idle();
        }
        else {
            idle_tackler.on_busy();
        }
    }
}
