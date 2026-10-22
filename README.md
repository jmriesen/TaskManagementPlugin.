# Task Management

An Obsidian plugin for searching and filtering tasks by tag across a vault, backed by a shared Rust `core` library (compiled to wasm) also used by a companion nvim plugin.

## Features

- Search todos, with optional include/any/exclude tag filters
- Named, reusable tag filters defined in `task-filters.md` at the vault root. Each filter becomes a **Search: `<name>`** command, and its tags are checklists you can tick or untick to change the search on the fly:

  ```markdown
  ## Next actions
  ### Include
  - [x] #NextAction
  ### Any
  - [ ] #JobHunt
  ### Exclude
  - [x] #BusinessHours
  ```

  Only ticked tags are used. Todo and requires searches skip this file.
- Search `#Requires<Name>` tasks, grouped by person
- Detect heading/filename mismatches
- Render excerpts of other notes via an `excerpt` code block

## Development

```bash
npm install
npm run dev    # build wasm + watch/rebuild the plugin
npm run build  # production build
```
