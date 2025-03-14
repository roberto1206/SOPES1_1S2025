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
use ctrlc;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use actix_files::Files;  // Importar actix_files


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

#[derive(Debug, Serialize, Deserialize)]
struct ContainerLog {
    timestamp: u64,  // Keep this as u64, but we'll convert when needed
    category: String,
    name: String,
    action: String,
}

#[allow(dead_code)]
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
        .args(&["run", "-d", "--name", "http_request_logger", "-v", "/tmp/http_logs:/logs", "alpine", "sh", "-c", "touch /logs/requests.log && touch /logs/container_logs.json && tail -f /dev/null"])
        .output()
        .expect("No se pudo crear el contenedor logger");
    
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

async fn gestionar_contenedores(docker: &Docker, logger_container_id: &str) {
    loop {
        let _ = gestionar_contenedores_por_categoria(docker, logger_container_id).await;
        sleep(Duration::from_secs(10)).await;
    }
}

async fn gestionar_contenedores_por_categoria(docker: &Docker, logger_container_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Leer información del sistema

    if let Ok(sys_info) = leer_sysinfo() {
        log::info!("=== Información del Sistema ===");
        log::info!("RAM Total: {} KB", sys_info.system.ram_total);
        log::info!("RAM Libre: {} KB", sys_info.system.ram_libre);
        log::info!("RAM Ocupada: {} KB", sys_info.system.ram_ocupada);
        log::info!("CPU Usada: {}%", sys_info.system.cpu_usada);
        let cpu_json = sys_info.system.cpu_usada;
        guardar_cpu_info(cpu_json);
        
        // Imprimir en consola de manera estilizada
        println!("\n╔═════════════════════════════════════════╗");
        println!("║           INFORMACIÓN DEL SISTEMA        ║");
        println!("╠═════════════════════════════════════════╣");
        println!("║ RAM Total:   {:10} KB              ║", sys_info.system.ram_total);
        println!("║ RAM Libre:   {:10} KB              ║", sys_info.system.ram_libre);
        println!("║ RAM Ocupada: {:10} KB              ║", sys_info.system.ram_ocupada);
        println!("║ CPU Usada:   {:10}%                ║", sys_info.system.cpu_usada);
        println!("╚═════════════════════════════════════════╝\n");
    }

    let filter = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    
    let containers = docker.list_containers(Some(filter)).await?;
    
    // Group containers by category
    let mut container_categories: HashMap<String, Vec<(String, String, i64)>> = HashMap::new();
    
    for container in containers {
        if let (Some(names), Some(id), Some(created)) = (&container.names, &container.id, container.created) {
            for name in names {
                if let Some(category) = clasificar_contenedor(name) {
                    // Store id, name and creation time
                    container_categories.entry(category).or_default().push((id.clone(), name.clone(), created));
                }
            }
        }
    }
    
    // Keep only the newest container of each type
    let mut removed_containers = Vec::new();
    
    for (category, mut containers) in container_categories.iter_mut() {
        // Sort by creation time (newest first)
        containers.sort_by(|a, b| b.2.cmp(&a.2));
        
        // Keep the first one (newest), remove the rest
        while containers.len() > 1 {
            let (id, name, created) = containers.pop().unwrap();
            
            // Skip the logger container
            if id == logger_container_id {
                continue;
            }
            
            log::info!("Eliminando contenedor de categoria {}: {} ({})", category, name, id);
            
            // Log container deletion with timestamp
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            
            let container_log = ContainerLog {
                timestamp,
                category: category.clone(),
                name: name.clone(),
                action: "eliminado".to_string(),
            };
            
            removed_containers.push(container_log);
            
            let _ = docker.remove_container(&id, Some(RemoveContainerOptions { 
                force: true, 
                ..Default::default() 
            })).await;
        }
    }
    
    // Print grouped containers
    println!("╔═════════════════════════════════════════╗");
    println!("║     CONTENEDORES ACTIVOS POR CATEGORÍA  ║");
    println!("╠═════════════════════════════════════════╣");
    
    for (category, containers) in &container_categories {
        println!("║ Categoría: {:<30} ║", category);
        for (id, name, created) in containers {
            // Here's where you need the safe conversion
            let created_time = if *created >= 0 {
                SystemTime::UNIX_EPOCH + Duration::from_secs(*created as u64)
            } else {
                SystemTime::UNIX_EPOCH // Use epoch time as default for negative values
            };
            
            let created_str = format!("{:?}", created_time);
            println!("║  - {}: {} (Creado: {}) ║", &id[0..12], name, created_str);
        }
        println!("╠═════════════════════════════════════════╣");
    }
    
    println!("╚═════════════════════════════════════════╝\n");
    
    // Log removed containers
    if !removed_containers.is_empty() {
        println!("╔═════════════════════════════════════════╗");
        println!("║          CONTENEDORES ELIMINADOS        ║");
        println!("╠═════════════════════════════════════════╣");
        
        for log in &removed_containers {
            println!("║ {} - {} - {} ║", log.category, log.name, log.timestamp);
        }
        
        println!("╚═════════════════════════════════════════╝\n");
        
        // Send logs to the logger container
        let logs_json = serde_json::to_string(&removed_containers)?;
        Command::new("docker")
            .args(&["exec", "http_request_logger", "sh", "-c", &format!("echo '{}' >> /logs/container_logs.json", logs_json)])
            .output()?;
    }
    
    Ok(())
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
    let file_path = "/proc/sysinfo_202201724";  // Ajusta el archivo según sea necesario
    let data = fs::read_to_string(file_path)?;
    let sys_info: SystemInfo = serde_json::from_str(&data)?;
    Ok(sys_info)
}

fn eliminar_cronjob() {
    // Eliminar el cronjob de crontab
    let output = Command::new("crontab")
        .args(&["-r"])
        .output()
        .expect("No se pudo eliminar el cronjob");

    // Comprobar si hubo algún error
    if !output.status.success() {
        eprintln!("Error al eliminar el cronjob: {}", String::from_utf8_lossy(&output.stderr));
    } else {
        println!("Cronjob eliminado correctamente");
    }
}

fn imprimir_estado_final() {
    if let Ok(sys_info) = leer_sysinfo() {
        println!("╔═════════════════════════════════════════╗");
        println!("║       INFORMACIÓN FINAL DEL SISTEMA     ║");
        println!("╠═════════════════════════════════════════╣");
        println!("║ RAM Total:   {:10} KB              ║", sys_info.system.ram_total);
        println!("║ RAM Libre:   {:10} KB              ║", sys_info.system.ram_libre);
        println!("║ RAM Ocupada: {:10} KB              ║", sys_info.system.ram_ocupada);
        println!("║ CPU Usada:   {:10}%                ║", sys_info.system.cpu_usada);
        println!("╚═════════════════════════════════════════╝\n");
    }
}

async fn get_logs() -> impl Responder {
    let log_file_path = "/tmp/http_logs/requests.log"; // Ruta de los logs
    let log_data = fs::read_to_string(log_file_path)
        .unwrap_or_else(|_| "No se pudieron obtener los logs.".to_string());

    HttpResponse::Ok().body(log_data)
}

async fn generate_graphs() -> impl Responder {
    // Aquí se implementaría la generación de gráficas
    // Por ahora, solo retornamos un mensaje de éxito
    HttpResponse::Ok().body("Gráficas generadas correctamente")
}

fn configurar_cronjob() {
    // Componentes del cronjob
    let bash_interpreter = "/bin/bash";
    let script_path = "/home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/sscripts/script.sh";

    // Construir el comando crontab con ambas líneas
    let cron_command = format!(
        "(crontab -l 2>/dev/null; echo \"* * * * * {} {}\"; echo \"* * * * * sleep 30 && {} {}\") | crontab -",
        bash_interpreter, script_path, bash_interpreter, script_path
    );

    // Ejecutar el comando
    let output = Command::new("bash")
        .arg("-c")
        .arg(&cron_command)
        .output()
        .expect("No se pudo configurar el cronjob");

    // Verificar el resultado
    if !output.status.success() {
        eprintln!("Error al configurar el cronjob: {}", String::from_utf8_lossy(&output.stderr));
    } else {
        println!("Cronjob configurado correctamente");
    }
}

fn guardar_cpu_info(cpu_usada: u8) {
    let file_path = "/home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/rust_services/graficas/cpu.json";
    let path = Path::new(file_path);
    let mut data = vec![];
    
    // Asegúrate de que el directorio exista
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("No se pudo crear el directorio");
        }
    }

    // Leer los datos existentes si el archivo ya existe
    if path.exists() {
        let mut file = fs::File::open(file_path).expect("No se pudo abrir el archivo JSON");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("No se pudo leer el archivo JSON");

        if !contents.trim().is_empty() {
            data = serde_json::from_str(&contents).unwrap_or(vec![]);
        }
    }
    
    let nuevo_registro = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "cpu_usada": cpu_usada
    });
    
    data.push(nuevo_registro);
    
    // Abrir el archivo y escribir los datos
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(file_path)
        .expect("No se pudo abrir el archivo JSON para escritura");
    
    let json_data = serde_json::to_string_pretty(&data).expect("Error serializando JSON");
    file.write_all(json_data.as_bytes()).expect("Error escribiendo en el archivo JSON");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("Iniciando servicio...");

    // Configurar el cronjob
    configurar_cronjob();

    // Configura y ejecuta el contenedor de logs
    let container_id = start_logger_container();
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let docker = Arc::new(Docker::connect_with_local_defaults().unwrap());

    let app_state = web::Data::new(AppState {
        logger_container_id: container_id.clone(),
        shutdown_flag: shutdown_flag.clone(),
        docker: docker.clone(),
    });

    // Iniciar el monitoreo de contenedores
    let docker_clone = docker.clone();
    let container_id_clone = container_id.clone();
    let _monitoring_task = tokio::spawn(async move {
        gestionar_contenedores(&docker_clone, &container_id_clone).await;
    });

    // Maneja la señal de cierre
    let container_id_clone = container_id.clone();
    ctrlc::set_handler(move || {
        log::info!("Señal de cierre recibida, finalizando...");
        
        // Imprimir información final
        imprimir_estado_final();
        
        // Enviar petición final al contenedor de logs para generar gráficas
        let output = Command::new("curl")
            .args(&["-X", "POST", "http://localhost:5000/logs/generate_graphs"])
            .output()
            .expect("No se pudo enviar la petición final para generar gráficas");
        
        println!("Respuesta del contenedor de logs: {}", String::from_utf8_lossy(&output.stdout));
        
        // Eliminar el cronjob
        eliminar_cronjob();
        
        std::process::exit(0);
    }).expect("Error configurando el manejador de cierre");

    // Iniciar el servidor HTTP
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/").to(index))
            .service(web::resource("/logs").to(get_logs)) // Ruta para obtener logs
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await
}