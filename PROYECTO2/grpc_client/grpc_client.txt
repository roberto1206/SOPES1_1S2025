package main

import (
	"context"
	"encoding/json"
	"log"
	"os"
	"time"

	"github.com/IBM/sarama"

	pb "grpc_client/proto" // cambia según tu estructura

	"google.golang.org/grpc"
)

func sendToKafka(data *pb.WeatherData, kafkaAddr string) error {
	log.Println("Conectando a Kafka en:", kafkaAddr) // Reemplaza fmt.Println con log.Println

	config := sarama.NewConfig()
	config.Producer.Return.Successes = true

	producer, err := sarama.NewSyncProducer([]string{kafkaAddr}, config)
	if err != nil {
		return err
	}
	defer producer.Close()

	// Serializar el mensaje como JSON (puedes usar protobuf si prefieres binario)
	msgBytes, err := json.Marshal(data)
	if err != nil {
		return err
	}

	msg := &sarama.ProducerMessage{
		Topic: "weather-topic",
		Value: sarama.ByteEncoder(msgBytes),
	}

	_, _, err = producer.SendMessage(msg)
	return err
}

func sendToGRPCServer(data *pb.WeatherData, address string) error {
	// Verifica si está activada la simulación
	if os.Getenv("SIMULATE") == "true" {
		log.Printf("Simulación activa: datos simuladamente enviados a %s", address)
		return nil
	}

	conn, err := grpc.Dial(address, grpc.WithInsecure())
	if err != nil {
		return err
	}
	defer conn.Close()

	client := pb.NewWeatherServiceClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	res, err := client.SendWeather(ctx, data)
	if err != nil {
		return err
	}

	log.Println("Respuesta desde producer:", res.Status)

	return nil
}
