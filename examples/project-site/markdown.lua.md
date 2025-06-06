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
```lua
local function hello()
	print "Hello"
end
```

```rust
fn main() {
	println!("Hello world!");
}
```

And math!: $T = \int_0^\infty \sqrt{t^2 + h^2 + 2 \cos t h} \; dt$
Block math too!
$$ T = \int_0^\infty \sqrt{t^2 + h^2 + 2 \cos t h} \; dt $$

<ul>
	<li> sus amogus</li>
	<li> sus amogus</li>
</ul>

More fennel!:
<?fnl (.. "Hello " " from " " fennel!") ?>
