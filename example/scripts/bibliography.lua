local mod = {}

-- Add a citation to the list
-- this should be run during setup when using the markdown renderer from the cookbook
function mod.addCitation(name, list, bib)
	-- add the citation to the list
	table.insert(list, name)

	-- sort the list, based on the author name
	table.sort(list, function(l, r)
		return bib[l].tags.author < bib[r].tags.author
	end)
end

-- Render a citation from the list
function mod.renderCitation(name, list)
	-- look up the index of the author
	local index = 0
	for key, value in ipairs(list) do
		-- stop when found
		if value == name then index = key break end
	end

	-- draw the citation
	return h.p('[' .. index ..']')
end

-- Generate the bibliography
-- Optionally accepts a list to only pick citations from that list
function mod.generateBibHtml(bib, list)
	-- output html
	local html = h.ol()

	-- generate the bibs
	for key, ref in pairs(bib.bibliographies) do
		-- skip if not in list, and the list was given
		if list and not list[key] then
			-- nothing
		elseif ref.type == 'book' then
			html = html:sub(h.li():sub(
				ref.tags.author,
				h.em(ref.tags.title),
				ref.tags.publisher,
				ref.tags.year
			))
		elseif ref.type == 'article' then
			html = html:sub(h.li():sub(
				ref.tags.author,
				'"' .. ref.tags.title .. '"',
				h.em(ref.tags.journal),
				ref.tags.volume and 'vol. ' .. ref.tags.volume,
				ref.tags.number, " ",
				ref.tags.pages and 'pp. ' .. ref.tags.pages,
				ref.tags.year
			))
		end
	end

	return html
end

return mod
