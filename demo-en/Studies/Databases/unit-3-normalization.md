# Unit 3 — Normalization

Normalizing means organizing tables so that **each piece of data is stored only once**. That avoids anomalies: if a customer's phone number is in ten rows, changing it means remembering all ten.

## Normal forms
- **1NF:** each cell holds a single value; no lists inside a field.
- **2NF:** in 1NF and every attribute depends on the **whole** key, not just part of it.
- **3NF:** in 2NF and no attribute depends on another non-key attribute (no transitive dependencies).

## Example
The table `Orders(id, customer, customer_phone, product)` repeats the phone number in every order. It is split into `Customers(id, name, phone)` and `Orders(id, customer_id, product)`.
