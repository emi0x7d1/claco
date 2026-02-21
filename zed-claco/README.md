# Claco Zed Extension

This extension provides the Claco agent for Zed.

## Development

To test this extension locally in Zed:

1. Open Zed.
2. Open the Extensions view (`cmd-shift-x`).
3. Click "Install Dev Extension".
4. Select this directory (`zed-claco`).
5. Open the Assistant panel (`cmd-shift-a`).
6. You should see "Claco Agent" in the list of available agents.

## Built-in Agent

The extension includes a local binary of `claco-acp-agent` for testing. In
production, Zed would download this binary based on the `archive` URLs in
`extension.toml`.
