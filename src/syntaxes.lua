-- Register all syntaxes
-- Converted from https://github.com/zyedidia/micro/tree/master/runtime/syntax
-- MIT License
--
-- Copyright (c) 2016-2020: Zachary Yedidia, et al.
--
-- Permission is hereby granted, free of charge, to any person obtaining
-- a copy of this software and associated documentation files (the
-- "Software"), to deal in the Software without restriction, including
-- without limitation the rights to use, copy, modify, merge, publish,
-- distribute, sublicense, and/or sell copies of the Software, and to
-- permit persons to whom the Software is furnished to do so, subject to
-- the following conditions:
--
-- The above copyright notice and this permission notice shall be
-- included in all copies or substantial portions of the Software.
--
-- THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
-- EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
-- MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
-- IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
-- CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
-- TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
-- SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

registersyntax {
  name = "lua",
  regex = "lua$",
  { token = "statement",       "\\b(do|end|while|break|repeat|until|if|elseif|then|else|for|in|function|local|return|goto)\\b" },
  { token = "statement",       "\\b(not|and|or)\\b" },
  { token = "statement",       "\\b(debug|string|math|table|io|coroutine|os|utf8|bit32)\\b\\." },
  { token = "statement",       "\\b(_ENV|_G|_VERSION|assert|collectgarbage|dofile|error|getfenv|getmetatable|ipairs|load|loadfile|module|next|pairs|pcall|print|rawequal|rawget|rawlen|rawset|require|select|setfenv|setmetatable|tonumber|tostring|type|unpack|xpcall)\\s*\\(" },
  { token = "identifier",      "io\\.\\b(close|flush|input|lines|open|output|popen|read|tmpfile|type|write)\\b" },
  { token = "identifier",      "math\\.\\b(abs|acos|asin|atan2|atan|ceil|cosh|cos|deg|exp|floor|fmod|frexp|huge|ldexp|log10|log|max|maxinteger|min|mininteger|modf|pi|pow|rad|random|randomseed|sin|sqrt|tan|tointeger|type|ult)\\b" },
  { token = "identifier",      "os\\.\\b(clock|date|difftime|execute|exit|getenv|remove|rename|setlocale|time|tmpname)\\b" },
  { token = "identifier",      "package\\.\\b(config|cpath|loaded|loadlib|path|preload|seeall|searchers|searchpath)\\b" },
  { token = "identifier",      "string\\.\\b(byte|char|dump|find|format|gmatch|gsub|len|lower|match|pack|packsize|rep|reverse|sub|unpack|upper)\\b" },
  { token = "identifier",      "table\\.\\b(concat|insert|maxn|move|pack|remove|sort|unpack)\\b" },
  { token = "identifier",      "utf8\\.\\b(char|charpattern|codes|codepoint|len|offset)\\b" },
  { token = "identifier",      "coroutine\\.\\b(create|isyieldable|resume|running|status|wrap|yield)\\b" },
  { token = "identifier",      "debug\\.\\b(debug|getfenv|gethook|getinfo|getlocal|getmetatable|getregistry|getupvalue|getuservalue|setfenv|sethook|setlocal|setmetatable|setupvalue|setuservalue|traceback|upvalueid|upvaluejoin)\\b" },
  { token = "identifier",      "bit32\\.\\b(arshift|band|bnot|bor|btest|bxor|extract|replace|lrotate|lshift|rrotate|rshift)\\b" },
  { token = "identifier",      "\\:\\b(close|flush|lines|read|seek|setvbuf|write|byte|char|dump|find|format|gmatch|gsub|len|lower|match|pack|packsize|rep|reverse|sub|unpack|upper)\\b" },
  { token = "identifier",      "\\b(self|arg)\\b" },
  { token = "constant",        "\\b(false|nil|true)\\b" },
  { token = "statement",       "(\\b(dofile|require|include)|%q|%!|%Q|%r|%x)\\b" },
  { token = "comment", "--.*" },

  { token = "symbol-brackets", "[(){}\\[\\]]" },
  { token = "symbol",          "(\\*|//|/|%|\\+|-|\\^|>|>=|<|<=|~=|=|[\\.]{2,3}|#)" },

  { token = "constant-number", "\\b((0[xX](([0-9A-Fa-f]+\\.[0-9A-Fa-f]*)|(\\.?[0-9A-Fa-f]+))([pP][-+]?[0-9]+)?)|((([0-9]+\\.[0-9]*)|(\\.?[0-9]+))([eE][-+]?[0-9]+)?))" },

  { token = "constant-string", open = "\"", close = "\"", skip = "\\\\.",
    { token = "constant-specialChar", "\\\\([abfnrtvz\\'\"]|[0-9]{1,3}|x[0-9a-fA-F][0-9a-fA-F]|u\\{[0-9a-fA-F]+\\})" }
  },

  { token = "constant-string", open = "\'", close = "\'", skip = "\\\\.",
    { token = "constant-specialChar", "\\\\([abfnrtvz\\'\"]|[0-9]{1,3}|x[0-9a-fA-F][0-9a-fA-F]|u\\{[0-9a-fA-F]+\\})" }
  },

  
}

registersyntax {
  name = "html",
  regex = "htm(l)?$"
}

registersyntax {
  name = "markdown",
  regex = "md$"
}

registersyntax {
  name = "shell",
  regex = "sh$",
  { token = "statement",  "--*\\w+" },
  { token = "identifier", "^\\w+" },
}

registersyntax {
  name = "rust",
  regex = "rs$",
  { token = "identifier",      "fn [a-z0-9_]+" },
  { token = "statement",       "\\b(abstract|alignof|as|async|await|become|box|break|const|continue|crate|do|dyn|else|enum|extern|false|final|fn|for|gen|if|impl|in|let|loop|macro|match|mod|move|mut|offsetof|override|priv|pub|pure|ref|return|sizeof|static|self|struct|super|true|trait|type|typeof|try|union|unsafe|unsized|use|virtual|where|while|yield)\\b" },
  { token = "special",         "[a-z_]+!" },
  { token = "constant",        "\\b[A-Z][A-Z_0-9]+\\b" },
  { token = "constant-number", "\\b[0-9]+\\b" },
  { token = "constant",        "\\b(true|false)\\b" },
  { token = "type",            "\\b[A-Z]+[a-zA-Z_0-9]*[a-z]+[a-zA-Z_0-9]*\\b" },
  { token = "type",            "\\b(bool|str|char|((i|u)(8|16|32|64|128|size))|f(16|32|64|128))\\b" },

  { token = "constant-string", open = "[bc]?\"", close = "\"", skip = "\\.",
    { token = "constant-special-char", "\\." }
  },
}
