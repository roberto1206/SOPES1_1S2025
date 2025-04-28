package main

import (
	"encoding/json"
	"log"
	"net/http"
	"os"

	pb "grpc_client/proto"
)

func handler(w http.ResponseWriter, r *http.Request) {
	var weather pb.WeatherData

	err := json.NewDecoder(r.Body).Decode(&weather)
	if err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	// Obtener direcciones de los servidores gRPC de variables de entorno
	rabbitServer := os.Getenv("RABBIT_SERVER")
	if rabbitServer == "" {
		rabbitServer = "localhost:50051"
	}

	kafkaServer := os.Getenv("KAFKA_SERVER")
	if kafkaServer == "" {
		kafkaServer = "localhost:50052"
	}

	// log.Printf("Recibido: %v - Reenviando a RabbitMQ(%s) y Kafka(%s)", weather, rabbitServer, kafkaServer)

	go func() {
		err := sendToGRPCServer(&weather, rabbitServer)
		if err != nil {
			log.Println("Error enviando a servidor gRPC RabbitMQ:", err)
		} else {
			log.Println("Datos enviados correctamente a RabbitMQ")
		}

		err = sendToGRPCServer(&weather, kafkaServer)
		if err != nil {
			log.Println("Error enviando a servidor gRPC Kafka:", err)
		} else {
			log.Println("Datos enviados correctamente a Kafka")
		}
	}()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Datos meteorológicos recibidos y reenviados"))
}

func main() {
	// Usar puerto 8081 para no colisionar con la API Rust que usa 8080
	port := "8081"

	http.HandleFunc("/input", handler)
	log.Printf("Cliente API REST ejecutándose en puerto :%s", port)
	http.ListenAndServe(":"+port, nil)
}
