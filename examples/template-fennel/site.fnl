;; ignore our template files
(ignorefiles :templates/*)

;; functions we can use in <? ... ?>
(local mod {})

;; files in the index
(local index {})

;; page template
(fn mod.page [args]
  (tset index curtargetdir {:title args.title :desc args.description}) ; add to the index
  (let [t (readfile :templates/page.html) ; template we'll use
        t (t:gsub "@@title" (or args.title "")) ; insert title
        t (t:gsub "@@description" (or args.description "")) ; description
        t (t:gsub "@@date" (or args.date ""))] ; date
    ;; insert the processed file into the template
    #(t:gsub "@@content" $)))

;; generate index
(fn genidx [idx]
  (table.sort idx #(< $1.date $2.date)) ; sort by date
  (var html :<ul>) ; make a list
  (each [k v (pairs idx)] ; add all our posts to the list
    (set html (.. html "<li><a href=\"" k "\">" v.title :</a> :</li>)))
  (.. :</ul> html))

;; Index page
(fn mod.index [args]
  (let [t (readfile :templates/index.html)] ; template we'll use
    ;; insert the processed file into the template
    #(let [t (t:gsub "@@title" args.title)
           t (t:gsub "@@description" args.description)
           t (t:gsub "@@content" $)
           t (t:gsub "@@index" (genidx index))]
       t)))

mod

