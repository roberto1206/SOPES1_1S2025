use bollard::Docker;
use bollard::container::{ListContainersOptions, RemoveContainerOptions, CreateContainerOptions, StartContainerOptions, Config};
use bollard::models::ContainerSummary;
use std::collections::HashMap;
use chrono::Utc;
use futures::future::join_all;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_local_defaults().unwrap();

    let container_types: HashMap<String, &str> = [
        ("ram".to_string(), "--vm 1 --vm-bytes 64M -t 30s"),
        ("cpu".to_string(), "--cpu 2 -t 30s"),
        ("io".to_string(), "--io 1 -t 30s"),
        ("disk".to_string(), "--hdd 1 --hdd-bytes 100M -t 30s"),
    ].iter().cloned().collect();

    let existing_containers = list_running_containers(&docker).await;
    let mut latest_containers: HashMap<String, String> = HashMap::new();

    // Primero, identifica los contenedores más recientes para cada tipo
    for (label, containers) in &existing_containers {
        if !containers.is_empty() {
            let mut sorted_containers = containers.clone();
            sorted_containers.sort_by(|a, b| a.created.cmp(&b.created));
            
            // Guarda el ID del contenedor más reciente
            if let Some(latest) = sorted_containers.last() {
                if let Some(id) = &latest.id {
                    latest_containers.insert(label.clone(), id.clone());
                }
            }
        }
    }

    // Ahora elimina los contenedores antiguos (todos excepto el más reciente)
    let mut removal_tasks = vec![];
    for (label, containers) in &existing_containers {
        for container in containers {
            if let Some(id) = &container.id {
                // Si este contenedor no es el más reciente de su tipo, elimínalo
                if latest_containers.get(label) != Some(id) {
                    println!(
                        "🗑 Eliminando contenedor antiguo de tipo {}: {}",
                        label,
                        id
                    );
                    removal_tasks.push(remove_container(&docker, id));
                } else {
                    println!(
                        "✅ Manteniendo contenedor más reciente de tipo {}: {}",
                        label,
                        id
                    );
                }
            }
        }
    }

    // Espera a que se completen todas las tareas de eliminación
    join_all(removal_tasks).await;

    // No crear nuevos contenedores, solo mantener los existentes
    println!("✅ Gestión de contenedores completada. Manteniendo los siguientes contenedores:");
    for (label, id) in &latest_containers {
        println!("  - Tipo: {}, ID: {}", label, id);
    }
}

async fn list_running_containers(docker: &Docker) -> HashMap<String, Vec<ContainerSummary>> {
    let mut containers_map: HashMap<String, Vec<ContainerSummary>> = HashMap::new();
    let filter = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };

    let containers = docker.list_containers(Some(filter)).await.unwrap();

    for container in containers {
        if let Some(names) = &container.names {
            for name in names {
                if let Some(label) = extract_label_from_name(name) {
                    containers_map.entry(label.to_string()).or_insert_with(Vec::new).push(container.clone());
                }
            }
        }
    }
    containers_map
}

fn extract_label_from_name(name: &str) -> Option<&str> {
    if name.contains("stress_ram") {
        Some("ram")
    } else if name.contains("stress_cpu") {
        Some("cpu")
    } else if name.contains("stress_io") {
        Some("io")
    } else if name.contains("stress_disk") {
        Some("disk")
    } else {
        None
    }
}

async fn remove_container(docker: &Docker, id: &str) -> Result<(), bollard::errors::Error> {
    let options = Some(RemoveContainerOptions { 
        force: true, 
        ..Default::default() 
    });
    
    match docker.remove_container(id, options).await {
        Ok(_) => Ok(()),
        Err(e) => {
            // Si el error es que el contenedor ya está siendo eliminado, no es un error fatal
            if e.to_string().contains("is already in progress") {
                println!("ℹ️ El contenedor {} ya está siendo eliminado", id);
                Ok(())
            } else {
                // Otro tipo de error, lo reportamos pero no lo consideramos fatal
                println!("⚠️ Error al eliminar contenedor {}: {}", id, e);
                Ok(())
            }
        }
    }
}

async fn create_container(docker: &Docker, label: &str, options: &str) {
    let container_name = format!("stress_{}_{}", label, Utc::now().timestamp());

    let config = Config {
        image: Some("containerstack/alpine-stress"),
        cmd: Some(vec!["stress", options]),
        ..Default::default()
    };

    println!("🚀 Creando contenedor: {} con opciones: {}", container_name, options);

    match docker.create_container(Some(CreateContainerOptions { name: container_name.as_str(), platform: None }), config).await {
        Ok(_) => {
            match docker.start_container(container_name.as_str(), Some(StartContainerOptions::<String>::default())).await {
                Ok(_) => println!("✅ Contenedor {} iniciado correctamente", container_name),
                Err(e) => println!("⚠️ Error al iniciar contenedor {}: {}", container_name, e),
            }
        },
        Err(e) => println!("⚠️ Error al crear contenedor {}: {}", container_name, e),
    }
}