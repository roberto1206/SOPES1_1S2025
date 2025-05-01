package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/segmentio/kafka-go"
)

func main() {
	broker := os.Getenv("KAFKA_BROKER")
	if broker == "" {
		broker = "my-cluster-kafka-bootstrap.kafka:9092"
	}

	topic := os.Getenv("KAFKA_TOPIC")
	if topic == "" {
		topic = "weather-topic"
	}

	groupID := os.Getenv("KAFKA_GROUP")
	if groupID == "" {
		groupID = "weather-group"
	}

	r := kafka.NewReader(kafka.ReaderConfig{
		Brokers: []string{broker},
		Topic:   topic,
		GroupID: groupID,
	})
	defer r.Close()

	fmt.Println("✅ Consumidor Kafka iniciado...")

	// Manejar Ctrl+C para salir limpiamente
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		for {
			m, err := r.ReadMessage(context.Background())
			if err != nil {
				log.Printf("❌ Error leyendo mensaje: %v", err)
				continue
			}
			fmt.Printf("📥 Mensaje recibido: %s\n", string(m.Value))
		}
	}()

	<-sig
	fmt.Println("🛑 Finalizando consumidor Kafka...")
}
