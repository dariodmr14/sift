-- Práctica 2: consultas sobre la tienda de ejemplo.

-- Clientes que han hecho más de tres pedidos este año.
SELECT c.nombre, COUNT(*) AS pedidos
FROM clientes c
JOIN pedidos p ON p.id_cliente = c.id
WHERE p.fecha >= DATE '2026-01-01'
GROUP BY c.nombre
HAVING COUNT(*) > 3
ORDER BY pedidos DESC;

-- Una transacción: o se hacen las dos operaciones o ninguna.
BEGIN;
UPDATE cuentas SET saldo = saldo - 50 WHERE id = 1;
UPDATE cuentas SET saldo = saldo + 50 WHERE id = 2;
COMMIT;
