-- Lab 2: queries on the example shop.

-- Customers who placed more than three orders this year.
SELECT c.name, COUNT(*) AS orders
FROM customers c
JOIN orders o ON o.customer_id = c.id
WHERE o.placed_on >= DATE '2026-01-01'
GROUP BY c.name
HAVING COUNT(*) > 3
ORDER BY orders DESC;

-- A transaction: either both updates happen or neither does.
BEGIN;
UPDATE accounts SET balance = balance - 50 WHERE id = 1;
UPDATE accounts SET balance = balance + 50 WHERE id = 2;
COMMIT;
