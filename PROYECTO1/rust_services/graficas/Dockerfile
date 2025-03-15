FROM node:18-alpine

WORKDIR /app

COPY cpu.json /app/cpu.json
COPY ram.json /app/ram.json

RUN npm install -g http-server

EXPOSE 8080

CMD ["http-server", "--port", "8080", "--cors"]