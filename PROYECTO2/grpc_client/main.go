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
	kafkaServer := os.Getenv("KAFKA_SERVER")
	if kafkaServer == "" {
		kafkaServer = "my-cluster-kafka-bootstrap.weather-tweets:9092"
	}

	// Enviar el mensaje a Kafka
	go func() {
		err := sendToKafka(&weather, kafkaServer)
		if err != nil {
			log.Println("No se pudo enviar a Kafka:", err)
		} else {
			log.Println("Mensaje enviado a Kafka correctamente")
		}
	}()

	// También enviar el mensaje a gRPC
	go func() {
		grpcServerAddress := "producer.weather-tweets.svc.cluster.local:50051"
		err := sendToGRPCServer(&weather, grpcServerAddress)
		if err != nil {
			log.Println("No se pudo enviar a gRPC:", err)
		} else {
			log.Println("Mensaje enviado a gRPC correctamente")
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
