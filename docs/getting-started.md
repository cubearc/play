# Inspect your first file

[Install Play](installation.md). In a POSIX shell, create an original input:

```sh
mkdir cube-play-example
printf 'Cube' > cube-play-example/sample.bin
cube play ./cube-play-example
```

The browser opens after readiness. In **Files**, select `sample.bin` and inspect
its four bytes: `43 75 62 65`. Unknown signatures remain observations; they do not
make the file unavailable. Opening a source file does not import or change it.

Use **Workspaces** to open another folder or resume an investigation.
Keep the private session link to yourself. To stop this workspace:

```sh
cube play --stop ./cube-play-example
```

The workspace's saved data survives shutdown. Continue with
[browse and import files](workspace.md), [visual analysis](visual-lab.md), or
[private notes](notebook.md).
