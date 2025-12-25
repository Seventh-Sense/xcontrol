#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    async_runtime, AppHandle, Emitter, Manager, WebviewWindow, WindowEvent, Wry
};
use tokio::time::sleep;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use encoding_rs::GBK;
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;
#[cfg(windows)]
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
#[cfg(windows)]
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE};
#[cfg(windows)]
use winapi::um::winuser::{UnregisterClassW, GetClassInfoW};
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::ptr::null_mut;

// --- 配置结构 ---
#[derive(Deserialize, Clone)]
struct ServiceConfig {
    name: String,
    executable: String,
    working_dir: String,
    #[serde(default)]
    debug: bool, // 默认为 false
    #[serde(default)]
    args: Vec<String>, // 默认为空数组
    #[serde(default)]
    health_check: Option<HealthCheckConfig>, // 可选字段
}

#[derive(Deserialize, Clone)]
struct HealthCheckConfig {
    #[serde(default)]
    enabled: bool, // 默认为 false
    #[serde(default)]
    url: String, // 默认为空字符串
    #[serde(default)]
    endpoint: String, // 默认为空字符串
    #[serde(default = "default_max_retries")]
    max_retries: usize,
    #[serde(default = "default_retry_interval")]
    retry_interval_ms: u64,
}

// 为 HealthCheckConfig 实现 Default trait
impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            endpoint: String::new(),
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval(),
        }
    }
}

#[derive(Deserialize)]
struct ServicesConfig {
    services: Vec<ServiceConfig>,
}

fn default_max_retries() -> usize {
    30
}
fn default_retry_interval() -> u64 {
    1000
}

// 全局进程管理器 - 现在只存储服务信息，不存储PID
type ProcessManager = Arc<Mutex<HashMap<String, ServiceInfo>>>;

#[derive(Clone)]
struct ServiceInfo {
    executable: String, // 存储可执行文件名用于清理
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
        PathBuf::from("services.dat"),
        PathBuf::from("./services.dat"),
        PathBuf::from("../services.dat"),
        // 生产环境 - 可执行文件目录及其父目录
        std::env::current_exe()?
            .parent()
            .unwrap()
            .join("services.dat"),
        std::env::current_exe()?
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("services.dat"),
        // Tauri 应用目录
        std::env::current_exe()?
            .parent()
            .unwrap()
            .join("resources")
            .join("services.dat"),
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
        "在所有可能的位置都找不到 services.dat 配置文件。\n当前工作目录: {:?}\n可执行文件路径: {:?}\n尝试的路径: {:#?}",
        current_dir, exe_path, possible_paths
    );

    println!("{}", error_msg);
    Err(error_msg.into())
}

/// 获取指定进程名的所有进程PID
fn get_processes_by_name(
    process_name: &str,
) -> Result<Vec<u32>, Box<dyn std::error::Error + Send + Sync>> {
    let mut pids = Vec::new();

    let mut cmd = Command::new("tasklist");
    cmd.args(&[
        "/FI",
        &format!("IMAGENAME eq {}", process_name),
        "/FO",
        "CSV",
        "/NH",
    ]);

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

    for line in output_str.lines() {
        if line.contains(process_name)
            && !line.contains("找不到任务")
            && !line.contains("INFO: No tasks")
        {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let pid_str = parts[1].trim_matches('"').trim();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }

    Ok(pids)
}

/// 检查并杀死指定名称的进程
fn kill_existing_processes(
    process_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("正在检查系统中是否存在 {} 进程...", process_name);

    let pids = get_processes_by_name(process_name)?;

    if pids.is_empty() {
        println!("未发现运行中的 {} 进程", process_name);
    } else {
        for pid in pids {
            println!("发现已存在的 {} 进程，PID: {}", process_name, pid);
            kill_process_by_pid(pid);
        }
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
    // 非Windows平台实现
    let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
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

    // 如果有参数才设置，避免设置空参数
    if !service.args.is_empty() {
        cmd.args(&service.args);
    }

    cmd.current_dir(&working_dir);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        // 检查debug字段，默认为false（不显示窗口）
        if !service.debug {
            println!("{} 服务将以无窗口模式启动", service.name);
            cmd.creation_flags(CREATE_NO_WINDOW);
        } else {
            println!("{} 服务将显示窗口启动", service.name);
        }
    }

    let child = cmd.spawn()?;
    let pid = child.id();

    // 保存服务信息（不保存PID，因为可能会变化）
    {
        let mut manager = process_manager.lock().unwrap();
        manager.insert(
            service.name.clone(),
            ServiceInfo {
                executable: service.executable.clone(),
            },
        );
    }

    println!("{} 服务进程已启动，PID: {}", service.name, pid);
    Ok(())
}

/// 获取服务的健康检查配置，如果没有配置则返回默认配置
fn get_health_check_config(service: &ServiceConfig) -> HealthCheckConfig {
    service.health_check.clone().unwrap_or_default()
}

/// 健康检查
async fn check_service_health(service: &ServiceConfig) -> bool {
    let health_check = get_health_check_config(service);

    if !health_check.enabled {
        println!("{} 服务未启用健康检查，跳过", service.name);
        return true; // 不需要健康检查的服务直接返回成功
    }

    if health_check.url.is_empty() {
        println!("{} 服务健康检查URL为空，跳过检查", service.name);
        return true;
    }

    let client = reqwest::Client::new();
    let health_check_url = format!("{}{}", health_check.url, health_check.endpoint);

    println!("开始对 {} 服务进行健康检查，URL: {}", service.name, health_check_url);

    for attempt in 1..=health_check.max_retries {
        match client.get(&health_check_url).send().await {
            Ok(response) if response.status().is_success() => {
                println!(
                    "{} 服务已就绪！（尝试 {} / {}）",
                    service.name, attempt, health_check.max_retries
                );
                return true;
            }
            Ok(response) => {
                println!(
                    "{} 服务未就绪，状态码: {}（尝试 {} / {}）",
                    service.name,
                    response.status(),
                    attempt,
                    health_check.max_retries
                );
            }
            Err(e) => {
                println!(
                    "{} 无法连接到服务: {}（尝试 {} / {}）",
                    service.name, e, attempt, health_check.max_retries
                );
            }
        }
        sleep(Duration::from_millis(health_check.retry_interval_ms)).await;
    }

    println!("{} 服务健康检查失败，已达到最大重试次数", service.name);
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

    println!("开始启动 {} 个服务", config.services.len());

    for service in &config.services {
        println!("处理服务: {}", service.name);
        println!("  - 可执行文件: {}", service.executable);
        println!("  - 工作目录: {}", service.working_dir);
        println!("  - 调试模式: {}", service.debug);
        println!("  - 参数: {:?}", service.args);

        // 打印健康检查配置
        let health_check = get_health_check_config(service);
        println!("  - 健康检查: enabled={}, url={}", health_check.enabled, health_check.url);

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
                    let health_check = get_health_check_config(service);
                    let event_data = ServiceEventData {
                        service_name: service.name.clone(),
                        url: health_check.url.clone(),
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

/// 应用退出时的清理函数 - 修改为使用进程名而不是PID
fn cleanup_on_exit(process_manager: ProcessManager) {
    println!("应用正在退出，执行清理操作...");

    // 使用作用域锁，避免长时间持有锁
    let services: Vec<(String, String)> = {
        let manager = process_manager.lock().unwrap();
        manager
            .iter()
            .map(|(name, info)| (name.clone(), info.executable.clone()))
            .collect()
    };

    for (service_name, executable) in services {
        println!("正在查找并终止 {} 服务的所有进程...", service_name);

        // 使用进程名查找所有相关进程并终止
        match get_processes_by_name(&executable) {
            Ok(pids) => {
                if pids.is_empty() {
                    println!("未找到 {} 服务的运行进程", service_name);
                } else {
                    for pid in pids {
                        println!("正在终止 {} 服务进程 (PID: {})...", service_name, pid);
                        kill_process_by_pid(pid);
                    }
                }
            }
            Err(e) => {
                eprintln!("查找 {} 服务进程时出错: {}", service_name, e);
            }
        }
    }

    // 等待进程完全终止
    std::thread::sleep(Duration::from_millis(1000));
    println!("清理操作完成");
}

/// 释放Windows窗口类资源
#[cfg(windows)]
unsafe fn cleanup_window_classes() {
    // 转换类名为宽字符
    let class_name = OsStr::new("Chrome_WidgetWin_0")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();

    let mut wnd_class = std::mem::zeroed();
    // 检查类是否存在
    if GetClassInfoW(null_mut(), class_name.as_ptr(), &mut wnd_class) != 0 {
        // 注销窗口类
        if UnregisterClassW(class_name.as_ptr(), null_mut()) == 0 {
            let err = winapi::um::errhandlingapi::GetLastError();
            if err != 1412 {
                // 忽略"类仍有打开的窗口"错误
                eprintln!("注销窗口类失败，错误码: {}", err);
            }
        } else {
            println!("成功注销 Chrome_WidgetWin_0 窗口类");
        }
    }
}

/// 聚焦并显示现有窗口
fn focus_existing_window(app_handle: &AppHandle<Wry>) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
        println!("已聚焦到现有的应用窗口");
    }
}

/// 安全退出应用
fn safe_exit(app_handle: AppHandle<Wry>, process_manager: ProcessManager) {
    // 1. 立即隐藏窗口
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }

    // 2. 在后台线程执行清理
    std::thread::spawn(move || {
        println!("开始后台清理...");

        // 执行进程清理
        cleanup_on_exit(process_manager);

        // Windows平台额外清理窗口类
        #[cfg(windows)]
        unsafe {
            cleanup_window_classes();
        }

        println!("清理完成，正在退出应用...");

        // 延迟退出，确保资源完全释放
        std::thread::sleep(Duration::from_millis(500));

        // 强制退出（避免Tauri的清理钩子冲突）
        std::process::exit(0);
    });
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
                process_manager.clone(),
            ));

            Ok(())
        })
        // Tauri 2.3.0 要求 on_window_event 闭包接收 (window, event) 两个参数
        .on_window_event(move |window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                println!("收到窗口关闭请求");

                // 阻止默认关闭行为
                api.prevent_close();

                // 获取AppHandle并安全退出
                let app_handle = window.app_handle().clone();
                safe_exit(app_handle, cleanup_manager.clone());

                // 立即隐藏窗口（提升用户体验）
                let _ = window.hide();
            }
            WindowEvent::Destroyed => {
                println!("窗口 {} 已销毁", window.label());
            }
            // 非穷尽变体必须加 ..
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}