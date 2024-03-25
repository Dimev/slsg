local mod = {}

-- render one node, the defaults
local defaults = {}
function defaults.root(c) return fragment(table.unpack(c)) end
function defaults.blockquote(c) return fragment(table.unpack(c)) end
function defaults.footnoteDefinition(c) return fragment(table.unpack(c)) end
function defaults.mdxJsxFlowElement(c) return fragment(table.unpack(c)) end
function defaults.list(c, ast)
  if ast.ordered then
    return h.ol():sub(table.unpack(c))
  else
    return h.ul():sub(table.unpack(c))
  end
end
function defaults.toml() return fragment() end
function defaults.yaml() return fragment() end
defaults["break"] = function() return h.hr() end
function defaults.inlineCode(ast) return h.code():sub(ast.value) end
function defaults.inlineMath(ast) return rawHtml(site.latexToMathml(ast.value, false)) end
function defaults.delete(c) return h.strikethrough():sub(c) end
function defaults.emphasis(c) return h.em():sub(c) end
function defaults.mdxTextExpression() return fragment(c.value) end
function defaults.mdxJsEsm() return fragment() end
function defaults.footnoteReference(ast) return fragment() end
function defaults.html(ast) return rawHtml(ast.value) end
function defaults.image(ast) return h.img():attrs({ alt = ast.alt, url = ast.url }) end
function defaults.imageReference(ast) return fragment() end
function defaults.mdxJsxTextElement(ast) return fragment() end
function defaults.link(c, ast) return h.a():attrs({ href = ast.url }):sub(title):sub(table.unpack(c)) end
function defaults.linkReference(ast) return fragment() end
function defaults.strong(c) return h.strong():sub(table.unpack(c)) end
function defaults.text(ast) return fragment(ast.value) end
function defaults.code(ast) return h.code():sub(h.pre():sub(ast.value)) end
function defaults.math(ast) return rawHtml(site.latexToMathml(ast.value, true)) end
function defaults.mdxFlowExpression() return fragment() end
function defaults.heading(c, ast) 
  local headings = { h.h1, h.h2, h.h3, h.h4, h.h5 }
  return (headings[ast.depth] or h.h6)():sub(table.unpack(c)) 
end
function defaults.table(c, ast) return fragment() end
function defaults.thematicBreak() return h.hr() end
function defaults.tableRow(c) return fragment() end
function defaults.tableCell(c) return fragment() end
function defaults.listItem(c, ast) return h.li():sub(table.unpack(c)) end
function defaults.paragraph(c) return h.p():sub(table.unpack(c)) end
function defaults.definition(c) return fragment() end

-- compile many markdown nodes
function compileMany(children, funcs)
  local res = {}
  for i, value in ipairs(children) do
    table.insert(res, mod.compileMd(value, funcs))
  end

  return res
end

-- Parse and compile markdown, with custom functions
function mod.compileMd(ast, funcs)
  -- compile the markdown
  -- if we have children, pass those in as special
  if ast.children then 
    return (funcs[ast.type] 
      or defaults[ast.type] 
      or warn("Missing " .. ast.type)
    )(compileMany(ast.children, funcs), ast)
  else
    return (funcs[ast.type] 
      or defaults[ast.type] 
      or warn("Missing " .. ast.type)
    )(ast)
  end
end

return mod
