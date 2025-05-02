package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/segmentio/kafka-go"
)

// WeatherData estructura para deserializar mensajes JSON
type WeatherData struct {
	Description string `json:"description"`
	Country     string `json:"country"`
	Weather     string `json:"weather"`
}

func main() {
	// Configuración desde variables de entorno
	topic := getEnvWithDefault("KAFKA_TOPIC", "weather-topic")
	broker := getEnvWithDefault("KAFKA_BOOTSTRAP_SERVERS", "my-cluster-kafka-bootstrap.weather-tweets:9092")
	groupID := getEnvWithDefault("KAFKA_GROUP_ID", "weather-consumer-group")

	// Configurar el logger para incluir hora y archivo
	log.SetFlags(log.LstdFlags | log.Lshortfile)
	log.Printf("🔄 Iniciando consumidor de Kafka: topic=%s, broker=%s", topic, broker)

	// Configurar lector de Kafka con opciones más robustas
	reader := kafka.NewReader(kafka.ReaderConfig{
		Brokers:        []string{broker},
		Topic:          topic,
		GroupID:        groupID,
		MinBytes:       10e3, // 10KB mínimo por lote
		MaxBytes:       10e6, // 10MB máximo por lote
		MaxWait:        1 * time.Second,
		StartOffset:    kafka.LastOffset, // Comenzar desde el último mensaje
		CommitInterval: 1 * time.Second,  // Confirmar offsets cada segundo

		// Usar un rebalance rápido para recuperación rápida
		RebalanceTimeout: 5 * time.Second,

		// Retry lógica
		ReadBackoffMin: 100 * time.Millisecond,
		ReadBackoffMax: 1 * time.Second,
	})

	// Configurar context con cancelación para shutdown limpio
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Canal para señales de terminación
	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGINT, syscall.SIGTERM)

	// Canal para errores de procesamiento
	errs := make(chan error, 1)

	// Iniciar procesamiento en goroutine
	go func() {
		defer reader.Close()
		log.Println("🟢 Consumer escuchando en el topic:", topic)

		for {
			select {
			case <-ctx.Done():
				return // Salir si el contexto fue cancelado
			default:
				msg, err := reader.ReadMessage(ctx)
				if err != nil {
					if ctx.Err() == context.Canceled {
						return // Ignorar errores por cancelación del contexto
					}
					log.Printf("❌ Error al leer mensaje: %v", err)
					errs <- err
					time.Sleep(1 * time.Second) // Esperar antes de reintentar
					continue
				}

				// Procesar el mensaje
				processMessage(msg)

				// Confirmación explícita (opcional con kafka-go)
				// Kafka-go lo hace automáticamente en CommitInterval
			}
		}
	}()

	// Esperar señal de terminación o error
	select {
	case sig := <-signals:
		log.Printf("📨 Señal recibida: %v", sig)
	case err := <-errs:
		log.Printf("🚨 Error crítico: %v", err)
	}

	// Iniciar terminación limpia
	log.Println("🔄 Cerrando consumidor...")
	cancel()                    // Cancelar context para que la goroutine de lectura termine
	time.Sleep(2 * time.Second) // Dar tiempo para que todo cierre correctamente
	log.Println("👋 Consumidor finalizado")
}

// Procesar un mensaje de Kafka
func processMessage(msg kafka.Message) {
	log.Printf("📦 Mensaje recibido - Partition: %d, Offset: %d", msg.Partition, msg.Offset)

	// Deserializar el mensaje JSON
	var weatherData WeatherData
	if err := json.Unmarshal(msg.Value, &weatherData); err != nil {
		log.Printf("❌ Error al deserializar mensaje: %v", err)
		fmt.Println("📄 Contenido del mensaje:", string(msg.Value))
		return
	}

	// Procesar datos
	log.Printf("🌤️ Datos meteorológicos: País=%s, Clima=%s, Descripción=%s",
		weatherData.Country, weatherData.Weather, weatherData.Description)

	// Aquí podrías implementar tu lógica de procesamiento:
	// - Guardar en base de datos
	// - Enviar notificaciones
	// - Analizar tendencias
	// - etc.

	fmt.Println("📄 Mensaje procesado correctamente")
}

// Obtener variable de entorno con valor predeterminado
func getEnvWithDefault(key, defaultValue string) string {
	value := os.Getenv(key)
	if value == "" {
		return defaultValue
	}
	return value
}
