<?lua
	local templ = require 'scripts/templates'
	return templ.page { title = 'Minimark' }
?> 

% This is a comment!
= This is a paragraph!
Hello world
We have text!

Next paragraph!
We have <? <emph>Inline html!</emph> ?>
And <?lua "Lua!" ?> and <?fnl "Fennel" ?>

We also have highlights!
`Mono`, *Bold*, _italic_, and *_combined_*

And syntax highlights!
```lua pre-
local function hello()
	print "Hello"
end
```
