+++
title = "Math"
+++

# Math in LSSG
lssg includes an extention to commonmark to allow writing math!

Any text between `$` and `$$` is interpreted as math
By default, this won't render to mathml. Instead, it has to be passed to the function latexToMathml

This converts latex code to mathml, like so!
```lua
site.latexToMathml("V = \\frac{4}{3} \\pi r^3")
```

Which results in

$$V = \frac{4}{3} \pi r^3$$

And
```lua
site.latexToMathml("\\int_0^1 x dx")
```

results in

$$\int_0^1 x dx$$

Here's a few more formulas:

$$x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}$$

$$\int_0^1 x^x dx = \sum_{n=1}^{\infty} {(-1)}^{n+1}n^{-1}$$

See the cookbook with the custom markdown renderer to see how to parse markdown manually, in order to do this step
