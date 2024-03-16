+++
title = "Math"
+++

# Math in LSSG
lssg includes an extention to commonmark to allow writing math!

Any text between `$` and `$$` is interpreted as inline and block math respectively
By default, this won't render to mathml. Instead, it has to be passed to the function latexToMathml

This converts latex code to mathml, like so!
```lua
latexToMathml("V = \\frac{4}{3} \\pi r^3")
```

Which results in

$$
V = \frac{4}{3} \pi r^3
$$

See the cookbook with the custom markdown renderer to see how to parse markdown manually, in order to do this step
