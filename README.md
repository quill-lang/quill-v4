# Quill

![License: MIT](https://img.shields.io/github/license/quill-lang/quill)
![CI badge](https://github.com/quill-lang/quill/actions/workflows/rust.yml/badge.svg)

An expressive, performant, modern functional programming language.

## Development

To generate the tree-sitter grammars, execute `npm run generate` inside `feather_grammar`.

It can be useful during development to run the following command from the root directory.
```sh
(cd feather_grammar; npm run generate); cargo run
```

## Other dependencies

The `feather_formatter` crate uses code from [Topiary](https://github.com/tweag/topiary), released under the MIT license, but does not list it as a Rust dependency.
