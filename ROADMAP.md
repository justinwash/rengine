# Rengine Roadmaps

Roadmap planning is now split into separate runtime and editor tracks.

- `ROADMAP_ENGINE.md` tracks engine-runtime work such as rendering, assets, simulation, serialization, and platform support.
- `ROADMAP_EDITOR.md` tracks editor-shell, authoring, workflow, validation, and play-in-editor work.
- `EDITOR_GUIDE.md` explains how the current editor works, why it is structured this way, and how to use it as the start of a game-authoring workflow.

Current status: the editor-to-`Scene2D` bridge now supports grouped multi-sprite prefab export, the shell has native open/save-as flow, scene work lives in per-document tabs, sprite nodes can browse image files or fall back to placeholders, and Camera2D nodes preview their screen bounds from typed scene and camera properties.

This split keeps runtime-system work distinct from authoring-tool work while both are moving forward in parallel.
