# Genera la carpeta de la prueba de carga de Sift: ~100 000 archivos inventados (texto,
# código y algunos PDF), siempre los mismos (semilla fija). Nada de nadie.
#
# Uso:  python scripts/aguante/generar.py <carpeta> [num_archivos]
# Borrar al acabar:  python scripts/aguante/generar.py <carpeta> --borrar
import os
import random
import shutil
import sys

# Un PDF de texto sin dependencias (lo mismo que scripts/demo-pdf.py).
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

WORDS = (
    "proyecto cliente factura reunión presupuesto entrega informe tema apuntes examen práctica base datos "
    "consulta tabla índice transacción normalización red protocolo servidor paquete viaje hotel billete tren "
    "vuelo reserva alquiler piso luz agua internet receta cocina ingrediente horno tienda pedido envío pago "
    "contrato plazo revisión diseño página estilo color imagen producto catálogo precio oferta cliente correo "
    "calendario tarea equipo objetivo resultado gráfico ventas mes trimestre año resumen conclusión pregunta "
    "respuesta idea borrador versión final copia carpeta archivo documento nota lista paso error prueba"
).split()
CODE = [
    ("rs", "fn {n}(x: u32) -> u32 {{\n    let total = x * {k};\n    total + {k}\n}}\n"),
    ("ts", "export function {n}(items: number[]): number {{\n  return items.reduce((a, b) => a + b * {k}, 0);\n}}\n"),
    ("py", "def {n}(datos):\n    total = 0\n    for d in datos:\n        total += d * {k}\n    return total\n"),
]


def para(rng, n):
    return " ".join(rng.choice(WORDS) for _ in range(n)).capitalize() + "."


def main():
    root = sys.argv[1]
    if "--borrar" in sys.argv:
        shutil.rmtree(root, ignore_errors=True)
        print("borrada", root, not os.path.exists(root))
        return
    total = int(sys.argv[2]) if len(sys.argv) > 2 else 100_000
    rng = random.Random(20261120)
    per_dir = 100
    made = 0
    for d in range(total // per_dir):
        folder = os.path.join(root, f"area-{d // 50:02d}", f"carpeta-{d:04d}")
        os.makedirs(folder, exist_ok=True)
        for i in range(per_dir):
            n = d * per_dir + i
            r = rng.random()
            if r < 0.003:  # ~300 PDF de 2–4 páginas
                pages = [[para(rng, 6)] + [para(rng, 12) for _ in range(3)] for _ in range(rng.randint(2, 4))]
                pdf(os.path.join(folder, f"documento-{n}.pdf"), pages)
            elif r < 0.25:  # código
                ext, tpl = rng.choice(CODE)
                body = "".join(tpl.format(n=f"calcular_{n}_{j}", k=rng.randint(2, 99)) for j in range(rng.randint(5, 40)))
                with open(os.path.join(folder, f"modulo_{n}.{ext}"), "w", encoding="utf-8", newline="\n") as f:
                    f.write(body)
            else:  # texto: notas en Markdown o .txt de 1–8 KB
                ext = "md" if rng.random() < 0.6 else "txt"
                title = f"# {para(rng, 4)}\n\n" if ext == "md" else ""
                body = title + "\n\n".join(para(rng, rng.randint(20, 90)) for _ in range(rng.randint(2, 12)))
                with open(os.path.join(folder, f"nota-{n}.{ext}"), "w", encoding="utf-8", newline="\n") as f:
                    f.write(body)
            made += 1
        if d % 100 == 0:
            print(f"{made} archivos…", flush=True)
    print("hecho:", made, "archivos en", root)


if __name__ == "__main__":
    main()
