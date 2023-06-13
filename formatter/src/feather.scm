; A query that allows for the automatic formatting of Feather code.
; See https://github.com/tweag/topiary/blob/main/languages/rust.scm for inspiration.

(identifier) @leaf

[
    (definition)
    (line_comment)
] @allow_blank_line_before

(definition body: _
    @prepend_input_softline @prepend_indent_start @append_indent_end)

(match_body
    .
    "{" @append_spaced_softline
    _
    "}" @prepend_spaced_softline
    .
)

(match_variant
    "," @append_input_softline
    .
)

[
    "module"
    "def"
    "fun"
    "for"
    "let"
    "Sort"
    "inst"
    "intro"
    "match"
    "return"
    "fix"
    "ref"
    "loan"
    "as"
    "with"
    "take"
    "in"
    "="
    "->"
    "=>"
    "{"
    "}"
    "0"
] @prepend_space @append_space

; Input softlines before and after all comments. This means that the input
; decides if a comment should have line breaks before or after. A line comment
; always ends with a line break.
[
    (line_comment)
] @prepend_input_softline

[
  ":"
  ","
] @append_space

[
  ":"
  ","
] @prepend_antispace

[
  "{"
  "("
] @append_indent_start

[
  "}"
  ")"
] @prepend_indent_end

";" @append_spaced_softline
