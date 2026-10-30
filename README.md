# Cube Play

<a id="start-an-investigation"></a>

> [!WARNING]
> **Beta software.** CubeArc projects are under active development and are not yet
> stable. Features, APIs, file formats and workflows may change without backward
> compatibility. Bugs and incomplete features are expected. Keep backups of
> important data and independently verify results before relying on them.

A local investigation IDE powered by [Cube](https://github.com/cubearc/cube).
Organize independent Explorations in a Workspace, keep clues in Stage notebooks,
and open an Asset in its own Trail canvas.

- Keep an image and its caption together, with distinct input Asset identities.
- Edit text and captions while preserving earlier revisions and Run evidence.
- Run ROT13 or Brainfuck on text and inspect results beside the cell.
- Reopen saved content, reasoning and retained Run evidence.

![Electron notebook showing an image with its caption and inline ROT13 result.](docs/assets/desktop/image-caption.png)

## Try the Electron preview

The Electron source preview executes operations and opens the contextual terminal
on macOS. Linux workers have a separate [native validation workflow](docs/development.md#validate-linux-workers);
Linux Electron/terminal integration, Windows workers and signed installers remain
incomplete. Cloud belongs to a separate milestone. See [installation](docs/installation.md).

From a trusted Play checkout with Cube beside it:

```sh
npm ci --prefix frontend
npm run desktop:build --prefix frontend
npm run desktop:proof --prefix frontend
```

This opens the Electron application with its local Rust engine. Docker, WSL and a
browser server are not required for this path. Follow the
[first notebook](docs/getting-started.md) to create an Exploration and decode a clue.

## Documentation

- [Workspaces](docs/launcher.md) and [Explorations](docs/explorations.md): organize work.
- [Notebook](docs/notebook.md), [Assets](docs/workspace.md) and [image inspection](docs/visual-lab.md): keep and explore clues.
- [Saved state](docs/persistence.md): resume reasoning and private Run history.
- [Documentation index](docs/index.md): current capabilities and native references.

## Contribute

See [Contributing](CONTRIBUTING.md) for development and verification.
Track work in [Issues](https://github.com/cubearc/play/issues) and
[CubeArc Roadmap](https://github.com/orgs/cubearc/projects/1).
Report vulnerabilities through [Security](SECURITY.md).
Licensed under [Apache-2.0](LICENSE).

<a id="developer-quick-start"></a>
<a id="docker-development"></a>

The [development guide](docs/development.md) describes the native workflow;
[contributor verification](docs/reference/validation.md) distinguishes it from
checks still required while the repository refactor is underway.

## Support CubeArc

Help support CubeArc development with a cryptocurrency donation.
Send only the listed currency on its native network.

| Currency | Donation address |
| --- | --- |
| <img src="assets/donations/btc.svg" width="24" height="24" alt="Bitcoin"> **Bitcoin (BTC)** | `bc1q7lw966p5069wr6z3czhdkua6e6ym52kgqe8qhr` |
| <img src="assets/donations/eth.svg" width="24" height="24" alt="Ethereum"> **Ethereum (ETH)** | `0x0Da7C21897aDC34a0A91B1de802385d845D48858` |
| <img src="assets/donations/zec.svg" width="24" height="24" alt="Zcash"> **Zcash (ZEC)** | `t1LbSr9K1v7Du5DVeQY9HxRJqB3cc7Pa9QP` |
