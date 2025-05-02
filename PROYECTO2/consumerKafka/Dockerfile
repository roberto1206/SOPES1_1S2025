# --- Stage 1: Build ---
FROM golang:1.24-alpine AS builder

WORKDIR /app
COPY go.mod ./
COPY go.sum ./

RUN go mod download -x  # Descargar dependencias

COPY . .

RUN go build -o consumer .

# --- Stage 2: Final Image ---
FROM alpine:latest
RUN apk --no-cache add ca-certificates

WORKDIR /app
COPY --from=builder /app/consumer /app/consumer

CMD ["./consumer"]