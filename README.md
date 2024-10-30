# SLSG
Scriptable Lua Site Generator

## How does this differ from other site generators?
- No templating lua. Instead, it provides a simple library to do templating inside lua
- No markdown. Instead, there is Luamark, which makes writing content with lua easier

## Safety
SLSG does no sandboxing, and does not guarantee the lua script can't read or write files to arbitrary locations.
When outputting to a build directory however, it does try and prevent writing to files outside of this directory.

## Current TODO:
- [ ] have example site also serve as short intro to slsg (show some features)
- [ ] API docs
- [X] Luamark parser
- [ ] Luamark parser tests
- [X] Syntax highlighting
- [X] Functioning macros

