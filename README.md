# LM Bot

A small Windows Service, written in Rust, that listens for Google pub/sub messages, prompts a local LLM and sends the result out on another pub/sub topic for consumption by the client.

## Development

Once you `cargo build` you'll want to register the service with the local system using an `Administrator` user:

```bat
sc create lm-bot binPath="C:\Users\Ejangi\Sites\lmbot\target\debug\lmbot.exe"
```