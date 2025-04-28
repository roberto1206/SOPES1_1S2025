package main

import (
	"context"
	"log"
	"net"

	pb "grpc_server_rabbit/proto"

	"google.golang.org/grpc"
)

type server struct {
	pb.UnimplementedWeatherServiceServer
}

func (s *server) SendWeather(ctx context.Context, data *pb.WeatherData) (*pb.WeatherData, error) {
	log.Printf("Recibido en servidor RabbitMQ: %v", data)
	// Aquí implementarías la lógica para publicar en RabbitMQ
	return data, nil
}

func main() {
	// Puerto 50051 para el servidor RabbitMQ
	port := ":50051"
	lis, err := net.Listen("tcp", port)
	if err != nil {
		log.Fatalf("Error al iniciar listener: %v", err)
	}

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{})
	log.Printf("Servidor gRPC RabbitMQ escuchando en %s", port)
	if err := s.Serve(lis); err != nil {
		log.Fatalf("Error al iniciar servidor: %v", err)
	}
}
