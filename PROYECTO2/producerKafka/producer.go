package main

import (
	"context"
	"encoding/json"
	"log"
	"net"
	"os"

	pb "go-producer/proto" // Adjust path as needed

	"github.com/segmentio/kafka-go"
	"google.golang.org/grpc"
)

type server struct {
	writer *kafka.Writer
	pb.UnimplementedWeatherServiceServer
}

func (s *server) SendWeather(ctx context.Context, in *pb.WeatherData) (*pb.WeatherData, error) {
	log.Printf("Recibido desde grpc-client: %+v", in)

	// Serializar como JSON para Kafka
	msgBytes, err := json.Marshal(in)
	if err != nil {
		log.Printf("Error al serializar: %v", err)
		return nil, err
	}

	err = s.writer.WriteMessages(context.Background(), kafka.Message{
		Value: msgBytes,
	})
	if err != nil {
		log.Printf("Error al escribir en Kafka: %v", err)
		return nil, err
	}

	log.Println("Mensaje enviado a Kafka correctamente")
	log.Println("✅ Se recibió un paquete desde el cliente gRPC.")
	log.Printf("Contenido del paquete: %+v", in)
	return in, nil
}

func main() {
	// Use environment variables if provided, otherwise use defaults
	kafkaServer := os.Getenv("KAFKA_BOOTSTRAP_SERVERS")
	if kafkaServer == "" {
		kafkaServer = "my-cluster-kafka-bootstrap.weather-tweets:9092"
	}

	kafkaTopic := os.Getenv("KAFKA_TOPIC")
	if kafkaTopic == "" {
		kafkaTopic = "weather-topic"
	}

	log.Printf("Conectando a Kafka en %s, topic %s", kafkaServer, kafkaTopic)

	kafkaWriter := &kafka.Writer{
		Addr:     kafka.TCP(kafkaServer),
		Topic:    kafkaTopic,
		Balancer: &kafka.LeastBytes{},
	}
	defer kafkaWriter.Close()

	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("Error al escuchar: %v", err)
	}

	s := grpc.NewServer()
	pb.RegisterWeatherServiceServer(s, &server{writer: kafkaWriter})

	log.Println("Servidor gRPC del Producer escuchando en :50051")

	if err := s.Serve(lis); err != nil {
		log.Fatalf("Error al iniciar el servidor: %v", err)
	}
}
