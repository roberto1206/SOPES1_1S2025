package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	pb "grpc_client/proto" // Asegúrate de que esta ruta es correcta según tu estructura

	"github.com/streadway/amqp"
	"google.golang.org/grpc"
)

func main() {
	port := "8081"
	http.HandleFunc("/input", handler)
	log.Printf("Cliente API REST ejecutándose en puerto :%s", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}

func handler(w http.ResponseWriter, r *http.Request) {
	var weather pb.WeatherData

	err := json.NewDecoder(r.Body).Decode(&weather)
	if err != nil {
		http.Error(w, "Solicitud inválida", http.StatusBadRequest)
		return
	}

	go func() {
		grpcServerAddress := "producer.tweets.svc.cluster.local:50051"
		err := sendToGRPCServer(&weather, grpcServerAddress)
		if err != nil {
			log.Println("❌ No se pudo enviar a gRPC:", err)
		} else {
			log.Println("✅ Mensaje enviado a gRPC correctamente")
		}

		// Después de enviar los datos por gRPC, enviarlos a RabbitMQ
		err = sendToRabbitMQ(&weather)
		if err != nil {
			log.Println("❌ No se pudo enviar a RabbitMQ:", err)
		} else {
			log.Println("✅ Mensaje enviado a RabbitMQ correctamente")
		}
	}()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Datos meteorológicos reenviados por gRPC y RabbitMQ"))
}

func sendToGRPCServer(data *pb.WeatherData, address string) error {
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
	log.Println("📬 Respuesta desde producer:", res.Status)
	return nil
}

func sendToRabbitMQ(data *pb.WeatherData) error {
	conn, err := amqp.Dial("amqp://user:Gh62vf3qHqIzFoI3@rabbitmq:5672/")
	if err != nil {
		return fmt.Errorf("No se pudo conectar a RabbitMQ: %v", err)
	}
	defer conn.Close()

	ch, err := conn.Channel()
	if err != nil {
		return fmt.Errorf("No se pudo abrir un canal: %v", err)
	}
	defer ch.Close()

	// Asegurarse de que la cola exista
	q, err := ch.QueueDeclare(
		"weather_queue", // Nombre de la cola
		false,           // Durable
		false,           // Auto-delete
		false,           // Exclusive
		false,           // No-wait
		nil,             // Argumentos adicionales
	)
	if err != nil {
		return fmt.Errorf("No se pudo declarar la cola: %v", err)
	}

	body, err := json.Marshal(data)
	if err != nil {
		return fmt.Errorf("Error al convertir los datos a JSON: %v", err)
	}

	err = ch.Publish(
		"",     // Exchange
		q.Name, // Routing key (Nombre de la cola)
		false,  // Mandatory
		false,  // Immediate
		amqp.Publishing{
			ContentType: "application/json",
			Body:        body,
		},
	)
	if err != nil {
		return fmt.Errorf("No se pudo publicar el mensaje en RabbitMQ: %v", err)
	}

	log.Println("📬 Mensaje enviado a RabbitMQ correctamente")
	return nil
}
