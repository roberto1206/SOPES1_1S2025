package main

import (
	"context"
	"encoding/json"
	"log"
	"net"
	"os"
	"os/signal"
	"syscall"
	"time"

	pb "go-producer/proto" // Ajusta la ruta según sea necesario

	"github.com/segmentio/kafka-go"
	"github.com/segmentio/kafka-go/compress"
	"google.golang.org/grpc"
	"google.golang.org/grpc/keepalive"
)

type server struct {
	writer *kafka.Writer
	pb.UnimplementedWeatherServiceServer
}

func (s *server) SendWeather(ctx context.Context, in *pb.WeatherData) (*pb.WeatherResponse, error) {
	log.Printf("📥 Recibido desde grpc-client: %+v", in)

	// Generar un ID de mensaje único
	messageID := time.Now().Format("20060102150405.000") + "-" + in.Country

	// Serializar como JSON para Kafka
	msgBytes, err := json.Marshal(in)
	if err != nil {
		log.Printf("❌ Error al serializar: %v", err)
		return &pb.WeatherResponse{Status: "error"}, err
	}

	// Crear un context con timeout para la escritura
	writeCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	// Escribir mensaje con reintentos
	err = s.writer.WriteMessages(writeCtx, kafka.Message{
		Key:   []byte(messageID), // Usar ID único como clave para mejor particionamiento
		Value: msgBytes,
		Time:  time.Now(),
		// Headers pueden usarse para metadatos adicionales
		Headers: []kafka.Header{
			{Key: "source", Value: []byte("grpc-producer")},
			{Key: "message_id", Value: []byte(messageID)},
		},
	})
	if err != nil {
		log.Printf("❌ Error al escribir en Kafka: %v", err)
		return &pb.WeatherResponse{Status: "error: " + err.Error()}, err
	}

	log.Println("✅ Mensaje enviado a Kafka correctamente")
	log.Printf("📤 Contenido del paquete: %+v", in)
	return &pb.WeatherResponse{Status: "success"}, nil
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.Println("🚀 Iniciando servidor productor...")

	// Usar variables de entorno o valores predeterminados
	kafkaServer := getEnvWithDefault("KAFKA_BOOTSTRAP_SERVERS", "my-cluster-kafka-bootstrap.weather-tweets:9092")
	kafkaTopic := getEnvWithDefault("KAFKA_TOPIC", "weather-topic")

	log.Printf("🔌 Conectando a Kafka en %s, topic %s", kafkaServer, kafkaTopic)

	// Configurar escritor de Kafka con opciones mejoradas
	kafkaWriter := &kafka.Writer{
		Addr:         kafka.TCP(kafkaServer),
		Topic:        kafkaTopic,
		Balancer:     &kafka.LeastBytes{},
		RequiredAcks: kafka.RequireAll, // Esperar confirmación de todos los replicas
		Async:        false,            // Modo síncrono para garantizar entrega
		// Reintentos y timeout
		WriteTimeout: 10 * time.Second,
		ReadTimeout:  10 * time.Second,
		MaxAttempts:  5,
		// Compresión para mejorar rendimiento
		Compression: compress.Snappy,
		// Configuración de lotes
		BatchSize:    100,
		BatchBytes:   1024 * 1024, // 1MB
		BatchTimeout: 200 * time.Millisecond,
	}
	defer kafkaWriter.Close()

	// Canal para señales de terminación
	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGINT, syscall.SIGTERM)

	// Opciones de gRPC mejoradas
	grpcOpts := []grpc.ServerOption{
		grpc.KeepaliveParams(keepalive.ServerParameters{
			MaxConnectionIdle:     15 * time.Minute,
			MaxConnectionAge:      30 * time.Minute,
			MaxConnectionAgeGrace: 5 * time.Minute,
			Time:                  5 * time.Minute,
			Timeout:               1 * time.Minute,
		}),
	}

	// Iniciar servidor gRPC
	lis, err := net.Listen("tcp", ":50051")
	if err != nil {
		log.Fatalf("❌ Error al escuchar: %v", err)
	}

	s := grpc.NewServer(grpcOpts...)
	pb.RegisterWeatherServiceServer(s, &server{writer: kafkaWriter})

	// Iniciar servidor en goroutine
	go func() {
		log.Println("🟢 Servidor gRPC del Producer escuchando en :50051")
		if err := s.Serve(lis); err != nil {
			log.Fatalf("❌ Error al iniciar el servidor: %v", err)
		}
	}()

	// Esperar señal de terminación
	sig := <-signals
	log.Printf("📨 Señal recibida: %v, cerrando servidor...", sig)

	// Terminar limpiamente
	s.GracefulStop()
	log.Println("👋 Servidor finalizado")
}

// Obtener variable de entorno con valor predeterminado
func getEnvWithDefault(key, defaultValue string) string {
	value := os.Getenv(key)
	if value == "" {
		return defaultValue
	}
	return value
}
