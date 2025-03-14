use actix_web::{web, App, HttpServer, HttpResponse, Responder, middleware, HttpRequest};

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::AtomicBool;
use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use std::fs;
use std::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RequestLog {
    timestamp: u64,
    method: String,
    path: String,
    status: u16,
    ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemInfo {
    system: SystemStats,
    containers: Vec<ContainerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct SystemStats {
    ram_total: u64,
    ram_libre: u64,
    ram_ocupada: u64,
    cpu_usada: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerInfo {
    pid: u32,
    name: String,
    cmdline: String,
    memory_rss: u64,
    memory_percent: u8,
    virtual_memory: u64,
    cpu_percent: u8,
    disk_usage: u64,
    io_read_bytes: u64,
    io_write_bytes: u64,
}

struct AppState {
    logger_container_id: String,
    shutdown_flag: Arc<AtomicBool>,
    docker: Arc<Docker>,
}

async fn index(req: HttpRequest) -> impl Responder {
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    
    let log = RequestLog {
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        method: req.method().to_string(),
        path: req.path().to_string(),
        status: 200,
        ip,
    };
    
    log::info!("Request recibida: {:?}", log);
    HttpResponse::Ok().body("¡Servicio funcionando!")
}

fn start_logger_container() -> String {
    let output = Command::new("docker")
        .args(&["run", "-d", "--name", "http_request_logger", "-v", "/tmp/http_logs:/logs", "alpine", "sh", "-c", "touch /logs/requests.log && tail -f /dev/null"])
        .output()
        .expect("No se pudo crear el contenedor logger");
    
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

async fn gestionar_contenedores(docker: &Docker) {
    loop {
        let _ = limpiar_contenedores(docker).await;
        sleep(Duration::from_secs(10)).await;
    }
}

async fn limpiar_contenedores(docker: &Docker) {
    let filter = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    let containers = docker.list_containers(Some(filter)).await.unwrap_or(vec![]);
    let mut container_map: HashMap<String, Vec<String>> = HashMap::new();
    
    for container in containers {
        if let Some(names) = &container.names {
            for name in names {
                if let Some(label) = clasificar_contenedor(name) {
                    if let Some(id) = &container.id {
                        container_map.entry(label).or_insert_with(Vec::new).push(id.clone());
                    }
                }
            }
        }
    }
    
    for (_, ids) in container_map.iter_mut() {
        ids.sort();
        while ids.len() > 1 {
            let id = ids.remove(0);
            let _ = docker.remove_container(&id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
        }
    }
}

fn clasificar_contenedor(name: &str) -> Option<String> {
    if name.contains("stress_ram") {
        Some("ram".to_string())
    } else if name.contains("stress_cpu") {
        Some("cpu".to_string())
    } else if name.contains("stress_io") {
        Some("io".to_string())
    } else if name.contains("stress_disk") {
        Some("disk".to_string())
    } else {
        None
    }
}

fn leer_sysinfo() -> Result<SystemInfo, Box<dyn Error>> {
    let file_path = "/proc/sysinfo_202201724";
    let data = fs::read_to_string(file_path)?;
    let sys_info: SystemInfo = serde_json::from_str(&data)?;
    Ok(sys_info)
}

fn eliminar_cronjob() {
    Command::new("crontab")
        .args(&["-r"])
        .output()
        .expect("No se pudo eliminar el cronjob");
    println!("Cronjob eliminado correctamente");
}

fn imprimir_estado_final() {
    if let Ok(sys_info) = leer_sysinfo() {
        println!("=== Información Final del Sistema ===");
        println!("RAM Total: {} KB", sys_info.system.ram_total);
        println!("RAM Libre: {} KB", sys_info.system.ram_libre);
        println!("CPU Usada: {}%", sys_info.system.cpu_usada);
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("Iniciando servicio...");
    
    let container_id = start_logger_container();
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());
    
    let app_state = web::Data::new(AppState {
        logger_container_id: container_id.clone(),
        shutdown_flag: shutdown_flag.clone(),
        docker: docker.clone(),
    });
    
    let docker_clone = Arc::clone(&docker);
    tokio::spawn(async move {
        gestionar_contenedores(&docker_clone).await;
    });
    
    ctrlc::set_handler(move || {
        log::info!("Señal de cierre recibida, finalizando...");
        imprimir_estado_final();
        eliminar_cronjob();
        std::process::exit(0);
    }).expect("Error configurando el manejador de cierre");
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/").to(index))
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await
}
