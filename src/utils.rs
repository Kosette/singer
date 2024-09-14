#[cfg(target_family = "windows")]
pub fn create_orphan_process(program: &str, args: &[&str]) -> std::io::Result<()> {
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

// #[cfg(target_family = "unix")]
// pub fn create_orphan_process(program: &str, args: &[&str]) -> std::io::Result<()> {
//     use nix::unistd::{fork, ForkResult};
//     use std::os::unix::process::CommandExt;

//     match unsafe { fork() } {
//         Ok(ForkResult::Parent { child: _ }) => {
//             // 父进程立即退出
//             std::process::exit(0);
//         }
//         Ok(ForkResult::Child) => {
//             // 子进程
//             match unsafe { fork() } {
//                 Ok(ForkResult::Parent { child: _ }) => {
//                     // 第一个子进程退出
//                     std::process::exit(0);
//                 }
//                 Ok(ForkResult::Child) => {
//                     // 孙进程（现在是孤儿进程）执行新程序
//                     Command::new(program).args(args).exec();
//                     unreachable!();
//                 }
//                 Err(_) => std::process::exit(1),
//             }
//         }
//         Err(_) => std::process::exit(1),
//     }
// }

pub fn compile_binary(bin: &str, file: &str, dir: &str) -> std::io::Result<()> {
    use serde_json::Value;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use std::process::Command;

    if !PathBuf::from(bin).exists() {
        eprintln!("sing-box program path does not exist");
        std::process::exit(-1);
    }

    // 读取config.json文件
    let config_file = PathBuf::from(dir).join("config.json");
    let mut config_file = File::open(config_file)?;
    let mut contents = String::new();
    config_file.read_to_string(&mut contents)?;

    // 解析JSON内容
    let json: Value = serde_json::from_str(&contents)?;

    // 提取category数组
    if let Some(categories) = json["category"].as_array() {
        for category in categories {
            if let Some(item) = category.as_str() {
                // 执行sing-box命令
                let output = Command::new(bin)
                    .args([
                        "geosite",
                        "export",
                        item,
                        "-f",
                        file,
                        "-o",
                        PathBuf::from(dir)
                            .join(format!("geosite-{}.json", item))
                            .display()
                            .to_string()
                            .as_str(),
                        "-D",
                        PathBuf::from(dir).display().to_string().as_str(),
                    ])
                    .output()?;

                if output.status.success() {
                    println!("Successfully exported {}", item);
                } else {
                    eprintln!("Failed to export {}", item);
                    eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                }

                let category_path = format!("geosite-{}.json", item);
                let output = Command::new(bin)
                    .args([
                        "rule-set",
                        "compile",
                        category_path.as_str(),
                        "-D",
                        PathBuf::from(dir).display().to_string().as_str(),
                    ])
                    .output()?;

                if output.status.success() {
                    println!("Successfully compiled {}", item);
                    std::fs::remove_file(
                        PathBuf::from(dir).join(format!("geosite-{}.json", item)),
                    )?;
                } else {
                    eprintln!("Failed to compile {}", item);
                    eprintln!("Error: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    } else {
        eprintln!("No valid category array found in config.json");
    }

    Ok(())
}

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Config {
    bin: Option<String>,
    wdir: Option<String>,
    cdir: Option<String>,
}

impl Config {
    pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
        let config_file = dirs::config_dir()
            .unwrap()
            .join("singer")
            .join("config.toml");

        let data = std::fs::read_to_string(&config_file)?;
        let config: Config = toml::from_str(&data)?;
        Ok(config)
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let bin = if let Some(bin) = &self.bin {
            bin
        } else {
            &"".to_string()
        };
        let wdir = if let Some(wdir) = &self.wdir {
            wdir
        } else {
            &"".to_string()
        };
        let cdir = if let Some(cdir) = &self.cdir {
            cdir
        } else {
            &"".to_string()
        };
        write!(f, "bin={}\nwdir={}\ncdir={}", bin, wdir, cdir)
    }
}

use std::path::PathBuf;

pub fn get_valid_options(
    bin: Option<String>,
    wdir: Option<String>,
    cdir: Option<String>,
) -> (PathBuf, PathBuf, PathBuf) {
    let sing_path = if let Some(bin) = bin {
        if !PathBuf::from(&bin).exists() {
            eprintln!("Error: <--bin/-b>: sing-box program path does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(bin)
        }
    } else if Config::load().is_ok() && Config::load().unwrap().bin.is_some() {
        if !PathBuf::from(&Config::load().unwrap().bin.unwrap()).exists() {
            eprintln!("Error: <config.toml>: sing-box program path does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(Config::load().unwrap().bin.unwrap())
        }
    } else {
        eprintln!("Missing option <--bin/-b>, using '-h' to print help");
        std::process::exit(-1);
    };

    let working_dir = if let Some(wdir) = wdir {
        if !PathBuf::from(&wdir).exists() {
            eprintln!("Error: <--wdir/-w>: working directory does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(wdir)
        }
    } else if Config::load().is_ok() && Config::load().unwrap().wdir.is_some() {
        if !PathBuf::from(&Config::load().unwrap().wdir.unwrap()).exists() {
            eprintln!("Error: <config.toml>: working directory does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(Config::load().unwrap().wdir.unwrap())
        }
    } else {
        eprintln!("Missing option <--wdir/-w>, using '-h' to print help");
        std::process::exit(-1);
    };

    let config_dir = if let Some(cdir) = cdir {
        if !PathBuf::from(&cdir).exists() {
            eprintln!("Error: <--cdir/-c>: config directory does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(cdir)
        }
    } else if Config::load().is_ok() && Config::load().unwrap().cdir.is_some() {
        if !PathBuf::from(&Config::load().unwrap().cdir.unwrap()).exists() {
            eprintln!("Error: <config.toml>: config directory does not exist");
            std::process::exit(-1);
        } else {
            PathBuf::from(Config::load().unwrap().cdir.unwrap())
        }
    } else {
        eprintln!("Missing option <--cdir/-c>, using '-h' to print help");
        std::process::exit(-1);
    };

    (sing_path, working_dir, config_dir)
}

pub fn set_config(setting: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = if let Ok(settings) = Config::load() {
        settings
    } else {
        Config {
            bin: None,
            wdir: None,
            cdir: None,
        }
    };

    if &setting[0] == "bin" {
        settings.bin = Some(setting[1].clone());
    } else if &setting[0] == "wdir" {
        settings.wdir = Some(setting[1].clone());
    } else if &setting[0] == "cdir" {
        settings.cdir = Some(setting[1].clone())
    }

    let data = toml::to_string(&settings)?;

    std::fs::write(
        dirs::config_dir()
            .unwrap()
            .join("singer")
            .join("config.toml"),
        &data,
    )?;

    Ok(())
}
