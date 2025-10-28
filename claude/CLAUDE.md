# General

- Stop puttting Created by claude code into my files, if it has an author use me Praveen Perera
- Don't add comments that need old removed code to make sense in context
- If creating git commits never co author the commit with cluade or add any notes about claude just use my name
- Start inline code comments with a lowercase
- Capitalize higher level doc comments like functions and modules (ex: in rust comments starting with /// instead of //)
- Only add important comments to explain why something is being done
- Try to minimize nesting in functions

# Rust Project Specific

- `info` and `error` logs are okay to start capitalized
- Generate a docs for a crate with cargo doc -p <crate-name>, this will then be available at target/doc/<crate-name>/index.html, if you are unsure about how to use a crate, please generate the docs and read them
- Whenever you get clippy errors first run cargo fix --allow-dirty and then fix whatever remains
- I always prefer eyre (color-eyre if cli) to anyhow
