#!/bin/bash
# Número de repeticiones (contenedores a crear)
REPETICIONES=4

# Array con los diferentes tipos de contenedores que se pueden ejecutar
CONTAINER_TYPES=("--vm 1 --vm-bytes 64M -t 30s" "--cpu 2 -t 30s" "--io 1 -t 30s" "--hdd 1 --hdd-bytes 100M -t 30s")
CONTAINER_LABELS=("ram" "cpu" "io" "disk")

# Generar contenedores aleatorios según el número de repeticiones
for i in $(seq 1 $REPETICIONES); do
    # Seleccionar aleatoriamente un tipo de contenedor
    RANDOM_INDEX=$((RANDOM % ${#CONTAINER_TYPES[@]}))
    RANDOM_TYPE=${CONTAINER_TYPES[$RANDOM_INDEX]}
    LABEL=${CONTAINER_LABELS[$RANDOM_INDEX]}

    # Generar un nombre único para el contenedor con su tipo identificado
    CONTAINER_NAME="stress_${LABEL}_$(date +%s)_$(head /dev/urandom | tr -dc A-Za-z0-9 | head -c 6)"

    # Ejecutar el contenedor con la imagen
    docker run -d --rm --name "$CONTAINER_NAME" containerstack/alpine-stress stress $RANDOM_TYPE

    echo "Contenedor $CONTAINER_NAME creado con la opción: $RANDOM_TYPE"
done

echo "Se han creado $REPETICIONES contenedores."