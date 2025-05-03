FROM golang:1.24-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go build -o producer-rabbit main.go

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/producer-rabbit .
COPY --from=builder /app/proto ./proto
EXPOSE 50051
CMD ["./producer-rabbit"]