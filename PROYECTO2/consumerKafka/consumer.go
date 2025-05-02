package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/redis/go-redis/v9"
	"github.com/segmentio/kafka-go"
)

type WeatherData struct {
	Description string `json:"description"`
	Country     string `json:"country"`
	Weather     string `json:"weather"`
}

func main() {
	ctx := context.Background()

	// Config Redis
	redisHost := os.Getenv("REDIS_HOST")
	redisPassword := os.Getenv("REDIS_PASSWORD")
	rdb := redis.NewClient(&redis.Options{
		Addr:     redisHost, // "redis-service:6379"
		Password: redisPassword,
		DB:       0,
	})

	// Config Kafka
	topic := "weather-topic"
	broker := "my-cluster-kafka-bootstrap:9092"
	r := kafka.NewReader(kafka.ReaderConfig{
		Brokers:  []string{broker},
		Topic:    topic,
		GroupID:  "weather-consumer-group",
		MinBytes: 10e3,
		MaxBytes: 10e6,
	})

	log.Printf("🟢 Escuchando en el topic: %s", topic)

	for {
		m, err := r.ReadMessage(ctx)
		if err != nil {
			log.Printf("❌ Error al leer mensaje Kafka: %v", err)
			continue
		}

		log.Printf("📦 Kafka JSON: %s", string(m.Value))

		var data WeatherData
		if err := json.Unmarshal(m.Value, &data); err != nil {
			log.Printf("❌ JSON inválido: %v", err)
			continue
		}

		// Guardar datos en Redis Hash
		timestamp := time.Now().Format("20060102_150405.000")
		key := fmt.Sprintf("weather:%s", timestamp)
		_, err = rdb.HSet(ctx, key,
			"country", data.Country,
			"weather", data.Weather,
			"description", data.Description,
			"timestamp", timestamp,
		).Result()
		if err != nil {
			log.Fatalf("❌ Error al guardar en Redis: %v", err)
			return
		}

		log.Printf("✅ Guardado en Redis: %s", key)

		// Contador incremental
		count, err := rdb.Incr(ctx, "weather:count").Result()
		if err != nil {
			log.Printf("❌ Error contador Redis: %v", err)
			continue
		}

		// Agregar al Sorted Set con timestamp (X = tiempo, Y = count)
		unixTime := float64(time.Now().UnixNano()) / 1e9
		_, err = rdb.ZAdd(ctx, "weather_counter_timeline", redis.Z{
			Score:  float64(count),
			Member: fmt.Sprintf("%.0f", unixTime*1000), // Convertir a milisegundos
		}).Result()
		if err != nil {
			log.Printf("❌ Error al agregar al Sorted Set: %v", err)
		} else {
			log.Printf("📈 Guardado en timeline: %d @ %.3f", count, unixTime)
		}
	}
}
