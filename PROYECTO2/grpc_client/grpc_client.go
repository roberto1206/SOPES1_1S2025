package main

import (
	"context"
	"time"

	pb "grpc_client/proto" // cambia según tu estructura

	"google.golang.org/grpc"
)

func sendToGRPCServer(data *pb.WeatherData, address string) error {
	conn, err := grpc.Dial(address, grpc.WithInsecure())
	if err != nil {
		return err
	}
	defer conn.Close()

	client := pb.NewWeatherServiceClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	_, err = client.SendWeather(ctx, data)
	if err != nil {
		return err
	}

	return nil
}
