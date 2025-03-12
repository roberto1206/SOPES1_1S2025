#!/bin/bash

# Imagen y comando para los contenedores de estrés
IMAGE="containerstack/alpine-stress"
COMMAND="stress"

# Opciones de carga para diferentes tipos de estrés (con CPU y DISK reducidos)
optionload=(
    "-i 1 -t 90s"                        # I/O stress - 30 segundos
    "-c 1 -t 90s --cpu-method sqrt"      # CPU stress reducido - 30 segundos (método sqrt es menos intensivo)
    "-m 1 -t 90s --vm-bytes 128M"        # RAM stress - 30 segundos
    "-d 1 -t 90s --hdd-bytes 10M"        # Disk stress reducido - 30 segundos (menos bytes de escritura)
)

# Prefijos descriptivos para los nombres de contenedores
prefixes=(
    "stress-io-"
    "stress-cpu-"
    "stress-ram-"
    "stress-disk-"
)

# Número de contenedores a crear
NUM_CONTAINERS=10

# Seleccionar aleatoriamente tipos de estrés
selected_options=()
selected_prefixes=()
for ((i=0; i<$NUM_CONTAINERS; i++)); do
    # Índice aleatorio para seleccionar el tipo de estrés
    index=$((RANDOM % ${#optionload[@]}))
    selected_options+=("${optionload[$index]}")
    selected_prefixes+=("${prefixes[$index]}")
done

# Almacenar nombres de contenedores para limpieza posterior
container_names=()

echo "=== FASE 1: Creando $NUM_CONTAINERS contenedores... ==="
# Crear los contenedores
for i in "${!selected_options[@]}"; do
    # Generar un nombre único para el contenedor
    unique_id=$(date +%Y%m%d%H%M%S)-$i-$RANDOM
    container_name="${selected_prefixes[$i]}${unique_id}"
    container_names+=("$container_name")
    
    # Crear el contenedor (sin iniciarlo todavía)
    docker create --name "$container_name" $IMAGE $COMMAND ${selected_options[$i]}
    
    echo "Contenedor #$((i+1))/$NUM_CONTAINERS: $container_name creado"
done

echo -e "\n=== FASE 2: Iniciando TODOS los contenedores... ==="
# Iniciar todos los contenedores juntos
for name in "${container_names[@]}"; do
    docker start "$name"
    echo "Contenedor $name iniciado"
done

echo -e "\nPuedes ver todos los contenedores con: docker stats"
echo -e "Los contenedores están ejecutando pruebas de estrés en paralelo.\n"

# Esperar 30 segundos
echo "=== FASE 3: Esperando 10 segundos mientras se ejecutan las pruebas de estrés... ==="
sleep 10

echo -e "\n=== FASE 4: Deteniendo todos los contenedores... ==="
# Detener todos los contenedores
for name in "${container_names[@]}"; do
    docker stop "$name"
    echo "Contenedor $name detenido"
done

echo -e "\n=== FASE 5: Eliminando todos los contenedores... ==="
# Eliminar todos los contenedores
for name in "${container_names[@]}"; do
    docker rm "$name"
    echo "Contenedor $name eliminado"
done

echo -e "\n=== Prueba de estrés completada con éxito. ==="