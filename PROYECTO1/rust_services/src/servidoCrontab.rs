use actix_web::{web, App, HttpServer, HttpResponse, Responder, middleware, HttpRequest};
use actix_web::dev::Service; // Necesario para usar call()
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RequestLog {
    timestamp: u64,
    method: String,
    path: String,
    status: u16,
    ip: String,
}

#[allow(dead_code)]
struct AppState {
    logger_container_id: String,
    shutdown_flag: Arc<AtomicBool>,
}

async fn index(req: HttpRequest) -> impl Responder {
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    
    let log = RequestLog {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        method: req.method().to_string(),
        path: req.path().to_string(),
        status: 200,
        ip,
    };
    
    log::info!("Request recibida: {:?}", log);
    
    HttpResponse::Ok().body("¡Servicio funcionando! Esta petición ha sido registrada en el contenedor.")
}

fn start_logger_container() -> String {
    let output = Command::new("docker")
        .args(&[
            "run",
            "-d",
            "--name", "http_request_logger",
            "-v", "/tmp/http_logs:/logs",
            "alpine",
            "sh", "-c", "touch /logs/requests.log && tail -f /dev/null"
        ])
        .output()
        .expect("No se pudo crear el contenedor logger");
    
    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    log::info!("Contenedor logger creado con ID: {}", container_id);
    container_id
}

fn stop_container(container_id: &str) {
    log::info!("Deteniendo el contenedor logger...");
    Command::new("docker")
        .args(&["stop", container_id])
        .output()
        .ok();
    
    Command::new("docker")
        .args(&["rm", "-f", container_id])
        .output()
        .ok();
    
    log::info!("Contenedor logger detenido y eliminado correctamente");
}

fn log_to_container(container_id: &str, log: &RequestLog) {
    let log_json = serde_json::to_string(log).unwrap();
    
    Command::new("docker")
        .args(&[
            "exec",
            container_id,
            "sh", "-c", &format!("echo '{}' >> /logs/requests.log", log_json)
        ])
        .output()
        .expect("No se pudo escribir en el log del contenedor");
}

fn configure_cronjob() {
    let cronjob = "\
* * * * * /bin/bash /home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/sscripts/script.sh
* * * * * sleep 30 && /bin/bash /home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/sscripts/script.sh
";
    
    println!("Actualizando crontab...");
    let mut child = Command::new("crontab")
        .stdin(Stdio::piped())
        .spawn()
        .expect("No se pudo iniciar el proceso crontab");
    
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(cronjob.as_bytes()).expect("Error al escribir en crontab");
    }
    
    let status = child.wait().expect("Error al esperar el proceso crontab");
    
    if status.success() {
        println!("Crontab actualizado correctamente");
    } else {
        println!("Error al actualizar crontab");
    }
    
    println!("Verificando crontab...");
    let output = Command::new("crontab")
        .arg("-l")
        .output()
        .expect("Error al ejecutar crontab -l");
    
    println!("Salida estándar:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("Salida de error:\n{}", String::from_utf8_lossy(&output.stderr));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    log::info!("Iniciando servicio HTTP en puerto 5000");
    
    configure_cronjob();
    
    let container_id = start_logger_container();
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();
    
    let container_id_clone = container_id.clone();
    ctrlc::set_handler(move || {
        log::info!("Señal CTRL+C recibida, iniciando apagado...");
        shutdown_flag_clone.store(true, Ordering::SeqCst);
        stop_container(&container_id_clone);
        std::process::exit(0);
    })
    .expect("Error configurando el manejador de CTRL+C");
    
    let app_state = web::Data::new(AppState {
        logger_container_id: container_id.clone(),
        shutdown_flag: shutdown_flag.clone(),
    });
    
    let container_id_outer = container_id.clone();
    
    HttpServer::new(move || {
        let container_id_inner = container_id_outer.clone();
        
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .service(web::resource("/").to(index))
            .default_service(web::route().to(index))
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await
}
