local mod = {}

-- render one node, the defaults
local defaults = {}

-- full list of all operations
-- some of these are not used, as they aren't parsed by the parser
-- They are still included for completeness
-- root node
function defaults.root(c)
  -- TODO: append definitions
  return fragment(table.unpack(c))
end

-- block quote (> c)
function defaults.blockquote(c)
  return fragment(table.unpack(c))
end

-- footnote definition
function defaults.footnoteDefinition(c) 
  return fragment(table.unpack(c))
end

-- mdx flow element, not used
function defaults.mdxJsxFlowElement(c)
  return fragment(table.unpack(c))
end

-- list (1. c), (- c), (1) c)
function defaults.list(c, ast)
  if ast.ordered then
    return h.ol():sub(table.unpack(c))
  else
    return h.ul():sub(table.unpack(c))
  end
end

-- toml (+++c+++), not rendered as it's used for config
function defaults.toml()
  return fragment()
end

-- yaml (---c---), not rendered as it's used for config
function defaults.yaml()
  return fragment()
end

-- break (\)
defaults["break"] = function()
  return fragment()
end

-- inline code (`c`)
function defaults.inlineCode(ast)
  return h.code():sub(ast.value)
end

-- inline math ($c$), ($$c$$)
function defaults.inlineMath(ast)
  return rawHtml(site.latexToMathml(ast.value))
end

-- delete (~c~), (~~c~~)
function defaults.delete(c)
  return h.strikethrough():sub(c)
end

-- emphasis (*a*)
function defaults.emphasis(c)
  return h.em():sub(c) 
end

-- mdxTextExpression ({=}), optionally enabled
function defaults.mdxTextExpression()
  return fragment("{" .. c.value .. "}")
end

-- mdx import, unused
function defaults.mdxJsEsm()
  return fragment() 
end

-- footnote reference ([^c]) TODO
function defaults.footnoteReference(ast, context)
  return fragment()
end

-- html (raw html)
function defaults.html(ast)
  return rawHtml(ast.value)
end

-- imgage (![a](b))
function defaults.image(ast)
  return h.img():attrs({ alt = ast.alt, url = ast.url })
end

-- image reference ![c] TODO
function defaults.imageReference(ast, context)
  return fragment()
end

-- mdx text element, not used
function defaults.mdxJsxTextElement(ast)
  return fragment()
end

-- link ([a](b))
function defaults.link(c, ast) 
  return h.a(ast.title)
    :attrs({ href = ast.url })
    :sub(table.unpack(c)) 
end

-- link reference ([c]) TODO
function defaults.linkReference(ast, context)
  return fragment()
end

-- strong (**c**)
function defaults.strong(c)
  return h.strong():sub(table.unpack(c))
end

-- text (any)
function defaults.text(ast)
  return fragment(ast.value)
end

-- code (```fang\ncode```)
function defaults.code(ast)
  return h.code():sub(h.pre(ast.value)) 
end

-- math ($$\n$$) Note that this is the same as inline math, but requires multiple lines
function defaults.math(ast)
  return rawHtml(site.latexToMathml(ast.value))
end

-- mdx flow expression, unused
function defaults.mdxFlowExpression()
  return fragment()
end

-- heading (# c), outputs up to h6
function defaults.heading(c, ast) 
  local headings = { h.h1, h.h2, h.h3, h.h4, h.h5 }
  return (headings[ast.depth] or h.h6)(table.unpack(c)) 
end

-- table (| a |), (| - |) TODO
function defaults.table(c, ast)
  return h.table():sub(table.unpack(c))
end

-- thematic break (***)
function defaults.thematicBreak()
  return h.hr()
end

-- table row (| a |) TODO
function defaults.tableRow(c)
  return fragment(table.unpack(c))
end

-- table cell (| a |) TODO
function defaults.tableCell(c) 
  return fragment(table.unpack(c))
end

-- list item (* c)
function defaults.listItem(c, ast) 
  return h.li():sub(table.unpack(c)) 
end

-- paragraph (text seperated by empty lines)
function defaults.paragraph(c)
  return h.p():sub(table.unpack(c))
end

-- definition ([a]: b) TODO
function defaults.definition(ast, context)
  return fragment()
end

-- compile many markdown nodes
function compileMany(children, funcs, context)
  local res = {}
  for i, value in ipairs(children) do
    table.insert(res, compileMd(value, funcs, context))
  end

  return res
end

-- Parse and compile markdown, with custom functions
function compileMd(ast, funcs, context)
  -- compile the markdown
  -- if we have children, pass those in as special
  if ast.children then 
    return (funcs[ast.type] 
      or defaults[ast.type] 
      or warn("Missing " .. ast.type)
    )(compileMany(ast.children, funcs), ast, context)
  else
    return (funcs[ast.type] 
      or defaults[ast.type] 
      or warn("Missing " .. ast.type)
    )(ast, context)
  end
end

-- Parse and compile markdown, with custom functions, and an empty context
function mod.compileMd(ast, funcs)
  return compileMd(ast, funcs, {})
end

return mod
