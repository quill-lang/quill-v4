module.exports = grammar({
    name: 'feather',

    word: $ => $.identifier,

    extras: $ => [
      /\s/,
      $.line_comment,
    ],

    rules: {
      source_file: $ => seq(
        field('module', $.module),
        field('definition', repeat($.definition)),
      ),

      module: $ => seq('module', field('path', $.path)),

      definition: $ => seq(
        'def',
        field('name', $.identifier),
        ':',
        field('usage', optional('0')),
        field('ty', $._expr),
        '=',
        field('body', $._expr),
      ),

      path: $ => seq(
        repeat(seq(field('first', $.identifier), '::')),
        field('last', $.identifier),
      ),

      identifier: $ => /[\pL\pN\pS]+/,

      universe: $ => /[0-9]+/,

      line_comment: $ => token(
        seq('//', /[^\n]*/),
      ),

      _expr: $ => choice($._expr_no_app, $.app, $.ref, $.in),

      _expr_no_app: $ => choice(
        $.paren,
        $.local,
        $.fun,
        $.for,
        $.let,
        $.sort,
        $.inst,
        $.intro,
        $.match,
        $.fix,
        $.deref,
        $.loan,
        $.take,
      ),

      paren: $ => seq("(", field('inner', $._expr), ")"),

      local: $ => $.identifier,

      app: $ => prec.left(10, seq(
        field('left', $._expr),
        field('right', $._expr)
      )),

      _binder_structure: $ => choice(
        $.explicit,
        $.implicit,
        $.weak,
      ),

      explicit: $ => seq(
        '(',
        field('name', $.identifier),
        ':',
        field('usage', optional('0')),
        field('ty', $._expr),
        ')',
      ),

      implicit: $ => seq(
        '{',
        field('name', $.identifier),
        ':',
        field('usage', optional('0')),
        field('ty', $._expr),
        '}',
      ),

      weak: $ => seq(
        '{{',
        field('name', $.identifier),
        ':',
        field('usage', optional('0')),
        field('ty', $._expr),
        '}}',
      ),

      fun: $ => seq(
        'fun',
        field('binder_structure', $._binder_structure),
        field('arrow', choice('->', '=>')),
        field('body', $._expr),
      ),

      for: $ => seq(
        'for',
        field('binder_structure', $._binder_structure),
        field('arrow', choice('->', '=>')),
        field('body', $._expr),
      ),

      let: $ => seq(
        'let',
        field('name', $.identifier),
        '=',
        field('to_assign', $._expr),
        ';',
        field('body', $._expr),
      ),

      sort: $ => seq('Sort', field('universe', $.universe)),

      inst: $ => seq('inst', field('path', $.path)),

      intro: $ => seq(
        'intro',
        field('path', $.path),
        field('param', repeat($._expr_no_app)),
        '/',
        field('variant', $.identifier),
        '{',
        field('field', repeat($.intro_field)),
        '}',
      ),

      intro_field: $ => seq(
        field('name', $.identifier),
        '=',
        field('value', $._expr),
        ',',
      ),

      match: $ => seq(
        'match',
        field('subject', $._expr),
        'return',
        field('return', $._expr),
        field('body', $.match_body)
      ),

      match_body: $ => seq(
        '{',
        field('variant', repeat($.match_variant)),
        '}',
      ),

      match_variant: $ => seq(
        field('name', $.identifier),
        '->',
        field('value', $._expr),
        ',',
      ),

      fix: $ => seq(
        'fix',
        field('binder_structure', $._binder_structure),
        '=>',
        field('return', $._expr),
        'with',
        field('rec_name', $.identifier),
        ';',
        field('body', $._expr),
      ),

      ref: $ => prec.left(10, seq('ref', field('ty', $._expr_no_app))),

      deref: $ => seq('*', field('value', $._expr)),

      loan: $ => seq(
        'loan',
        field('ident', $.identifier),
        'as',
        field('as', $.identifier),
        'with',
        field('with', $.identifier),
        ';',
        field('body', $._expr),
      ),

      take: $ => seq(
        'take',
        field('ident', $.identifier),
        field('proofs', $.take_proofs),
        ';',
        field('body', $._expr),
      ),

      take_proofs: $ => seq(
        '{',
        field('proof', repeat($.take_proof)),
        '}',
      ),

      take_proof: $ => seq(
        field('local', $.identifier),
        '->',
        field('proof', $._expr),
        ',',
      ),

      in: $ => prec.left(5, seq(
        field('reference', $._expr),
        'in',
        field('target', $._expr)
      )),
    }
  });
