module.exports = grammar({
    name: 'feather',

    word: $ => $.identifier,

    extras: $ => [
      /\s/,
      $.comment,
    ],

    rules: {
      source_file: $ => seq($.module, repeat($.definition)),

      module: $ => seq('module', $.qualified_name),

      definition: $ => seq(
        'def',
        $.identifier,
        ':',
        optional('0'),
        $._expr,
        '=',
        $._expr,
      ),

      qualified_name: $ => seq(
        repeat(seq($.identifier, '::')),
        $.identifier
      ),

      identifier: $ => /[\pL\pN\pS]+/,

      universe: $ => /[0-9]+/,

      comment: $ => token(
        seq('//', /[^\n]*/),
      ),

      _expr: $ => choice($._expr_no_app, $.app, $.ref, $.in),

      _expr_no_app: $ => choice(
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

      local: $ => $.identifier,

      app: $ => prec.left(10, seq($._expr, $._expr)),

      _binder: $ => choice(
        $.explicit,
        $.implicit,
        $.weak,
      ),

      explicit: $ => seq(
        '(',
        $.identifier,
        ':',
        optional('0'),
        $._expr,
        ')',
      ),

      implicit: $ => seq(
        '{',
        $.identifier,
        ':',
        optional('0'),
        $._expr,
        '}',
      ),

      weak: $ => seq(
        '{{',
        $.identifier,
        ':',
        optional('0'),
        $._expr,
        '}}',
      ),

      fun: $ => seq(
        'fun',
        $._binder,
        choice('->', '=>'),
        $._expr,
      ),

      for: $ => seq(
        'for',
        $._binder,
        choice('->', '=>'),
        $._expr,
      ),

      let: $ => seq(
        'let',
        $.identifier,
        '=',
        $._expr,
        ';',
        $._expr,
      ),

      sort: $ => seq('Sort', $.universe),

      inst: $ => seq('inst', $.qualified_name),

      intro: $ => seq(
        'intro',
        field('name', $.qualified_name),
        field('param', repeat($._expr_no_app)),
        '/',
        field('variant', $.identifier),
        '{',
        field('field', repeat($.intro_field)),
        '}',
      ),

      intro_field: $ => seq(
        $.identifier,
        '=',
        $._expr,
        ',',
      ),

      match: $ => seq(
        'match',
        field('arg', $._expr),
        'return',
        field('return', $._expr),
        '{',
        field('variant', repeat($.match_variant)),
        '}',
      ),

      match_variant: $ => seq(
        $.identifier,
        '->',
        $._expr,
        ',',
      ),

      fix: $ => seq(
        'fix',
        field('binder', $._binder),
        'return',
        field('return', $._expr),
        'with',
        field('body', $._expr),
      ),

      ref: $ => prec.left(10, seq('ref', $._expr_no_app)),

      deref: $ => seq('*', $._expr),

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
        '{',
        field('proof', $.take_proof),
        '}',
        ';',
        field('body', $._expr),
      ),

      take_proof: $ => seq(
        $.identifier,
        '->',
        $._expr,
        ',',
      ),

      in: $ => prec.left(5, seq($._expr, 'in', $._expr)),
    }
  });
