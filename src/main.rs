use std::process::Command;
use std::time::{Duration, SystemTime};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn red(s: &str) -> String {
    format!("{RED}{BOLD}{s}{RESET}")
}

fn green(s: &str) -> String {
    format!("{GREEN}{BOLD}{s}{RESET}")
}

fn yellow(s: &str) -> String {
    format!("{YELLOW}{s}{RESET}")
}

fn pick<'a>(items: &'a [&str]) -> &'a str {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    items[nanos % items.len()]
}

const KILL_INSULTS: &[&str] = &[
    "Get the fuck off my port.",
    "Fuck off.",
    "You're done, dickhead.",
    "Die, bitch.",
    "Piss off.",
    "Eat shit.",
    "Fuck outta here.",
    "Killed, asshole.",
];

const ESCALATION_INSULTS: &[&str] = &[
    "Still alive? SIGKILL.",
    "SIGKILL, motherfucker.",
    "Just fucking die already.",
    "SIGKILL. Fuck you.",
];

const INVALID_PORT_MSG: &str = "Not a valid port. Must be 1-65535.";

const FREE_PORT_MSG: &str = "Nothing listening. Port is already free.";

const SUCCESS_MSGS: &[&str] = &["Done.", "Dead.", "Gone."];

struct ProcessInfo {
    pid: u32,
    name: String,
}

#[cfg(target_os = "macos")]
fn find_process_on_port(port: u16) -> Vec<ProcessInfo> {
    let output = match Command::new("lsof")
        .args(["-ti", &format!(":{port}"), "-sTCP:LISTEN"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut results = vec![];

    for line in stdout.lines() {
        let pid: u32 = match line.trim().parse() {
            Ok(p) if seen.insert(p) => p,
            _ => continue,
        };

        let name = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(
                        String::from_utf8_lossy(&o.stdout)
                            .trim()
                            .rsplit('/')
                            .next()
                            .unwrap_or("unknown")
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        results.push(ProcessInfo { pid, name });
    }

    results
}

#[cfg(target_os = "linux")]
fn find_process_on_port(port: u16) -> Vec<ProcessInfo> {
    if let Some(results) = try_ss(port)
        && !results.is_empty()
    {
        return results;
    }
    try_fuser(port).unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn try_ss(port: u16) -> Option<Vec<ProcessInfo>> {
    let output = Command::new("ss")
        .args(["-tlnp", "sport", "=", &format!(":{port}")])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut results = vec![];

    for line in stdout.lines().skip(1) {
        if let Some(users_start) = line.find("users:((") {
            let rest = &line[users_start + 8..];

            let name = rest.split('"').nth(1).unwrap_or("unknown").to_string();

            if let Some(pid_start) = rest.find("pid=") {
                let pid_str = &rest[pid_start + 4..];
                let pid_end = pid_str
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(pid_str.len());
                if let Ok(pid) = pid_str[..pid_end].parse::<u32>()
                    && seen.insert(pid)
                {
                    results.push(ProcessInfo { pid, name });
                }
            }
        }
    }

    Some(results)
}

#[cfg(target_os = "linux")]
fn try_fuser(port: u16) -> Option<Vec<ProcessInfo>> {
    let output = Command::new("fuser")
        .args([&format!("{port}/tcp")])
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout} {stderr}");

    let mut seen = std::collections::HashSet::new();
    let mut results = vec![];

    for token in combined.split_whitespace() {
        let cleaned = token.trim_end_matches(|c: char| !c.is_ascii_digit());
        if let Ok(pid) = cleaned.parse::<u32>()
            && seen.insert(pid)
        {
            let name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            results.push(ProcessInfo { pid, name });
        }
    }

    Some(results)
}

fn send_signal(pid: u32, signal: &str) -> Result<(), String> {
    let output = Command::new("kill")
        .args([&format!("-{signal}"), &pid.to_string()])
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_lowercase();
            if stderr.contains("not permitted") || stderr.contains("eperm") {
                Err("permission".into())
            } else if stderr.contains("no such process") || stderr.contains("esrch") {
                Err("dead".into())
            } else {
                Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

fn is_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn kill_process(info: &ProcessInfo, port: u16) -> bool {
    println!(
        "  {} {} (pid {}) on port {}",
        red(pick(KILL_INSULTS)),
        red(&info.name),
        red(&info.pid.to_string()),
        red(&port.to_string()),
    );

    if let Err(e) = send_signal(info.pid, "TERM") {
        if e == "permission" {
            println!(
                "  {} Permission denied. Try: {}",
                red("✗"),
                yellow(&format!("sudo evict {port}"))
            );
        } else if e == "dead" {
            println!("  {} Process already dead.", yellow("⦿"));
        } else {
            println!("  {} Failed to send SIGTERM: {e}", red("✗"));
        }
        return e == "dead";
    }

    std::thread::sleep(Duration::from_millis(500));

    if !is_alive(info.pid) {
        return true;
    }

    println!("  {}", red(pick(ESCALATION_INSULTS)));

    if let Err(e) = send_signal(info.pid, "KILL") {
        if e == "permission" {
            println!(
                "  {} Permission denied. Try: {}",
                red("✗"),
                yellow(&format!("sudo evict {port}"))
            );
        } else {
            println!("  {} Failed to send SIGKILL: {e}", red("✗"));
        }
        return false;
    }

    std::thread::sleep(Duration::from_millis(100));
    true
}

fn handle_port(arg: &str) {
    let port: u16 = match arg.parse() {
        Ok(0) | Err(_) => {
            println!("{} '{}' — {}", red("✗"), arg, red(INVALID_PORT_MSG));
            return;
        }
        Ok(p) => p,
    };

    println!(
        "{} Looking for something to kill on port {}...",
        yellow("⦿"),
        yellow(&port.to_string())
    );

    let processes = find_process_on_port(port);

    if processes.is_empty() {
        println!("{} Port {} {}", green("✓"), port, green(FREE_PORT_MSG));
        return;
    }

    for info in &processes {
        println!(
            "  {} Found {} (pid {}) on port {}",
            yellow("→"),
            yellow(&info.name),
            yellow(&info.pid.to_string()),
            yellow(&port.to_string()),
        );
    }

    let mut killed = 0;
    for info in &processes {
        if kill_process(info, port) {
            killed += 1;
        }
    }

    if killed > 0 {
        println!(
            "{} Port {} is free. {}",
            green("✓"),
            port,
            green(pick(SUCCESS_MSGS))
        );
    }
}

fn print_usage() {
    eprintln!("{BOLD}evict{RESET} — kill whatever the fuck is on a port\n");
    eprintln!("Usage: evict <port> [port ...]\n");
    eprintln!("Examples:");
    eprintln!("  evict 3000");
    eprintln!("  evict 3000 8080 5432");
    eprintln!("  sudo evict 80");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        std::process::exit(if args.is_empty() { 1 } else { 0 });
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("evict {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    for arg in &args {
        handle_port(arg);
    }
}
