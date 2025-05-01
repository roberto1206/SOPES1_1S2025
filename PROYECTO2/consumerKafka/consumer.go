package main

import (
	"context"
	"fmt"
	"log"

	"github.com/segmentio/kafka-go"
)

func main() {
	topic := "weather-topic"
	broker := "my-cluster-kafka-bootstrap.weather-tweets:9092"

	reader := kafka.NewReader(kafka.ReaderConfig{
		Brokers: []string{broker},
		Topic:   topic,
		GroupID: "weather-consumer",
	})
	defer reader.Close()

	log.Println("🟢 Consumer escuchando en el topic:", topic)

	for {
		msg, err := reader.ReadMessage(context.Background())
		if err != nil {
			log.Println("Error al leer mensaje:", err)
			continue
		}

		fmt.Println("📦 Mensaje recibido desde Kafka:")
		fmt.Println(string(msg.Value))
	}
}
