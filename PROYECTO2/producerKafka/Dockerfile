# Start from the official Golang image
FROM golang:1.24.2 AS builder

# Set working directory
WORKDIR /app

# Copy go.mod and go.sum files first for better caching
COPY go.mod go.sum* ./

# Download dependencies
RUN go mod download

# Copy the source code, including proto directory
COPY . .

# Build the application with optimizations
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-w -s" -o producer .

# Use a minimal alpine image for the final container
FROM alpine:3.19

# Add CA certificates for secure connections
RUN apk --no-cache add ca-certificates tzdata

# Set working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/producer .

# Create a non-root user to run the application
RUN adduser -D appuser
USER appuser

# Set environment variables (these can be overridden at runtime)
ENV KAFKA_TOPIC="weather-topic" \
    KAFKA_BOOTSTRAP_SERVERS="kafka-cluster-kafka-bootstrap.kafka-system:9092"

# Expose the gRPC port
EXPOSE 50051

# Run the application
ENTRYPOINT ["./producer"]

# Document that the service listens on port 50051 (gRPC)
LABEL description="gRPC server that produces weather data to Kafka"