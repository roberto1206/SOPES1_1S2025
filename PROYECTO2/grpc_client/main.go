package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"time"

	pb "grpc_client/proto" // Asegúrate de que esta ruta es correcta según tu estructura

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
		grpcServerAddress := "producer.weather-tweets.svc.cluster.local:50051"
		err := sendToGRPCServer(&weather, grpcServerAddress)
		if err != nil {
			log.Println("❌ No se pudo enviar a gRPC:", err)
		} else {
			log.Println("✅ Mensaje enviado a gRPC correctamente")
		}
	}()

	w.WriteHeader(http.StatusOK)
	w.Write([]byte("Datos meteorológicos reenviados por gRPC"))
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
