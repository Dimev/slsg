---@meta
-- lua language server file, to help completions

---@alias DirIter userdata
---@alias FileIter userdata

---@class Site
---@field dev boolean Whether the site is run with `slsg dev`
site = {}

--- Emit a file to the site generator
---@param path string Path to serve the file on
---@param data string Contents of the file
function site.emit(path, data) end

--- Emit a file to the site generator, by copying it from the original
---@param path string Path to serve the file on
---@param original string path to the original file, relative `to main.lua`
function site.emit_file(path, original) end

--- Emit a file to the site generator, by running a command
---@param path string Path to serve the file on
---@param command string command to run
---@param ... string arguments to the command
function site.emit_command(path, command, ...) end

--- Mark a file to be used as 404 page
--- This only has effect in development mode, when run with `slsg dev`
--- @param path string file to mark as 404 page
function site.set_404(path) end

--- Read all directory names at the given path
---@param path string path to read
---@return DirIter Iterator over the directory names
function site.dirs(path) end

--- Reads all file names at the given path
---@param path string path to read
---@return FileIter Iterator over the file names
function site.files(path) end

--- Reads the file at the given path
---@param path string path to read
---@return string file contents
function site.read(path) end

--- Check if a directory exists
---@param path string path to check
---@return boolean whether the directory exists
function site.dir_exists(path) end

--- Check if a file exists
---@param path string path to check
---@return boolean whether the file exists
function site.file_exists(path) end

--- Get the name of a file in the given path
--- The name is the final component of the path
--- This function is the same as rust's `Path::file_name`
---@param path string the path to use
---@return string the file name
function site.file_name(path) end

--- Get the stem of a file in the given path
--- The stem is the file name without the extension
--- This function is the same as rust's `Path::file_stem`
---@param path string the path to use
---@return string the file stem
function site.file_stem(path) end

--- Get the extension of a file in the given path
--- The extension is everything after the final `.` in the file path,
--- if it does not start with that
--- This function is the same as rust's `Path::extension`
---@param path string the path to use
---@return string the file name
function site.file_ext(path) end

--- Compile a tex expression to mathml
--- @param tex string the tex expression
--- @param inline? boolean whether to inline the mathml. Doing so sets `inline` on the `<math>` element to true
--- @return string the mathml string
function site.compile_tex(tex, inline) end
