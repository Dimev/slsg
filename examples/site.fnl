
(fn site.macro.replace-text [x] (print :sus!))

(each [key val (pairs site.macro)] (print key))

;; set the not found page
(set site.not_found :index.html)

(site.rewrite :main (fn [] (print :mogus)))
