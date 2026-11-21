# Summary: TCP and UDP

**TCP** makes sure data arrives complete and in order: it numbers the segments, acknowledges what it receives and resends what gets lost. Before sending anything it opens the connection with the three-way handshake (SYN, SYN-ACK, ACK).

**UDP** acknowledges nothing: it is faster and is used when a lost packet matters less than delay (calls, games, live video).

Common ports: 80 and 443 (web), 22 (SSH), 53 (DNS), 5432 (PostgreSQL).
