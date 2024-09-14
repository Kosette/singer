mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about=None, arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Start sing-box
    #[command(name = "start")]
    Start {
        /// Path to sing-box programm
        #[arg(short, long, name = "PROGRAM")]
        bin: Option<String>,

        /// Set config directory
        #[arg(short, long, name = "CONFIG DIR")]
        cdir: Option<String>,
    },

    /// Stop sing-box
    #[command(name = "stop")]
    Stop,

    /// Restart sing-box
    #[command(name = "restart")]
    Restart {
        /// Path to sing-box programm
        #[arg(short, long, name = "PROGRAM")]
        bin: Option<String>,

        /// Set config directory
        #[arg(short, long, name = "CONFIG DIR")]
        cdir: Option<String>,
    },

    /// Compile geosite.db categories to .srs binary
    ///
    /// Read categories from config.json file located in working dir <wdir>
    ///
    /// config.json structure: {"category":["abc","cde"]}
    #[command(name = "compile")]
    Compile {
        /// File path to geosite.db
        #[arg(short, long)]
        file: String,

        /// Path to sing-box program
        #[arg(short, long, name = "PROGRAM")]
        bin: Option<String>,

        /// Set Working directory
        #[arg(short, long, name = "WORKING DIR")]
        wdir: Option<String>,
    },

    /// Configure settings
    ///
    /// for example, set "wdir='c:/abc'"
    #[command(name = "config", arg_required_else_help(true))]
    Config {
        /// List all settings
        #[arg(short, long, exclusive(true))]
        list: bool,

        /// Configure specific setting
        #[arg(
            long,
            name = "\"key\" \"value\"",
            num_args(..=2),
            exclusive(true)
        )]
        set: Vec<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Operation::Start { bin, cdir } => {
            let (sing_path, _, config_dir) = utils::get_valid_options(bin, cdir.clone(), cdir);

            let output = utils::create_orphan_process(
                sing_path.display().to_string().as_str(),
                &[
                    "run",
                    "-C",
                    config_dir.display().to_string().as_str(),
                    "-D",
                    config_dir.display().to_string().as_str(),
                ],
            );

            if output.is_ok() {
                println!("sing-box successfully started");
            } else {
                eprintln!("Error: {}", output.err().unwrap());
            }
        }
        Operation::Stop => {
            let mut stop = std::process::Command::new("taskkill");
            stop.args(["/F", "/IM", "sing-box*"]);

            let output = stop.output()?;
            if output.status.success() {
                println!("sing-box successfully stopped")
            } else {
                eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Operation::Restart { bin, cdir } => {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "sing-box*"])
                .output()?;

            let (sing_path, _, config_dir) = utils::get_valid_options(bin, cdir.clone(), cdir);

            let output = utils::create_orphan_process(
                sing_path.display().to_string().as_str(),
                &[
                    "run",
                    "-C",
                    config_dir.display().to_string().as_str(),
                    "-D",
                    config_dir.display().to_string().as_str(),
                ],
            );

            if output.is_ok() {
                println!("sing-box successfully restarted");
            } else {
                eprintln!("Error: {}", output.err().unwrap());
            }
        }
        Operation::Compile { file, bin, wdir } => {
            let (sing_path, working_dir, _) = utils::get_valid_options(bin, wdir.clone(), wdir);

            let output = utils::compile_binary(
                sing_path.display().to_string().as_str(),
                file.as_str(),
                working_dir.display().to_string().as_str(),
            );

            if output.is_err() {
                eprintln!("Error: {}", output.err().unwrap())
            }
        }
        Operation::Config { list, set } => {
            if list {
                match utils::Config::load() {
                    Ok(s) => println!("{}", s),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }

            if set.len() == 2 {
                utils::set_config(set)?;
            }
        }
    };
    Ok(())
}
