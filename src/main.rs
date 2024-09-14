use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about=None, arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Operation,

    /// Set path to sing-box programm, using slash not back-slash
    #[arg(short, long, name = "PROGRAM")]
    exe: Option<String>,

    /// Set working directory and config directory
    #[arg(short, long, name = "WORKING DIR")]
    dir: Option<String>,
}

#[derive(Subcommand)]
enum Operation {
    /// Start sing-box
    #[command(name = "start")]
    Start,

    /// Stop sing-box
    #[command(name = "stop")]
    Stop,

    /// Restart sing-box
    #[command(name = "restart")]
    Restart,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let sing_path = PathBuf::from(
        cli.exe
            .unwrap_or("d:/Applications/sing-box/sing-box.exe".to_string()),
    );
    let working_dir = PathBuf::from(cli.dir.unwrap_or("d:/Applications/sing-box".to_string()));

    if !sing_path.exists() {
        eprintln!("Error: sing-box program path does not exist");
        std::process::exit(-1);
    } else if !working_dir.exists() {
        eprintln!("Error: working directory does not exist");
        std::process::exit(-2);
    }

    match cli.command {
        Operation::Start => {
            let output = create_orphan_process(
                sing_path.display().to_string().as_str(),
                &[
                    "run",
                    "-C",
                    working_dir.display().to_string().as_str(),
                    "-D",
                    working_dir.display().to_string().as_str(),
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
        Operation::Restart => {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/IM", "sing-box*"])
                .output()?;

            let output = create_orphan_process(
                sing_path.display().to_string().as_str(),
                &[
                    "run",
                    "-C",
                    working_dir.display().to_string().as_str(),
                    "-D",
                    working_dir.display().to_string().as_str(),
                ],
            );

            if output.is_ok() {
                println!("sing-box successfully started");
            } else {
                eprintln!("Error: {}", output.err().unwrap());
            }
        }
    };
    Ok(())
}

#[cfg(target_family = "windows")]
fn create_orphan_process(program: &str, args: &[&str]) -> std::io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PWSTR;
    use windows::Win32::System::Threading::{
        CreateProcessW, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS, PROCESS_INFORMATION,
        STARTUPINFOW,
    };

    let mut command_line = format!("\"{}\"", program);
    for arg in args {
        command_line.push_str(&format!(" \"{}\"", arg));
    }

    let mut wide_command: Vec<u16> = OsStr::new(&command_line)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

    let mut process_info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let creation_flags = CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS;

    let result = unsafe {
        CreateProcessW(
            PWSTR::null(),
            PWSTR(wide_command.as_mut_ptr()),
            None,
            None,
            false,
            creation_flags,
            None,
            PWSTR::null(),
            &startup_info,
            &mut process_info,
        )
    };

    if result.is_ok() {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(process_info.hProcess);
            let _ = windows::Win32::Foundation::CloseHandle(process_info.hThread);
        }
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_family = "unix")]
fn create_orphan_process(program: &str, args: &[&str]) -> std::io::Result<()> {
    use nix::unistd::{fork, ForkResult};
    use std::os::unix::process::CommandExt;

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child: _ }) => {
            // 父进程立即退出
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            // 子进程
            match unsafe { fork() } {
                Ok(ForkResult::Parent { child: _ }) => {
                    // 第一个子进程退出
                    std::process::exit(0);
                }
                Ok(ForkResult::Child) => {
                    // 孙进程（现在是孤儿进程）执行新程序
                    Command::new(program).args(args).exec();
                    unreachable!();
                }
                Err(_) => std::process::exit(1),
            }
        }
        Err(_) => std::process::exit(1),
    }
}
