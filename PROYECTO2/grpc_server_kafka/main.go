package main

import (
	"context"
	"log"
	"net"

	pb "grpc_server_kafka/proto"

	"google.golang.org/grpc"
)

type server struct {
	pb.UnimplementedWeatherServiceServer
}

func (s *server) SendWeather(ctx context.Context, data *pb.WeatherData) (*pb.WeatherData, error) {
	log.Printf("Recibido en servidor Kafka: %v", data)
	// Aquí implementarías la lógica para publicar en Kafka
	return data, nil
}

func main() {
	// Puerto 50052 para el servidor Kafka
	port := ":50052"
	lis, err := net.Listen("tcp", port)
	if err != nil {
		log.Fatalf("Error al iniciar listener: %v", err)
	}

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{})
	log.Printf("Servidor gRPC Kafka escuchando en %s", port)
	if err := s.Serve(lis); err != nil {
		log.Fatalf("Error al iniciar servidor: %v", err)
	}
}
