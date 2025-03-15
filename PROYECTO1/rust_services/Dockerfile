# Usa la imagen base oficial de Grafana
FROM grafana/grafana:latest

# Instala plugins adicionales (opcional)
# Ejemplo: grafana-piechart-panel
# RUN grafana-cli plugins install grafana-piechart-panel

# Copia archivos de configuración personalizados (opcional)
# Ejemplo: dashboards, datasources
# COPY dashboards /var/lib/grafana/dashboards
# COPY datasources /etc/grafana/provisioning/datasources

# Expone el puerto 3000 (puerto predeterminado de Grafana)
EXPOSE 3000

# Comando para iniciar Grafana
CMD ["grafana-server"]