FROM golang:1.24.2 AS builder

WORKDIR /app
COPY . .
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux go build -o app

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/app .
# Exponer 50051 para RabbitMQ o 50052 para Kafka según corresponda
EXPOSE 50051
CMD ["./app"]