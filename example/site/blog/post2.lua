-- svg illustration
local ascii = [[
       .---.
      /-o-/--
   .-/ / /->
  ( *  \/
   '-.  \
      \ /
       '

+---------------+
| Hello svgbob! |
+---------------+

 .~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~.
!                                                            !
! We can draw cool boxes, whichohopefullyofitotheofontosize! !
!                                                            !
 '~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~'
 
    H  H  H
    |  |  |
H --*--*--*--H
    |  |  |
    H  H  H
]]

local svg = yassg.svgbob(ascii, { fontfamily = "monospace" })

local html = div()
  :sub(
    rawHtml(svg)
  )
  :render()

return page()
  :withHtml(html)
