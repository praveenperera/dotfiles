# General

- Stop puttting Created by claude code into my files, if it has an author use me Praveen Perera
- Don't add comments that need old removed code to make sense in context
- If creating git commits never co author the commit with cluade or add any notes about claude just use my name
- Start inline code comments with a lowercase
- Capitalize higher level doc comments like functions and modules (ex: in rust comments starting with /// instead of //)
- Don't end comments with a period (periods within comments are fine)
- Only add important comments to explain why something is being done
- Try to minimize nesting in functions
- All comments should make sense without the context if this particular conversation

# Rust Project Specific

- `info` and `error` logs are okay to start capitalized
- Generate docs for a crate with `cargo doc -p <crate-name>`, this will then be available at `target/doc/<crate-name>/index.html`, if you are unsure about how to use a crate, please generate the docs and read them
- If docs have already been generated, check `target/doc/` for existing documentation of the project and its dependencies before regenerating
- Whenever you get clippy errors first run cargo fix --allow-dirty and then fix whatever remains
- I always prefer eyre (color-eyre if cli) to anyhow
- if-let chains are stable in Rust now, always collapse nested if-lets into a single statement using `&&`
- Avoid redundant closures - use `.map(func)` instead of `.map(|x| func(x))`
- Prefer tuple structs over named field structs for simple wrappers (e.g., `struct Foo(Arc<Inner>)` not `struct Foo { inner: Arc<Inner> }`)
- `#[act_zero_ext::into_actor_result]` on `fn foo()` generates: public async `foo() -> ActorResult<T>` wrapper + private `do_foo()` with original logic
- Prefer structs with methods over freestanding functions to encapsulate state and provide a cleaner API
