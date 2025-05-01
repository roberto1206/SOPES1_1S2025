# Etapa de compilación
FROM rust:latest AS builder

WORKDIR /usr/src/app
COPY . .

# Compilamos la aplicación
RUN cargo build --release

# Etapa de ejecución
FROM debian:bookworm-slim

# Instalamos las dependencias necesarias para runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
# Copiamos el binario desde la etapa de construcción
COPY --from=builder /usr/src/app/target/release/api_rust .

# Exponemos el puerto que utiliza la aplicación
EXPOSE 8080

# Ejecutamos la aplicación
CMD ["./api_rust"]