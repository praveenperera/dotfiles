#!/usr/bin/env python3
"""Read the protobuf schema embedded in the antigravity binary, to update src/cmd/agy/proto.rs after an agy update

antigravity is a Go program, so each .proto file's descriptor sits in it whole. This finds them,
then prints message definitions or decodes a blob with field names, using the standard library only

  agy-proto.py BINARY describe REGEX           message definitions whose full name matches REGEX
  agy-proto.py BINARY decode MESSAGE FILE      a serialized MESSAGE as JSON, long strings cut short

MESSAGE is a full name with its leading dot, such as .gemini_coder.Step for a steps.step_payload or
.exa.cortex_pb.CortexStepGeneratorMetadata for a gen_metadata.data
"""
import json
import re
import sys

# FileDescriptorProto field numbers and the wire types they use
FILE_FIELDS = {1: {2}, 2: {2}, 3: {2}, 4: {2}, 5: {2}, 6: {2}, 7: {2}, 8: {2}, 9: {2}, 10: {0, 2}, 11: {0, 2}, 12: {2}, 14: {0}, 15: {2}}
TYPES = {1: "double", 2: "float", 3: "int64", 4: "uint64", 5: "int32", 6: "fixed64", 7: "fixed32", 8: "bool", 9: "string",
         10: "group", 11: "message", 12: "bytes", 13: "uint32", 14: "enum", 15: "sfixed32", 16: "sfixed64", 17: "sint32", 18: "sint64"}
MESSAGES, ENUMS = {}, {}


def varint(buf, i):
    shift = value = 0
    while True:
        if i >= len(buf) or shift > 63:
            raise ValueError("bad varint")
        byte = buf[i]
        i += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, i
        shift += 7


def fields(buf):
    i = 0
    while i < len(buf):
        tag, i = varint(buf, i)
        number, wire = tag >> 3, tag & 7
        if wire == 0:
            value, i = varint(buf, i)
        elif wire == 2:
            n, i = varint(buf, i)
            value = buf[i:i + n]
            if len(value) < n:
                raise ValueError("truncated")
            i += n
        elif wire in (1, 5):
            size = 8 if wire == 1 else 4
            value, i = buf[i:i + size], i + size
        else:
            raise ValueError(f"wire type {wire}")
        yield number, wire, value


def safe(gen):
    try:
        yield from gen
    except ValueError:
        return


def descriptor_end(buf, start):
    """Where the descriptor starting at start ends: protoc writes fields in number order, so the
    first field that goes backwards, or is not a FileDescriptorProto field, starts something else"""
    i, last = start, 0
    while i < len(buf):
        try:
            tag, j = varint(buf, i)
            number, wire = tag >> 3, tag & 7
            if wire not in FILE_FIELDS.get(number, ()) or number < last:
                break
            if wire == 0:
                _, j = varint(buf, j)
            else:
                n, j = varint(buf, j)
                j += n
                if j > len(buf):
                    break
        except ValueError:
            break
        last, i = number, j
    return i


def load_enum(buf, prefix):
    name, values = "", {}
    for number, _, value in safe(fields(buf)):
        if number == 1:
            name = value.decode()
        elif number == 2:
            entry = dict((n, v) for n, _, v in safe(fields(value)))
            values[entry.get(2, 0)] = entry.get(1, b"").decode()
    ENUMS[f"{prefix}.{name}"] = values


def load_message(buf, prefix):
    name, declared, nested = "", {}, []
    for number, _, value in safe(fields(buf)):
        if number == 1:
            name = value.decode()
        elif number == 2:
            field = {}
            for n, _, v in safe(fields(value)):
                key = {1: "name", 3: "number", 4: "label", 5: "type", 6: "type_name"}.get(n)
                if key:
                    field[key] = v.decode() if isinstance(v, bytes) else v
            if "number" in field:
                field["type"] = TYPES.get(field.get("type"), field.get("type"))
                declared[field["number"]] = field
        elif number in (3, 4):
            nested.append((number, value))
    full = f"{prefix}.{name}"
    MESSAGES[full] = declared
    for number, value in nested:
        (load_message if number == 3 else load_enum)(value, full)


def load(binary):
    data = open(binary, "rb").read()
    found = {}
    # a lookahead, since the byte before a descriptor's 0x0a tag can be 0x0a too
    for match in re.finditer(rb"(?=\x0a([\x01-\x7f])([A-Za-z0-9_./-]+?\.proto))", data):
        if match.group(1)[0] != len(match.group(2)):
            continue
        blob = data[match.start():descriptor_end(data, match.start())]
        name = match.group(2)
        if len(blob) > len(found.get(name, b"")):
            found[name] = blob
    for blob in found.values():
        package, items = "", []
        for number, _, value in safe(fields(blob)):
            if number == 2:
                package = value.decode()
            elif number in (4, 5):
                items.append((number, value))
        for number, value in items:
            (load_message if number == 4 else load_enum)(value, f".{package}" if package else "")


def decode(buf, message, cut=300):
    declared = MESSAGES.get(message, {})
    out = {}
    for number, wire, value in fields(buf):
        field = declared.get(number)
        key = field["name"] if field else f"#{number}"
        kind = field.get("type") if field else None
        if kind == "message" and wire == 2:
            try:
                value = decode(value, field["type_name"], cut)
            except ValueError:
                value = {"_raw": value[:80].hex()}
        elif kind == "enum" and wire == 0:
            value = ENUMS.get(field["type_name"], {}).get(value, value)
        elif wire == 2 and kind in ("string", None):
            try:
                text = value.decode()
                value = text if len(text) <= cut else f"{text[:cut]}...<{len(text)}>"
            except UnicodeDecodeError:
                value = f"<bytes {len(value)}>"
        elif wire == 2 and kind != "bytes":
            packed, i = [], 0
            while i < len(value):
                item, i = varint(value, i)
                packed.append(item)
            value = packed
        elif isinstance(value, bytes):
            value = f"<bytes {len(value)}>"
        if field and field.get("label") == 3:
            out.setdefault(key, []).append(value)
        else:
            out[key] = value
    return out


def main():
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    load(sys.argv[1])
    if sys.argv[2] == "describe":
        pattern = re.compile(sys.argv[3])
        for name, declared in MESSAGES.items():
            if pattern.search(name):
                print(name)
                for number, field in sorted(declared.items()):
                    repeated = "repeated " if field.get("label") == 3 else ""
                    print(f"  {number:>4} {field.get('name')}: {repeated}{field.get('type')} {field.get('type_name', '')}")
    elif sys.argv[2] == "decode":
        print(json.dumps(decode(open(sys.argv[4], "rb").read(), sys.argv[3]), indent=1))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
