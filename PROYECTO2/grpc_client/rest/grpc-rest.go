package main

import (
	"encoding/json"
	"log"
	"net/http"
	"os"

	"github.com/segmentio/kafka-go"
)

type WeatherData struct {
	Temperature float32 `json:"temperature"`
	Humidity    float32 `json:"humidity"`
	City        string  `json:"city"`
}

func handler(kafkaWriter *kafka.Writer) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var data WeatherData
		if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
			http.Error(w, "Invalid request", http.StatusBadRequest)
			return
		}
		jsonData, err := json.Marshal(data)
		if err != nil {
			http.Error(w, "Error serializing data", http.StatusInternalServerError)
			return
		}

		err = kafkaWriter.WriteMessages(r.Context(), kafka.Message{
			Key:   []byte(data.City),
			Value: jsonData,
		})
		if err != nil {
			http.Error(w, "Error sending to Kafka", http.StatusInternalServerError)
			log.Println("Kafka error:", err)
			return
		}

		log.Printf("✅ Enviado a Kafka: %+v", data)
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("Datos enviados a Kafka"))
	}
}

func main() {
	kafkaBroker := os.Getenv("KAFKA_BROKER")
	if kafkaBroker == "" {
		kafkaBroker = "localhost:9092"
	}
	kafkaWriter := kafka.NewWriter(kafka.WriterConfig{
		Brokers:  []string{kafkaBroker},
		Topic:    "weather-topic",
		Balancer: &kafka.LeastBytes{},
	})
	defer kafkaWriter.Close()

	http.HandleFunc("/weather", handler(kafkaWriter))
	log.Println("🌐 Servidor REST escuchando en :5000")
	log.Fatal(http.ListenAndServe(":5000", nil))
}
