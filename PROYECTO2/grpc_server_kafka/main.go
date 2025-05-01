package main

import (
	"context"
	"encoding/json"
	"log"
	"net"
	"os"

	"github.com/segmentio/kafka-go"
	"google.golang.org/grpc"

	pb "grpc_server_kafka/proto" // Asegúrate que la ruta sea correcta
)

type server struct {
	writer *kafka.Writer
	pb.UnimplementedWeatherServiceServer
}

func (s *server) SendWeather(ctx context.Context, req *pb.WeatherData) (*pb.WeatherData, error) {
	jsonData, err := json.Marshal(req)
	if err != nil {
		log.Printf("❌ Error al serializar JSON: %v", err)
		return nil, err
	}

	err = s.writer.WriteMessages(ctx, kafka.Message{
		Key:   []byte("weather"),
		Value: jsonData,
	})
	if err != nil {
		log.Printf("❌ Error al enviar a Kafka: %v", err)
		return nil, err
	}

	log.Printf("✅ Datos enviados a Kafka: %s", jsonData)
	return req, nil
}

func main() {
	port := ":50052"
	broker := os.Getenv("KAFKA_BROKER")
	if broker == "" {
		broker = "localhost:9092"
	}

	writer := &kafka.Writer{
		Addr:     kafka.TCP(broker),
		Topic:    "weather-topic",
		Balancer: &kafka.LeastBytes{},
	}

	lis, err := net.Listen("tcp", port)
	if err != nil {
		log.Fatalf("❌ Fallo al escuchar en el puerto %s: %v", port, err)
	}

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{writer: writer})

	log.Printf("🛰️ Servidor gRPC escuchando en %s", port)
	if err := s.Serve(lis); err != nil {
		log.Fatalf("❌ Error al iniciar servidor gRPC: %v", err)
	}
}
