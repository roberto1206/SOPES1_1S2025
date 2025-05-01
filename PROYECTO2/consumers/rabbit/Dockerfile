# ====================
# RabbitMQ Consumer Dockerfile
# ====================
FROM golang:1.24.2 AS builder

WORKDIR /app

COPY rabbit-consumer.go .

RUN go mod init rabbit-consumer && \
    go get github.com/streadway/amqp && \
    go build -o rabbit-consumer

CMD ["./rabbitmq-consumer"]