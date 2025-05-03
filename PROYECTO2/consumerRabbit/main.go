package main

import (
	"encoding/json"
	"log"
	"os"

	"github.com/streadway/amqp"
)

type WeatherData struct {
	Country     string `json:"country"`
	Weather     string `json:"weather"`
	Description string `json:"description"`
}

func main() {
	rabbitURL := os.Getenv("RABBITMQ_URL")
	if rabbitURL == "" {
		rabbitURL = "amqp://user:Gh62vf3qHqIzFoI3@rabbitmq:5672/"
	}

	conn, err := amqp.Dial(rabbitURL)
	if err != nil {
		log.Fatalf("❌ No se pudo conectar a RabbitMQ: %v", err)
	}
	log.Println("✅ Conectado a RabbitMQ")
	defer conn.Close()

	ch, err := conn.Channel()
	if err != nil {
		log.Fatalf("❌ Error al abrir canal: %v", err)
	}
	log.Println("✅ Canal abierto")
	defer ch.Close()

	_, err = ch.QueueDeclare(
		"weather_queue",
		true,
		false,
		false,
		false,
		nil,
	)
	if err != nil {
		log.Fatalf("❌ Error al declarar cola: %v", err)
	}
	log.Println("✅ Cola declarada: weather_queue")

	msgs, err := ch.Consume(
		"weather_queue",
		"",
		true,
		false,
		false,
		false,
		nil,
	)
	if err != nil {
		log.Fatalf("❌ Error al consumir: %v", err)
	}
	log.Println("✅ Inicio de consumo")

	log.Println("🟢 Escuchando mensajes RabbitMQ...")

	for msg := range msgs {
		var data WeatherData
		err := json.Unmarshal(msg.Body, &data)
		if err != nil {
			log.Printf("❌ JSON inválido: %v", err)
			continue
		}

		log.Printf("📩 Mensaje recibido -> País: %s, Clima: %s, Descripción: %s", data.Country, data.Weather, data.Description)
	}
}
