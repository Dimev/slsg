; ignore our template files
(ignorefiles :templates/*)

; functions we can use in <? ... ?>
(local mod {})

; page template
(fn mod.page [args]
  (let [t (readfile :templates/page.html) ; template we'll use
        t (t:gsub "@@title" (or args.title "")) ; insert title
        t (t:gsub "@@description" (or args.description ""))] ; description
    (fn [content] ; insert the processed file into the template
      (t:gsub "@@content" content))))

mod

