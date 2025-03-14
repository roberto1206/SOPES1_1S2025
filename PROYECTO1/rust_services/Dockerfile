# Usa una imagen base de Rust para compilar la aplicación
FROM rust:1.73 as builder

# Establece el directorio de trabajo dentro del contenedor
WORKDIR /usr/src/app

# Copia todo el contenido del proyecto al contenedor
COPY . .

# Compila la aplicación en modo release
RUN cargo build --release

# Usa una imagen base ligera para la etapa de ejecución
FROM debian:buster-slim

# Instala las dependencias necesarias para ejecutar la aplicación
RUN apt-get update && apt-get install -y \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

# Copia el binario compilado desde la etapa de compilación
COPY --from=builder /usr/src/app/target/release/rust_services /usr/local/bin/rust_services

# Expone el puerto en el que la aplicación escucha
EXPOSE 5000

# Define el comando para ejecutar la aplicación
CMD ["rust_services"]