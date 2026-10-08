with open("src/engine.rs", "r") as f:
    content = f.read()

content = content.replace("Ctx<'js>", "Ctx<'_>")
content = content.replace("Object<'js>", "Object<'_>")
content = content.replace("Function<'js>", "Function<'_>")

with open("src/engine.rs", "w") as f:
    f.write(content)
