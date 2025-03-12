#!/bin/bash

# Lista de letras del abecedario
ABECEDARIO=("A" "B" "C" "D" "E" "F" "G" "H" "I" "J" "K" "L" "M" "N" "O" "P" "Q" "R" "S" "T" "U" "V" "W" "X" "Y" "Z")

# Seleccionar una letra aleatoria
LETRA=${ABECEDARIO[$RANDOM % ${#ABECEDARIO[@]}]}

# Guardar la letra en un archivo
echo "Letra generada: $LETRA" >> /home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/letras.txt
