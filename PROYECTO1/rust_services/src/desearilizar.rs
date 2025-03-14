use std::fs;
use std::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SystemInfo {
    system: SystemStats,
    containers: Vec<ContainerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]  // Mantiene los nombres del JSON
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

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "/proc/sysinfo_202201724";
    let data = fs::read_to_string(file_path)?;
    
    let sys_info: SystemInfo = serde_json::from_str(&data)?;

    println!("=== Información del Sistema ===");
    println!("RAM Total: {} KB", sys_info.system.ram_total);
    println!("RAM Libre: {} KB", sys_info.system.ram_libre);
    println!("RAM Ocupada: {} KB", sys_info.system.ram_ocupada);
    println!("CPU Usada: {}%", sys_info.system.cpu_usada);

    println!("\n=== Información de Contenedores ===");
    for container in sys_info.containers.iter() {
        println!("PID: {}, Nombre: {}", container.pid, container.name);
        println!("  Comando: {}", container.cmdline);
        println!("  Memoria RSS: {} KB", container.memory_rss);
        println!("  CPU: {}%", container.cpu_percent);
        println!("  Uso de Disco: {} KB", container.disk_usage);
        println!("  IO Read: {} bytes, IO Write: {} bytes", container.io_read_bytes, container.io_write_bytes);
        println!("{}", "-".repeat(40));
    }

    Ok(())
}
