<?lua
	local templ = require 'scripts/templates'
	return templ.page { title = 'Markdown' }
?> 

# This is a paragraph!
Hello world
We have text!

Next paragraph!
We have <emph>Inline html</emph>
And <?lua "Lua!" ?> and <?fnl "Fennel" ?>

We also have highlights!
`Mono`, **Bold**, *italic*, and ***combined***

And syntax highlights!
```lua pre-
local function hello()
	print "Hello"
end
```

And math!: $1 + 1$
Block math too!
$$ 1 + 1 $$

<ul>
	<li> sus amogus</li>
	<li> sus amogus</li>
</ul>
