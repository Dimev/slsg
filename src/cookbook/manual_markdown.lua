-- Manually convert markdown to HTML, according to commonmark

local function tomany(many)
  local html = {}
  for _, node in pairs(many) do
    -- TODO: if nill, do not include
    table.insert(html, compileMarkdown(node))
  end

  -- we want a fragment here
  return fragment(table.unpack(html))
end

function compileMarkdown(ast, defaults)
  local nodetypes = {defaults} -- TODO: find way to add custom node types here
  -- TODO: big table of what nodes to use
  -- implement default case when only children are needed/only value is needed
  -- add special cases for the rest

  -- TODO: also keep track of footnotes

  
  if ast.type == "root" then
    return tomany(ast.children)
  elseif ast.type == "blockquote" then
    return div():sub(tomany(ast.children))
  elseif ast.type == "footnotedefinition" then
    -- TODO
    warn("Not yet implemented")
  elseif ast.type == "mdxjsxflowelement" then
    -- TODO
    warn("Not yet implemented")
  elseif ast.type == "list" then
    return el('li'):sub(tomany(ast.children))
  elseif ast.type == "toml" or ast.type == "yaml" then 
    return el('comment') -- TODO: some way to skip these
  elseif ast.type == "break" then
    return el('break')
  elseif ast.type == "inlinecode" then
    return el('pre'):sub(txt(ast.value))
  elseif ast.type == "inlinemath" then
    return rawHtml(latextomathml(ast.value))
  elseif ast.type == "delete" then
    return el('strikethrough'):sub(tomany(ast.children))
  elseif ast.type == "mdxtextexpression" then
    -- TODO
    warn("Not yet implemented")
  else
    warn("Node type " .. ast.type .. " not found!")
  end
end

