use env_logger;

mod prog_settings;
use prog_settings::*;

mod log;
use log::*;

mod report_by_email;

mod monitor;
use monitor::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    
    let exec = async || -> Result<(), String> {
        let cmd_args = CmdArgs::parse()?;
        let mut prog_settings = load_settings_from_dir(&cmd_args.cfg_dir)?;
        set_min_log_level(prog_settings.common_settings.min_log_level);
        run_monitoring(&mut prog_settings).await;
        Ok(())
    };
    if let Err(msg) = exec().await {
        eprintln!("{}", msg);
        std::process::exit(1);
    }
}

