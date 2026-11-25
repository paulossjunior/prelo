"""Monta o plugin WASM mínimo de teste (T010) sem ferramentas externas.

Exporta `memory`, `hello` (devolve b"hello" ao Typst) e `forever` (laço sem fim, para o
teste de compilação que não termina). Protocolo: wasm-minimal-protocol do Typst.
Uso: python3 gen_wasm.py > teste.wasm
"""
import sys

def leb(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        out.append(b | (0x80 if n else 0))
        if not n:
            return bytes(out)

def name(s):
    b = s.encode()
    return leb(len(b)) + b

def section(sid, payload):
    return bytes([sid]) + leb(len(payload)) + payload

def vec(items):
    return leb(len(items)) + b"".join(items)

types = vec([
    b"\x60\x02\x7f\x7f\x00",  # 0: (i32, i32) -> ()
    b"\x60\x01\x7f\x00",      # 1: (i32) -> ()
    b"\x60\x00\x01\x7f",      # 2: () -> i32
])
imports = vec([
    name("typst_env") + name("wasm_minimal_protocol_send_result_to_host") + b"\x00" + leb(0),
    name("typst_env") + name("wasm_minimal_protocol_write_args_to_buffer") + b"\x00" + leb(1),
])
funcs = vec([leb(2), leb(2)])
memory = vec([b"\x00\x01"])
exports = vec([
    name("memory") + b"\x02" + leb(0),
    name("hello") + b"\x00" + leb(2),
    name("forever") + b"\x00" + leb(3),
])
hello = b"\x00" + b"\x41\x00" + b"\x41\x05" + b"\x10\x00" + b"\x41\x00" + b"\x0b"
forever = b"\x00" + b"\x03\x40\x0c\x00\x0b" + b"\x41\x00" + b"\x0b"
code = vec([leb(len(hello)) + hello, leb(len(forever)) + forever])
data = vec([b"\x00" + b"\x41\x00\x0b" + leb(5) + b"hello"])

module = (b"\x00asm\x01\x00\x00\x00" + section(1, types) + section(2, imports)
          + section(3, funcs) + section(5, memory) + section(7, exports)
          + section(10, code) + section(11, data))
sys.stdout.buffer.write(module)
