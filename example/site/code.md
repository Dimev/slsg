+++
title = "Code in YASSG"
+++

# Code in YASSG
YASSG can highlight codeblocks using a simple set of regex rules
For a number of languages, these are included, but you can also add your own.

For highlighting to be used, you need to use a custom markdown parser that can highlight the codeblocks.
See how this is done in the cookbook.

Below is a simple example of this for a non-existent functional language

```toml
[funlang]
extentions = ["fn", "fnl"]
keywords = 'def|import|let|in||match'
```

With these, the language looks like this

```funlang
def fibbonachi n:
  match n:
    0: 0
    1: 1
    _: fibbonachi (n - 1) (n - 2) 
```

Below, you can also see the rest of the supported languages

## Rust
```rust
// print hello world
fn main() {
  println!("Hello world!");
}
```

## Haskell
```haskell
-- print hello world
main :: IO {}
main = putStrLn "Hello world!"
```

## Html
TODO: find decent list of languages

## CSS

## Javascript

## Typescript

## Markdown

## Toml

## Yaml

## Json

## Lua

## C

## C++

## C#

## Zig
