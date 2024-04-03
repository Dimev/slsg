local mod = {}

-- Add a citation to the list
-- TODO: figure out a nice way to generate citation key indices
function mod.addCitation(list, name)
	list[name] = true
end

-- Generate the bibliography
-- Optionally accepts a list to only pick citations from that list
function mod.generateBibHtml(bib, list)
	-- parse the bib
	local bibtex = bib:parseBibtex()

	-- output html
	local html = h.ol()

	-- generate the bibs
	for key, ref in pairs(bibtex.bibliographies) do
		-- skip if not in list
		if list and not list[key] then  
			-- nothing
		elseif ref.type == 'book' then
			html = html:sub(h.li():sub(
				ref.tags.author, " ",
				h.em(ref.tags.title), " ",
				ref.tags.publisher, " ",
				ref.tags.year
			))
		elseif ref.type == 'article' then
			html = html:sub(h.li():sub(
				ref.tags.author, " ",
				'"' .. ref.tags.title .. '" ',
				h.em(ref.tags.journal), " ",
				ref.tags.volume and 'vol. ' .. ref.tags.volume .. " ",
				ref.tags.number, " ",
				ref.tags.pages and 'pp. ' .. ref.tags.pages .. " ",
				ref.tags.year
			))
		end
	end

	return html
end

return mod
