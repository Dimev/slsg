-- Builtin functions
-- functionality for making pages and html

-- globally accesssible warnings
debugWarnings = {}

-- add a warning
function warn(text)
	table.insert(debugWarnings, text)
end

-- make a page
function page() 
	local table = {
		-- no html to start off with
		html = nil,
		files = {},
		pages = {},
	}

 	-- set the html
	function table:withHtml(html) 
		-- check if it's a string
		if type(html) ~= "string" then 
			warn("The provided html is not a string, did you forget to call `:render()` ?")
		else		
			self.html = html
		end
		
		return self
	end

	-- add a file
	function table:withFile(path, file)
		self.files[path] = file
				
		return self
	end

	-- add a page
	function table:withPage(path, page) 
		self.pages[path] = page
		
		return self
	end

	-- add many files
	function table:withManyFiles(files) 
		for key, value in pairs(files) do 
			self:withFile(key, value)
		end
		
		return self
	end

	-- add many pages
	function table:withManyPages(pages) 
		for key, value in pairs(pages) do 
			self:withPage(key, value)
		end

		return self
	end
	
	return table
end

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

	-- render ourselves for internal use
	function element:render()
		return "<!DOCTYPE html>" .. self:renderself()
	end

	-- render ourselves
	function element:renderself()			
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

-- anchor
a = function(href)
	return el("a"):attrs({ href = href })
end
