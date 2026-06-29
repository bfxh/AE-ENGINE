# Update workspace Cargo.toml
ws = r"d:\rj\wasteland_project\Cargo.toml"
with open(ws, "r", encoding="utf-8") as f:
    s = f.read()

old = 'criterion = "0.5"'
new = 'criterion = "0.5"\ncpal = "0.18"'
assert old in s
s = s.replace(old, new)

with open(ws, "w", encoding="utf-8") as f:
    f.write(s)
print("Workspace Cargo.toml updated")

# Update wasteland_audio Cargo.toml
audio = r"d:\rj\wasteland_project\wasteland_audio\Cargo.toml"
with open(audio, "r", encoding="utf-8") as f:
    s = f.read()

old = """[dependencies]
glam.workspace = true
rand.workspace = true"""
new = """[dependencies]
glam.workspace = true
rand.workspace = true
cpal.workspace = true
parking_lot = "0.12"
crossbeam-channel = "0.5\""""
assert old in s
s = s.replace(old, new)

with open(audio, "w", encoding="utf-8") as f:
    f.write(s)
print("wasteland_audio Cargo.toml updated")
