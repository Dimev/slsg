local mod = {}

-- default handelers
local defaults = {
  root = function(c) return fragment(table.unpack(c)) end,
  blockquote = function(c) return fragment(c) end,
  toml = function() return fragment() end,
  yaml = function() return fragment() end,
  ["break"] = function() return h.hr() end,
  inlineCode = function(c) return h.pre():sub(c) end,
  heading = function(c) return h.h1():sub(table.unpack(c)) end,
  paragraph = function(c) return h.div():sub(table.unpack(c)) end,
  text = function(c) return h.p():sub(c) end,
  code = function(c, lang) return h.pre():sub(rawHtml(site.highlightCodeHtml(c, lang, "code--"))) end,
  math = function(c) return rawHtml(site.latexToMathml(tostring(c))) end,
  inlineMath = function(c) return rawHtml(site.latexToMathml(tostring(c))) end,
}

-- Parse and compile markdown, with custom functions
function mod.compileMd(ast, funcs)
  -- run a function with children
  function runWith(children, ...)
    return (funcs[ast.type] 
      or defaults[ast.type] 
      or warn("Missing " .. ast.type))(compileMany(ast.children, funcs), ...)
  end

  -- compile the markdown
  -- if it has children, look up the type, and run the function
  if ast.children then 
    return runWith(children)
  else
    return (funcs[ast.type] or defaults[ast.type] or warn("Missing " .. ast.type))(ast.value, ast.language)
  end
end

-- compile many markdown nodes
function compileMany(ast, funcs)
  local res = {}
  for i, value in ipairs(ast) do
    table.insert(res, mod.compileMd(value, funcs))
  end

  return res
end

return mod
