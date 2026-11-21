# Genera los dos PDF inventados de los datos de demostración (texto en Helvetica, varias
# páginas), sin dependencias. Uso, desde la raíz del repo:
#   python scripts/demo-pdf.py demo          (en español)
#   python scripts/demo-pdf.py demo-en en    (en inglés)
import sys

BS = chr(92)


def esc(t):
    return t.replace(BS, BS + BS).replace("(", BS + "(").replace(")", BS + ")")


def pdf(path, pages):
    objs = []

    def add(b):
        objs.append(b)
        return len(objs)

    font = add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
    pages_id = len(objs) + 1
    objs.append(None)
    kids = []
    for lines in pages:
        body = b"BT 72 760 Td 18 TL\n"
        for i, line in enumerate(lines):
            size = b"/F1 18 Tf " if i == 0 else b"/F1 12 Tf "
            body += size + b"(" + esc(line).encode("cp1252") + b") Tj T*\n"
        body += b"ET"
        c = add(b"<< /Length %d >>\nstream\n" % len(body) + body + b"\nendstream")
        p = add(
            b"<< /Type /Page /Parent %d 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 %d 0 R >> >> /Contents %d 0 R >>"
            % (pages_id, font, c)
        )
        kids.append(p)
    objs[pages_id - 1] = (
        b"<< /Type /Pages /Kids [" + b" ".join(b"%d 0 R" % k for k in kids) + b"] /Count %d >>" % len(kids)
    )
    cat = add(b"<< /Type /Catalog /Pages %d 0 R >>" % pages_id)
    out = b"%PDF-1.4\n"
    offs = []
    for i, o in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % i + o + b"\nendobj\n"
    x = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objs) + 1)
    out += b"".join(b"%010d 00000 n \n" % o for o in offs)
    out += b"trailer\n<< /Size %d /Root %d 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objs) + 1, cat, x)
    open(path, "wb").write(out)


D = sys.argv[1]
LANG = sys.argv[2] if len(sys.argv) > 2 else "es"

if LANG == "en":
    pdf(
        D + "/Studies/Databases/transactions-notes.pdf",
        [
            [
                "Unit 5. Transactions",
                "A transaction groups several operations that happen all together or not at all.",
                "If something fails halfway, what was done is undone (rollback) and the database",
                "is left as it was before it started.",
            ],
            [
                "ACID properties",
                "Atomicity: all or nothing.",
                "Consistency: from one valid state to another valid state.",
                "Isolation: two transactions running at once don't step on each other.",
                "Durability: what is committed is not lost, even in a power cut.",
            ],
            [
                "Isolation levels",
                "Read uncommitted, read committed, repeatable read and serializable.",
                "More isolation means fewer surprises and more waiting between transactions.",
            ],
        ],
    )
    pdf(
        D + "/Projects/Shop website/services-contract.pdf",
        [
            [
                "Services contract: online shop",
                "Between Lucy Fern (ceramics) and the person who builds the shop.",
                "Purpose: design and launch of an online shop with a payment gateway.",
            ],
            [
                "Deadlines and payments",
                "Delivery before 15 December.",
                "Payment in two parts: 50% on accepting the quote and 50% on delivery.",
                "Changes after delivery are invoiced separately.",
            ],
        ],
    )
else:
    pdf(
        D + "/Estudios/Bases de datos/apuntes-transacciones.pdf",
        [
            [
                "Tema 5. Transacciones",
                "Una transacción agrupa varias operaciones que se hacen todas o ninguna.",
                "Si algo falla a mitad, se deshace lo hecho (rollback) y la base de datos",
                "queda como estaba antes de empezar.",
            ],
            [
                "Propiedades ACID",
                "Atomicidad: todo o nada.",
                "Consistencia: de un estado válido a otro estado válido.",
                "Aislamiento: dos transacciones a la vez no se pisan.",
                "Durabilidad: lo confirmado (commit) no se pierde aunque se vaya la luz.",
            ],
            [
                "Niveles de aislamiento",
                "Lectura no confirmada, lectura confirmada, lectura repetible y serializable.",
                "Cuanto más aislamiento, menos sorpresas y más esperas entre transacciones.",
            ],
        ],
    )
    pdf(
        D + "/Proyectos/Web de la tienda/contrato-servicios.pdf",
        [
            [
                "Contrato de servicios: tienda online",
                "Entre Lucía Ferrer (cerámica) y la persona que desarrolla la tienda.",
                "Objeto: diseño y puesta en marcha de una tienda online con pasarela de pago.",
            ],
            [
                "Plazos y pagos",
                "Entrega antes del 15 de diciembre.",
                "Pago en dos partes: 50 % al aceptar el presupuesto y 50 % a la entrega.",
                "Las revisiones después de la entrega se facturan aparte.",
            ],
        ],
    )
