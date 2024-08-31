// Parse tex and run lua with it
// commands are in the form of \name[1, 2, 3]{arg4, arg5, arg6}
// arguments are passed as tables of strings, if any commands are used, they are also added
// verbatim is done with \verb||, where | can be any character
// any top level text is put into a paragraph, if there are no empty lines between it
// this is done using the special paragraph function, which by default simply concatenates all text
