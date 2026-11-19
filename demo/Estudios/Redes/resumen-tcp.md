# Resumen: TCP y UDP

**TCP** asegura que los datos lleguen enteros y en orden: numera los segmentos, confirma lo recibido y reenvía lo que se pierde. Antes de enviar nada abre la conexión con el saludo en tres pasos (SYN, SYN-ACK, ACK).

**UDP** no confirma nada: es más rápido y se usa cuando un dato perdido no importa tanto como el retraso (llamadas, juegos, vídeo en directo).

Puertos típicos: 80 y 443 (web), 22 (SSH), 53 (DNS), 5432 (PostgreSQL).
