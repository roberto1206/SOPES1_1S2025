# Etapa de build
FROM golang:1.24.2 AS builder

WORKDIR /app
COPY . .
RUN go mod tidy && go build -o kafka-consumer

# Etapa de ejecución
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/kafka-consumer .

CMD ["./kafka-consumer"]
