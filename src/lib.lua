-- Library for writing pages

-- make a node
function el(ty) 
	local element =  {
		tag = ty,
		attributes = "",
		content = "",
	}

	-- add attributes
	function element:attrs(props)
		for key, value in pairs(props) do 
			-- append
			self.attributes = self.attributes .. " " .. key .. "=" .. value
		end
		return self 
	end

	-- add content
	function element:sub(...)
		for i, value in ipairs({ ... }) do 
			self.content = self.content .. value:render() 
		end
		return self
	end

	-- render ourselves
	function element:render()			
		return "<" .. self.tag .. self.attributes .. ">" 
			.. self.content 
			.. "</" .. self.tag .. ">"
	end

	return element
end

-- make a node function
function mkEl(ty) 
	return function() return el(ty) end
end

-- text node
-- TODO: escaping
function txt(text) 
	return {
		render = function() return text end
	}
end

-- raw html
-- TODO: compression/minification
function rawHtml(text) 
	return {
		render = function() return text end
	}
end

-- collection of common nodes
-- text
p = mkEl("p")
h1 = mkEl("h1")
h2 = mkEl("h2")
h3 = mkEl("h3")
h4 = mkEl("h4")
h5 = mkEl("h5")
h6 = mkEl("h6")

-- block
div = mkEl("div")
section = mkEl("section")
article = mkEl("article")
main = mkEl("main")

-- make a page
function page() 
	local pageInfo = {
		type = "page",
		meta = {},
		html = "<!DOCTYPE html>",
		subs = {},
	}

	function pageInfo:withHtml(node) 
		self.html = self.html .. node:render()
		return self
	end

	function pageInfo:withMeta(table)
		-- concat 
		for key, value in pairs(table) do 
			self.meta[key] = value
		end
		return self
	end

	function pageInfo:withSubs(directories) 
		-- concat
		for key, value in pairs(directories) do 
			self.subs[key] = value
		end
		return self
	end

	return pageInfo
end
