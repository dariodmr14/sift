# Tema 3 — Normalización

Normalizar es organizar las tablas para que **cada dato se guarde una sola vez**. Así se evitan las anomalías: si el teléfono de un cliente está en diez filas, al cambiarlo hay que acordarse de las diez.

## Formas normales
- **1FN:** cada celda guarda un único valor; nada de listas dentro de un campo.
- **2FN:** en 1FN y cada atributo depende de **toda** la clave, no de una parte.
- **3FN:** en 2FN y ningún atributo depende de otro que no sea clave (sin dependencias transitivas).

## Ejemplo
La tabla `Pedidos(id, cliente, telefono_cliente, producto)` repite el teléfono en cada pedido. Se separa en `Clientes(id, nombre, telefono)` y `Pedidos(id, id_cliente, producto)`.
