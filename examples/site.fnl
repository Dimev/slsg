
(fn site.rewrite.replace-text [x] (print :sus!))

(each [key val (pairs site.rewrite)] (print key))

;; set the not found page
(set site.not_found :index.html)


