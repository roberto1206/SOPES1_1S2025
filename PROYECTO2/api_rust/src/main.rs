use actix_web::{post, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;

#[derive(Deserialize, Serialize)]
struct WeatherData {
    description: String,
    country: String,
    weather: String,
}

#[post("/input")]
async fn receive_weather(data: web::Json<WeatherData>) -> impl Responder {
    let client = Client::new();

    // Obtiene la URL del servicio Go desde una variable de entorno
    // o usa un valor predeterminado para pruebas locales
    let go_service_url = env::var("GO_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8081/input".to_string());
    
    println!("Enviando datos a: {}", go_service_url);

    // Para pruebas independientes, puedes desactivar el reenvío si está configurado
    if env::var("SKIP_FORWARDING").is_ok() {
        return HttpResponse::Ok().body("Datos recibidos (modo prueba local)");
    }

    // Envía la información al servicio Go
    let res = client.post(go_service_url)
        .json(&*data)
        .send()
        .await;

    match res {
        Ok(_) => HttpResponse::Ok().body("Weather data forwarded!"),
        Err(e) => {
            eprintln!("Error sending to Go service: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to forward weather data")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Iniciando API Rust en 0.0.0.0:8080");
    HttpServer::new(|| {
        App::new()
            .service(receive_weather)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}