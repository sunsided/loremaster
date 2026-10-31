# loremaster-tools

This crate is part of [loremaster](https://github.com/sunsided/loremaster), a world editor for text adventures. In loremaster, each text in the game comes from the world state.

This crate has the operations that an AI assistant can do on a project: read, play and verify. The arguments and the results are JSON values. Thus an MCP server and a tool loop for a chat API can use the same operations.

The tools do not change the project files. To suggest a change, the assistant makes a proposal. A person accepts the proposal or rejects it.

This crate has no input or output of its own.

## License

EUPL-1.2. Read [LICENSE](LICENSE).
