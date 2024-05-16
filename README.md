# LM Bot

A small Windows Service, written in Rust, that listens for Google pub/sub messages, prompts a local LLM and sends the result out on another pub/sub topic for consumption by the client.

## Development

Open `cmd.exe` as admin and run:

```bat
cargo post build
```

This will build the code and run the `post_build.rs` script to install the service on your machine.

You'll then need to go to the Services dashboard in Windows and start the service. Alternatively, you can run `net start lm-bot`.

After the initial installation, you no longer need to use the `post_build.rs` script, so you can simply run:

```bat
cargo build
```

And restart the service.