#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
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

// --- 配置区 ---
//const XNODE_ROOT_DIR: &str = "./xnode";
const XNODE_ROOT_DIR: &str = "F:\\XControl-1.3.1-win\\xnode";
const SERVICE_URL: &str = "http://127.0.0.1:9860";
const HEALTH_CHECK_ENDPOINT: &str = "/";
const MAX_RETRIES: usize = 30;
const RETRY_INTERVAL_MS: u64 = 1000;

// 全局进程管理器
type ProcessManager = Arc<Mutex<Option<ProcessInfo>>>;

#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
}

/// 服务状态事件的数据结构（用于序列化）
#[derive(Serialize, Clone)]
struct ServiceEventData {
    url: String,   // 服务地址（成功时返回）
    error: String, // 错误信息（失败时返回）
}

/// 检查并杀死所有已存在的 xnode 进程
fn kill_existing_xnode_processes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("正在检查系统中是否存在 xnode 进程...");

    let mut cmd = Command::new("tasklist");
    cmd.args(&["/FI", "IMAGENAME eq xnode.exe", "/FO", "CSV", "/NH"]);

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output()?;

     let output_str = if cfg!(windows) {
        // 尝试先用 UTF-8 解码
        match String::from_utf8(output.stdout.clone()) {
            Ok(s) => s,
            Err(_) => {
                // 如果 UTF-8 失败，尝试 GBK 解码
                let (decoded, _, _) = GBK.decode(&output.stdout);
                decoded.to_string()
            }
        }
    } else {
        String::from_utf8(output.stdout)?
    };
    let mut found_processes = false;
    
    for line in output_str.lines() {
        if line.contains("xnode.exe") {
             found_processes = true;
            // 解析CSV格式的输出 "xnode.exe","33540","Console","1","112,400 K"
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let pid_str = parts[1].trim_matches('"').trim();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    println!("发现已存在的 xnode 进程，PID: {}", pid);
                    kill_process_by_pid(pid);
                }
            }
        }
    }

    if !found_processes {
        println!("未发现运行中的 xnode 进程");
    }

    Ok(())
}

/// 通过 PID 杀死进程
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

/// 启动 xnode 服务进程
fn spawn_xnode_process(process_manager: ProcessManager) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("正在启动 xnode 服务...");

    //首先检查并杀死已存在的 xnode 进程
    if let Err(e) = kill_existing_xnode_processes() {
        eprintln!("清理已存在的 xnode 进程时出错: {}", e);
    }

    // 等待一小段时间确保进程完全终止
    std::thread::sleep(Duration::from_millis(2000));

    let xnode_exe_path: PathBuf = [XNODE_ROOT_DIR, "xnode.exe"].iter().collect();
    if !xnode_exe_path.exists() {
        return Err(format!("xnode.exe 不存在于路径: {:?}", xnode_exe_path).into());
    }

    let working_dir = PathBuf::from(XNODE_ROOT_DIR);
    let env_file_path = PathBuf::from(XNODE_ROOT_DIR).join("..").join(".env");
    let env_file_path = env_file_path.canonicalize()?;

    if !env_file_path.exists() {
        return Err(format!(".env 文件不存在于路径: {:?}", env_file_path).into());
    }

    let mut cmd = Command::new(xnode_exe_path);
    cmd.args(&["run", "--env-file", &env_file_path.to_string_lossy()])
        .current_dir(working_dir);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Windows: 隐藏控制台窗口
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd.spawn()?;
    let pid = child.id();

    // 保存进程信息
    {
        let mut manager = process_manager.lock().unwrap();
        *manager = Some(ProcessInfo { pid });
    }

    // Windows 下不需要保持 child 句柄，可以分离进程
    let _ = child;

    println!("xnode 服务进程已启动（后台运行），PID: {}", pid);

    Ok(())
}

/// 杀死 xnode 服务进程
fn kill_xnode_process(process_manager: ProcessManager) {
    let manager = process_manager.lock().unwrap();
    if let Some(process_info) = &*manager {
        let pid = process_info.pid;
        println!("正在终止 xnode 服务进程 (PID: {})...", pid);
        kill_process_by_pid(pid);
    } else {
        println!("没有找到需要终止的 xnode 进程");
    }
}

/// 等待 xnode 服务启动完成，并通过事件通知前端
async fn wait_for_service_and_notify(window: WebviewWindow, process_manager: ProcessManager) {
    match spawn_xnode_process(process_manager.clone()) {
        Ok(_) => {
            let client = reqwest::Client::new();
            let health_check_url = format!("{}{}", SERVICE_URL, HEALTH_CHECK_ENDPOINT);

            println!("正在等待服务在 {} 上就绪...", health_check_url);

            for attempt in 1..=MAX_RETRIES {
                match client.get(&health_check_url).send().await {
                    Ok(response) if response.status().is_success() => {
                        println!(
                            "服务已就绪！（尝试 {} / {} / {}）",
                            attempt,
                            MAX_RETRIES,
                            response.status()
                        );
                        // 服务就绪，发送事件通知前端
                        let event_data = ServiceEventData {
                            url: SERVICE_URL.to_string(),
                            error: String::new(),
                        };
                        let _ = window.emit("service_ready", event_data);
                        return; // 任务完成，退出函数
                    }
                    Ok(response) => {
                        println!(
                            "服务未就绪，状态码: {}（尝试 {} / {}）",
                            response.status(),
                            attempt,
                            MAX_RETRIES
                        );
                    }
                    Err(e) => {
                        println!(
                            "无法连接到服务: {}（尝试 {} / {}）",
                            e, attempt, MAX_RETRIES
                        );
                    }
                }
                sleep(Duration::from_millis(RETRY_INTERVAL_MS)).await;
            }

            // 如果重试次数用完仍未成功，发送失败事件
            eprintln!("服务启动超时。");
            let _ = window.emit("service_error", "服务启动超时，请检查配置。");
        }

        Err(e) => {
            // 如果启动进程本身就失败了，立即发送错误事件
            eprintln!("启动 xnode 服务失败: {}", e);
            let error_data = ServiceEventData {
                url: String::new(),
                error: format!("启动服务失败: {}", e),
            };
            let _ = window.emit("service_error", error_data);
        }
    }
}

/// 应用退出时的清理函数
fn cleanup_on_exit(process_manager: ProcessManager) {
    println!("应用正在退出，执行清理操作...");
    kill_xnode_process(process_manager);
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
    let process_manager: ProcessManager = Arc::new(Mutex::new(None));
    let cleanup_manager = process_manager.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            println!("检测到重复启动，聚焦到现有窗口");
            focus_existing_window(app);
        }))
        .setup(move |app| {
            // 获取主窗口的句柄
            let main_window = app.get_webview_window("main").expect("找不到主窗口");

            // 在 setup 中启动一个后台异步任务
            // 这个任务会独立于主进程运行，不会阻塞窗口显示
            async_runtime::spawn(wait_for_service_and_notify(
                main_window,
                process_manager.clone()
            ));

            Ok(())
        })
        // 注册窗口关闭事件处理器
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