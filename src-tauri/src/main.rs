#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{async_runtime, Emitter, Manager, WebviewWindow};
use tokio::time::sleep;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
#[cfg(windows)]
use winapi::um::winnt::{PROCESS_TERMINATE, PROCESS_QUERY_INFORMATION};
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;
#[cfg(windows)]
use encoding_rs::GBK;

// --- 配置结构 ---
#[derive(Deserialize, Clone)]
struct ServiceConfig {
    name: String,
    executable: String,
    working_dir: String,
    args: Vec<String>,
    health_check: HealthCheckConfig,
}

#[derive(Deserialize, Clone)]
struct HealthCheckConfig {
    enabled: bool,
    #[serde(default)]
    url: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default = "default_max_retries")]
    max_retries: usize,
    #[serde(default = "default_retry_interval")]
    retry_interval_ms: u64,
}

#[derive(Deserialize)]
struct ServicesConfig {
    services: Vec<ServiceConfig>,
}

fn default_max_retries() -> usize { 30 }
fn default_retry_interval() -> u64 { 1000 }

// 全局进程管理器
type ProcessManager = Arc<Mutex<HashMap<String, ProcessInfo>>>;

#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
    name: String,
}

/// 服务状态事件的数据结构
#[derive(Serialize, Clone)]
struct ServiceEventData {
    service_name: String,
    url: String,
    error: String,
    status: String, // "starting", "ready", "error"
}

/// 加载服务配置文件
fn load_services_config() -> Result<ServicesConfig, Box<dyn std::error::Error + Send + Sync>> {
    // 尝试多个可能的配置文件位置
    let possible_paths = vec![
        // 开发环境 - 项目根目录
        PathBuf::from("services.json"),
        PathBuf::from("./services.json"),
        PathBuf::from("../services.json"),
        // 生产环境 - 可执行文件目录及其父目录
        std::env::current_exe()?.parent().unwrap().join("services.json"),
        std::env::current_exe()?.parent().unwrap().parent().unwrap().join("services.json"),
        // Tauri 应用目录
        std::env::current_exe()?.parent().unwrap().join("resources").join("services.json"),
    ];
    
    println!("正在查找配置文件...");
    
    for path in &possible_paths {
        println!("尝试路径: {:?}", path);
        if path.exists() {
            println!("找到配置文件: {:?}", path);
            let config_content = std::fs::read_to_string(path)?;
            let config: ServicesConfig = serde_json::from_str(&config_content)?;
            println!("成功加载配置，包含 {} 个服务", config.services.len());
            return Ok(config);
        }
    }
    
    // 如果找不到配置文件，输出当前工作目录和可执行文件路径用于调试
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("unknown"));
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));
    
    let error_msg = format!(
        "在所有可能的位置都找不到 services.json 配置文件。\n当前工作目录: {:?}\n可执行文件路径: {:?}\n尝试的路径: {:#?}",
        current_dir,
        exe_path,
        possible_paths
    );
    
    println!("{}", error_msg);
    Err(error_msg.into())
}

/// 检查并杀死指定名称的进程
fn kill_existing_processes(process_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("正在检查系统中是否存在 {} 进程...", process_name);

    let mut cmd = Command::new("tasklist");
    cmd.args(&["/FI", &format!("IMAGENAME eq {}", process_name), "/FO", "CSV", "/NH"]);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output()?;

    let output_str = if cfg!(windows) {
        match String::from_utf8(output.stdout.clone()) {
            Ok(s) => s,
            Err(_) => {
                let (decoded, _, _) = GBK.decode(&output.stdout);
                decoded.to_string()
            }
        }
    } else {
        String::from_utf8(output.stdout)?
    };

    let mut found_processes = false;
    
    for line in output_str.lines() {
        if line.contains(process_name) {
            found_processes = true;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let pid_str = parts[1].trim_matches('"').trim();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    println!("发现已存在的 {} 进程，PID: {}", process_name, pid);
                    kill_process_by_pid(pid);
                }
            }
        }
    }

    if !found_processes {
        println!("未发现运行中的 {} 进程", process_name);
    }

    Ok(())
}

/// 通过 PID 杀死进程
#[cfg(windows)]
fn kill_process_by_pid(pid: u32) {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_INFORMATION, 0, pid);
        if handle.is_null() {
            eprintln!("无法打开进程 {} 进行终止", pid);
            return;
        }

        let result = TerminateProcess(handle, 1);
        if result != 0 {
            println!("成功终止进程 {}", pid);
        } else {
            eprintln!("终止进程 {} 失败", pid);
        }

        CloseHandle(handle);
    }
}

#[cfg(not(windows))]
fn kill_process_by_pid(pid: u32) {
    use std::process::Command;
    let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
}

/// 启动单个服务进程
fn spawn_service_process(
    service: &ServiceConfig,
    process_manager: ProcessManager,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("正在启动 {} 服务...", service.name);

    // 清理已存在的同名进程
    if let Err(e) = kill_existing_processes(&service.executable) {
        eprintln!("清理已存在的 {} 进程时出错: {}", service.executable, e);
    }

    // 等待进程完全终止
    std::thread::sleep(Duration::from_millis(1000));

    let exe_path: PathBuf = [&service.working_dir, &service.executable].iter().collect();
    if !exe_path.exists() {
        return Err(format!("{} 不存在于路径: {:?}", service.executable, exe_path).into());
    }

    let working_dir = PathBuf::from(&service.working_dir);

    let mut cmd = Command::new(&exe_path);
    cmd.args(&service.args).current_dir(&working_dir);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd.spawn()?;
    let pid = child.id();

    // 保存进程信息
    {
        let mut manager = process_manager.lock().unwrap();
        manager.insert(service.name.clone(), ProcessInfo {
            pid,
            name: service.name.clone(),
        });
    }

    println!("{} 服务进程已启动，PID: {}", service.name, pid);
    Ok(())
}

/// 健康检查
async fn check_service_health(service: &ServiceConfig) -> bool {
    if !service.health_check.enabled {
        return true; // 不需要健康检查的服务直接返回成功
    }

    let client = reqwest::Client::new();
    let health_check_url = format!("{}{}", service.health_check.url, service.health_check.endpoint);

    for attempt in 1..=service.health_check.max_retries {
        match client.get(&health_check_url).send().await {
            Ok(response) if response.status().is_success() => {
                println!(
                    "{} 服务已就绪！（尝试 {} / {}）",
                    service.name, attempt, service.health_check.max_retries
                );
                return true;
            }
            Ok(response) => {
                println!(
                    "{} 服务未就绪，状态码: {}（尝试 {} / {}）",
                    service.name, response.status(), attempt, service.health_check.max_retries
                );
            }
            Err(e) => {
                println!(
                    "{} 无法连接到服务: {}（尝试 {} / {}）",
                    service.name, e, attempt, service.health_check.max_retries
                );
            }
        }
        sleep(Duration::from_millis(service.health_check.retry_interval_ms)).await;
    }

    false
}

/// 启动所有服务并通知前端
async fn start_all_services_and_notify(window: WebviewWindow, process_manager: ProcessManager) {
    let config = match load_services_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("加载配置文件失败: {}", e);
            let event_data = ServiceEventData {
                service_name: "config".to_string(),
                url: String::new(),
                error: format!("加载配置文件失败: {}", e),
                status: "error".to_string(),
            };
            let _ = window.emit("service_error", event_data);
            return;
        }
    };

    for service in &config.services {
        // 通知前端服务正在启动
        let event_data = ServiceEventData {
            service_name: service.name.clone(),
            url: String::new(),
            error: String::new(),
            status: "starting".to_string(),
        };
        let _ = window.emit("service_starting", event_data);

        // 启动服务进程
        match spawn_service_process(service, process_manager.clone()) {
            Ok(_) => {
                // 等待一小段时间让进程完全启动
                sleep(Duration::from_millis(2000)).await;
                
                // 进行健康检查
                if check_service_health(service).await {
                    let event_data = ServiceEventData {
                        service_name: service.name.clone(),
                        url: service.health_check.url.clone(),
                        error: String::new(),
                        status: "ready".to_string(),
                    };
                    let _ = window.emit("service_ready", event_data);
                } else {
                    let event_data = ServiceEventData {
                        service_name: service.name.clone(),
                        url: String::new(),
                        error: "服务启动超时或健康检查失败".to_string(),
                        status: "error".to_string(),
                    };
                    let _ = window.emit("service_error", event_data);
                }
            }
            Err(e) => {
                eprintln!("启动 {} 服务失败: {}", service.name, e);
                let event_data = ServiceEventData {
                    service_name: service.name.clone(),
                    url: String::new(),
                    error: format!("启动服务失败: {}", e),
                    status: "error".to_string(),
                };
                let _ = window.emit("service_error", event_data);
            }
        }
    }
}

/// 应用退出时的清理函数
fn cleanup_on_exit(process_manager: ProcessManager) {
    println!("应用正在退出，执行清理操作...");
    let manager = process_manager.lock().unwrap();
    for (service_name, process_info) in manager.iter() {
        println!("正在终止 {} 服务进程 (PID: {} {})...", service_name, process_info.name, process_info.pid);
        kill_process_by_pid(process_info.pid);
    }
}

/// 聚焦并显示现有窗口
fn focus_existing_window(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
        println!("已聚焦到现有的应用窗口");
    }
}

fn main() {
    // 创建进程管理器
    let process_manager: ProcessManager = Arc::new(Mutex::new(HashMap::new()));
    let cleanup_manager = process_manager.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("检测到重复启动，聚焦到现有窗口");
            focus_existing_window(app);
        }))
        .setup(move |app| {
            let main_window = app.get_webview_window("main").expect("找不到主窗口");

            // 启动所有服务
            async_runtime::spawn(start_all_services_and_notify(
                main_window,
                process_manager.clone()
            ));

            Ok(())
        })
        .on_window_event(move |_window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    println!("窗口关闭请求");
                    cleanup_on_exit(cleanup_manager.clone());
                }
                tauri::WindowEvent::Destroyed => {
                    println!("窗口已销毁");
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}