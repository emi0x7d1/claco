# Claco

A proof-of-concept TOS-compliant Claude Code "proxy" for third-party clients.

<video src="https://private-user-images.githubusercontent.com/138755561/553543206-26af1540-740e-4c94-a4ea-2ab86f2295cf.mp4?jwt=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3NzE4NTkyMzYsIm5iZiI6MTc3MTg1ODkzNiwicGF0aCI6Ii8xMzg3NTU1NjEvNTUzNTQzMjA2LTI2YWYxNTQwLTc0MGUtNGM5NC1hNGVhLTJhYjg2ZjIyOTVjZi5tcDQ_WC1BbXotQWxnb3JpdGhtPUFXUzQtSE1BQy1TSEEyNTYmWC1BbXotQ3JlZGVudGlhbD1BS0lBVkNPRFlMU0E1M1BRSzRaQSUyRjIwMjYwMjIzJTJGdXMtZWFzdC0xJTJGczMlMkZhd3M0X3JlcXVlc3QmWC1BbXotRGF0ZT0yMDI2MDIyM1QxNTAyMTZaJlgtQW16LUV4cGlyZXM9MzAwJlgtQW16LVNpZ25hdHVyZT04ZmU0OGQ5ZmM1NGExMzdkNGU4OTk3YWIyM2VhYjY3YjAxMjFkYjRiZjBiYjgyYjBiMDIzNTdjNDdiNjA1NTdmJlgtQW16LVNpZ25lZEhlYWRlcnM9aG9zdCJ9.KJmolCpN1iiQmBWxhJueg3IAVLNEu3kGH-5rW66MsQU">
</video>

## Building

1. Install Rust
2. Build `cargo build`

If you want to use it with Zed, you must serve the agent archive:

1. Install Deno.
2. Run server

```sh
$ deno run -A serve-agent.ts
```

3. Run `Zed: Install dev extension` in Zed's command palette and select the
   `zed-claco` directory.

If you make any changes and wish to update your extension, you must first delete
the old agent binary because Zed caches the archive and your changes won't be
reflected.

```sh
$ rm -r ~/Library/Application\ Support/Zed/external_agents/claco/
```

Then you must: uninstall the old extension, restart Zed and install the
extension again ¯\\_(ツ)_/¯

## Using

`claco-acp-agent` contains an Agent implementation for the Agent Client Protocol
that you may use in editor such as Zed and Intellij. You may point your editor
to it.

`claco-termulator` allows you to run processes in a headless terminal and attach
to it from other terminals. It also lets you inspect the terminal state to help
with debugging.

```sh
$ claco-termulator run -- nvim # Runs neovim in our own PTY
$ claco-termulator attach # Stream the neovim session
```

`zed-claco`: see [Building](#building)

## Limitations

Unfortunately, I do not have the time to maintain this but I am willing to
contribute if someone else takes over.

Aside from that, I can't afford Claude Pro so I have been testing with a
recorded asciinema session behind an ollama proxy so I'm not sure if the
behavior might change due to this.

The current implementation is very alpha quality and only works for:

- Sending regular prompts
- Receiving tool call responses
- Receiving text responses

It does not work yet for:

- Responding Claude's questions
- More...
